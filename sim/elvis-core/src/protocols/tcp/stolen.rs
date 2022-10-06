use std::{
    cmp,
    fmt::{self, Display},
    ops,
    ptr::copy_nonoverlapping,
};

/// Error returned by [`Socket::listen`]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ListenError {
    InvalidState,
    Unaddressable,
}

/// Error returned by [`Socket::connect`]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectError {
    InvalidState,
    Unaddressable,
}

/// Error returned by [`Socket::send`]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SendError {
    InvalidState,
}

/// Error returned by [`Socket::recv`]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RecvError {
    InvalidState,
    Finished,
}

/// The state of a TCP socket, according to [RFC 793].
///
/// [RFC 793]: https://tools.ietf.org/html/rfc793
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            State::Closed => write!(f, "CLOSED"),
            State::Listen => write!(f, "LISTEN"),
            State::SynSent => write!(f, "SYN-SENT"),
            State::SynReceived => write!(f, "SYN-RECEIVED"),
            State::Established => write!(f, "ESTABLISHED"),
            State::FinWait1 => write!(f, "FIN-WAIT-1"),
            State::FinWait2 => write!(f, "FIN-WAIT-2"),
            State::CloseWait => write!(f, "CLOSE-WAIT"),
            State::Closing => write!(f, "CLOSING"),
            State::LastAck => write!(f, "LAST-ACK"),
            State::TimeWait => write!(f, "TIME-WAIT"),
        }
    }
}

// Conservative initial RTT estimate.
const RTTE_INITIAL_RTT: u32 = 300;
const RTTE_INITIAL_DEV: u32 = 100;

// Minimum "safety margin" for the RTO that kicks in when the
// variance gets very low.
const RTTE_MIN_MARGIN: u32 = 5;

const RTTE_MIN_RTO: u32 = 10;
const RTTE_MAX_RTO: u32 = 10000;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct RttEstimator {
    // Using u32 instead of Duration to save space (Duration is i64)
    rtt: u32,
    deviation: u32,
    timestamp: Option<(Instant, TcpSeqNumber)>,
    max_seq_sent: Option<TcpSeqNumber>,
    rto_count: u8,
}

impl Default for RttEstimator {
    fn default() -> Self {
        Self {
            rtt: RTTE_INITIAL_RTT,
            deviation: RTTE_INITIAL_DEV,
            timestamp: None,
            max_seq_sent: None,
            rto_count: 0,
        }
    }
}

impl RttEstimator {
    fn retransmission_timeout(&self) -> Duration {
        let margin = RTTE_MIN_MARGIN.max(self.deviation * 4);
        let ms = (self.rtt + margin).max(RTTE_MIN_RTO).min(RTTE_MAX_RTO);
        Duration::from_millis(ms as u64)
    }

    fn sample(&mut self, new_rtt: u32) {
        // "Congestion Avoidance and Control", Van Jacobson, Michael J. Karels, 1988
        self.rtt = (self.rtt * 7 + new_rtt + 7) / 8;
        let diff = (self.rtt as i32 - new_rtt as i32).unsigned_abs();
        self.deviation = (self.deviation * 3 + diff + 3) / 4;

        self.rto_count = 0;

        let rto = self.retransmission_timeout().total_millis();
    }

    fn on_send(&mut self, timestamp: Instant, seq: TcpSeqNumber) {
        if self
            .max_seq_sent
            .map(|max_seq_sent| seq > max_seq_sent)
            .unwrap_or(true)
        {
            self.max_seq_sent = Some(seq);
            if self.timestamp.is_none() {
                self.timestamp = Some((timestamp, seq));
            }
        }
    }

    fn on_ack(&mut self, timestamp: Instant, seq: TcpSeqNumber) {
        if let Some((sent_timestamp, sent_seq)) = self.timestamp {
            if seq >= sent_seq {
                self.sample((timestamp - sent_timestamp).total_millis() as u32);
                self.timestamp = None;
            }
        }
    }

    fn on_retransmit(&mut self) {
        if self.timestamp.is_some() {}
        self.timestamp = None;
        self.rto_count = self.rto_count.saturating_add(1);
        if self.rto_count >= 3 {
            // This happens in 2 scenarios:
            // - The RTT is higher than the initial estimate
            // - The network conditions change, suddenly making the RTT much higher
            // In these cases, the estimator can get stuck, because it can't sample because
            // all packets sent would incur a retransmit. To avoid this, force an estimate
            // increase if we see 3 consecutive retransmissions without any successful sample.
            self.rto_count = 0;
            self.rtt = RTTE_MAX_RTO.min(self.rtt * 2);
            let rto = self.retransmission_timeout().total_millis();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum Timer {
    Idle {
        keep_alive_at: Option<Instant>,
    },
    Retransmit {
        expires_at: Instant,
        delay: Duration,
    },
    FastRetransmit,
    Close {
        expires_at: Instant,
    },
}

const ACK_DELAY_DEFAULT: Duration = Duration::from_millis(10);
const CLOSE_DELAY: Duration = Duration::from_millis(10_000);

impl Timer {
    fn new() -> Timer {
        Timer::Idle {
            keep_alive_at: None,
        }
    }

    fn should_keep_alive(&self, timestamp: Instant) -> bool {
        match *self {
            Timer::Idle {
                keep_alive_at: Some(keep_alive_at),
            } if timestamp >= keep_alive_at => true,
            _ => false,
        }
    }

    fn should_retransmit(&self, timestamp: Instant) -> Option<Duration> {
        match *self {
            Timer::Retransmit { expires_at, delay } if timestamp >= expires_at => {
                Some(timestamp - expires_at + delay)
            }
            Timer::FastRetransmit => Some(Duration::from_millis(0)),
            _ => None,
        }
    }

    fn should_close(&self, timestamp: Instant) -> bool {
        match *self {
            Timer::Close { expires_at } if timestamp >= expires_at => true,
            _ => false,
        }
    }

    fn poll_at(&self) -> PollAt {
        match *self {
            Timer::Idle {
                keep_alive_at: Some(keep_alive_at),
            } => PollAt::Time(keep_alive_at),
            Timer::Idle {
                keep_alive_at: None,
            } => PollAt::Ingress,
            Timer::Retransmit { expires_at, .. } => PollAt::Time(expires_at),
            Timer::FastRetransmit => PollAt::Now,
            Timer::Close { expires_at } => PollAt::Time(expires_at),
        }
    }

    fn set_for_idle(&mut self, timestamp: Instant, interval: Option<Duration>) {
        *self = Timer::Idle {
            keep_alive_at: interval.map(|interval| timestamp + interval),
        }
    }

    fn set_keep_alive(&mut self) {
        if let Timer::Idle {
            ref mut keep_alive_at,
        } = *self
        {
            if keep_alive_at.is_none() {
                *keep_alive_at = Some(Instant::from_millis(0))
            }
        }
    }

    fn rewind_keep_alive(&mut self, timestamp: Instant, interval: Option<Duration>) {
        if let Timer::Idle {
            ref mut keep_alive_at,
        } = *self
        {
            *keep_alive_at = interval.map(|interval| timestamp + interval)
        }
    }

    fn set_for_retransmit(&mut self, timestamp: Instant, delay: Duration) {
        match *self {
            Timer::Idle { .. } | Timer::FastRetransmit { .. } => {
                *self = Timer::Retransmit {
                    expires_at: timestamp + delay,
                    delay,
                }
            }
            Timer::Retransmit { expires_at, delay } if timestamp >= expires_at => {
                *self = Timer::Retransmit {
                    expires_at: timestamp + delay,
                    delay: delay * 2,
                }
            }
            Timer::Retransmit { .. } => (),
            Timer::Close { .. } => (),
        }
    }

    fn set_for_fast_retransmit(&mut self) {
        *self = Timer::FastRetransmit
    }

    fn set_for_close(&mut self, timestamp: Instant) {
        *self = Timer::Close {
            expires_at: timestamp + CLOSE_DELAY,
        }
    }

    fn is_retransmit(&self) -> bool {
        match *self {
            Timer::Retransmit { .. } | Timer::FastRetransmit => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AckDelayTimer {
    Idle,
    Waiting(Instant),
    Immediate,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Tuple {
    local: IpEndpoint,
    remote: IpEndpoint,
}

impl Display for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.local, self.remote)
    }
}

/// A representation of an absolute time value.
///
/// The `Instant` type is a wrapper around a `i64` value that
/// represents a number of milliseconds, monotonically increasing
/// since an arbitrary moment in time, such as system startup.
///
/// * A value of `0` is inherently arbitrary.
/// * A value less than `0` indicates a time before the starting
///   point.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Instant {
    micros: i64,
}

impl Instant {
    pub const ZERO: Instant = Instant::from_micros_const(0);

    /// Create a new `Instant` from a number of microseconds.
    pub fn from_micros<T: Into<i64>>(micros: T) -> Instant {
        Instant {
            micros: micros.into(),
        }
    }

    pub const fn from_micros_const(micros: i64) -> Instant {
        Instant { micros }
    }

    /// Create a new `Instant` from a number of milliseconds.
    pub fn from_millis<T: Into<i64>>(millis: T) -> Instant {
        Instant {
            micros: millis.into() * 1000,
        }
    }

    /// Create a new `Instant` from a number of milliseconds.
    pub const fn from_millis_const(millis: i64) -> Instant {
        Instant {
            micros: millis * 1000,
        }
    }

    /// Create a new `Instant` from a number of seconds.
    pub fn from_secs<T: Into<i64>>(secs: T) -> Instant {
        Instant {
            micros: secs.into() * 1000000,
        }
    }

    /// Create a new `Instant` from the current [std::time::SystemTime].
    ///
    /// See [std::time::SystemTime::now]
    ///
    /// [std::time::SystemTime]: https://doc.rust-lang.org/std/time/struct.SystemTime.html
    /// [std::time::SystemTime::now]: https://doc.rust-lang.org/std/time/struct.SystemTime.html#method.now
    #[cfg(feature = "std")]
    pub fn now() -> Instant {
        Self::from(::std::time::SystemTime::now())
    }

    /// The fractional number of milliseconds that have passed
    /// since the beginning of time.
    pub const fn millis(&self) -> i64 {
        self.micros % 1000000 / 1000
    }

    /// The fractional number of microseconds that have passed
    /// since the beginning of time.
    pub const fn micros(&self) -> i64 {
        self.micros % 1000000
    }

    /// The number of whole seconds that have passed since the
    /// beginning of time.
    pub const fn secs(&self) -> i64 {
        self.micros / 1000000
    }

    /// The total number of milliseconds that have passed since
    /// the beginning of time.
    pub const fn total_millis(&self) -> i64 {
        self.micros / 1000
    }
    /// The total number of milliseconds that have passed since
    /// the beginning of time.
    pub const fn total_micros(&self) -> i64 {
        self.micros
    }
}

#[cfg(feature = "std")]
impl From<::std::time::Instant> for Instant {
    fn from(other: ::std::time::Instant) -> Instant {
        let elapsed = other.elapsed();
        Instant::from_micros((elapsed.as_secs() * 1_000000) as i64 + elapsed.subsec_micros() as i64)
    }
}

#[cfg(feature = "std")]
impl From<::std::time::SystemTime> for Instant {
    fn from(other: ::std::time::SystemTime) -> Instant {
        let n = other
            .duration_since(::std::time::UNIX_EPOCH)
            .expect("start time must not be before the unix epoch");
        Self::from_micros(n.as_secs() as i64 * 1000000 + n.subsec_micros() as i64)
    }
}

#[cfg(feature = "std")]
impl From<Instant> for ::std::time::SystemTime {
    fn from(val: Instant) -> Self {
        ::std::time::UNIX_EPOCH + ::std::time::Duration::from_micros(val.micros as u64)
    }
}

impl fmt::Display for Instant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}s", self.secs(), self.millis())
    }
}

impl ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Instant {
        Instant::from_micros(self.micros + rhs.total_micros() as i64)
    }
}

impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.micros += rhs.total_micros() as i64;
    }
}

impl ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Instant {
        Instant::from_micros(self.micros - rhs.total_micros() as i64)
    }
}

impl ops::SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.micros -= rhs.total_micros() as i64;
    }
}

impl ops::Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Duration {
        Duration::from_micros((self.micros - rhs.micros).unsigned_abs())
    }
}

/// A relative amount of time.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Duration {
    micros: u64,
}

impl Duration {
    pub const ZERO: Duration = Duration::from_micros(0);
    /// Create a new `Duration` from a number of microseconds.
    pub const fn from_micros(micros: u64) -> Duration {
        Duration { micros }
    }

    /// Create a new `Duration` from a number of milliseconds.
    pub const fn from_millis(millis: u64) -> Duration {
        Duration {
            micros: millis * 1000,
        }
    }

    /// Create a new `Instant` from a number of seconds.
    pub const fn from_secs(secs: u64) -> Duration {
        Duration {
            micros: secs * 1000000,
        }
    }

    /// The fractional number of milliseconds in this `Duration`.
    pub const fn millis(&self) -> u64 {
        self.micros / 1000 % 1000
    }

    /// The fractional number of milliseconds in this `Duration`.
    pub const fn micros(&self) -> u64 {
        self.micros % 1000000
    }

    /// The number of whole seconds in this `Duration`.
    pub const fn secs(&self) -> u64 {
        self.micros / 1000000
    }

    /// The total number of milliseconds in this `Duration`.
    pub const fn total_millis(&self) -> u64 {
        self.micros / 1000
    }

    /// The total number of microseconds in this `Duration`.
    pub const fn total_micros(&self) -> u64 {
        self.micros
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{:03}s", self.secs(), self.millis())
    }
}

impl ops::Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Duration {
        Duration::from_micros(self.micros + rhs.total_micros())
    }
}

impl ops::AddAssign<Duration> for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        self.micros += rhs.total_micros();
    }
}

impl ops::Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Duration {
        Duration::from_micros(
            self.micros
                .checked_sub(rhs.total_micros())
                .expect("overflow when subtracting durations"),
        )
    }
}

impl ops::SubAssign<Duration> for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        self.micros = self
            .micros
            .checked_sub(rhs.total_micros())
            .expect("overflow when subtracting durations");
    }
}

impl ops::Mul<u32> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u32) -> Duration {
        Duration::from_micros(self.micros * rhs as u64)
    }
}

impl ops::MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        self.micros *= rhs as u64;
    }
}

impl ops::Div<u32> for Duration {
    type Output = Duration;

    fn div(self, rhs: u32) -> Duration {
        Duration::from_micros(self.micros / rhs as u64)
    }
}

impl ops::DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        self.micros /= rhs as u64;
    }
}

impl ops::Shl<u32> for Duration {
    type Output = Duration;

    fn shl(self, rhs: u32) -> Duration {
        Duration::from_micros(self.micros << rhs)
    }
}

impl ops::ShlAssign<u32> for Duration {
    fn shl_assign(&mut self, rhs: u32) {
        self.micros <<= rhs;
    }
}

impl ops::Shr<u32> for Duration {
    type Output = Duration;

    fn shr(self, rhs: u32) -> Duration {
        Duration::from_micros(self.micros >> rhs)
    }
}

impl ops::ShrAssign<u32> for Duration {
    fn shr_assign(&mut self, rhs: u32) {
        self.micros >>= rhs;
    }
}

impl From<::core::time::Duration> for Duration {
    fn from(other: ::core::time::Duration) -> Duration {
        Duration::from_micros(other.as_secs() * 1000000 + other.subsec_micros() as u64)
    }
}

impl From<Duration> for ::core::time::Duration {
    fn from(val: Duration) -> Self {
        ::core::time::Duration::from_micros(val.total_micros())
    }
}

/// A TCP sequence number.
///
/// A sequence number is a monotonically advancing integer modulo 2<sup>32</sup>.
/// Sequence numbers do not have a discontiguity when compared pairwise across a signed overflow.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TcpSeqNumber(pub i32);

impl fmt::Display for TcpSeqNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0 as u32)
    }
}

impl ops::Add<usize> for TcpSeqNumber {
    type Output = TcpSeqNumber;

    fn add(self, rhs: usize) -> TcpSeqNumber {
        if rhs > i32::MAX as usize {
            panic!("attempt to add to sequence number with unsigned overflow")
        }
        TcpSeqNumber(self.0.wrapping_add(rhs as i32))
    }
}

impl ops::Sub<usize> for TcpSeqNumber {
    type Output = TcpSeqNumber;

    fn sub(self, rhs: usize) -> TcpSeqNumber {
        if rhs > i32::MAX as usize {
            panic!("attempt to subtract to sequence number with unsigned overflow")
        }
        TcpSeqNumber(self.0.wrapping_sub(rhs as i32))
    }
}

impl ops::AddAssign<usize> for TcpSeqNumber {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl ops::Sub for TcpSeqNumber {
    type Output = usize;

    fn sub(self, rhs: TcpSeqNumber) -> usize {
        let result = self.0.wrapping_sub(rhs.0);
        if result < 0 {
            panic!("attempt to subtract sequence numbers with underflow")
        }
        result as usize
    }
}

impl cmp::PartialOrd for TcpSeqNumber {
    fn partial_cmp(&self, other: &TcpSeqNumber) -> Option<cmp::Ordering> {
        self.0.wrapping_sub(other.0).partial_cmp(&0)
    }
}

/// Gives an indication on the next time the socket should be polled.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum PollAt {
    /// The socket needs to be polled immediately.
    Now,
    /// The socket needs to be polled at given [Instant][struct.Instant].
    Time(Instant),
    /// The socket does not need to be polled unless there are external changes.
    Ingress,
}

/// An internet endpoint address.
///
/// `Endpoint` always fully specifies both the address and the port.
///
/// See also ['ListenEndpoint'], which allows not specifying the address
/// in order to listen on a given port on any address.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct IpEndpoint {
    pub addr: Address,
    pub port: u16,
}

impl IpEndpoint {
    /// Create an endpoint address from given address and port.
    pub fn new(addr: Address, port: u16) -> IpEndpoint {
        IpEndpoint { addr: addr, port }
    }
}

impl fmt::Display for IpEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

impl<T: Into<Address>> From<(T, u16)> for IpEndpoint {
    fn from((addr, port): (T, u16)) -> IpEndpoint {
        IpEndpoint {
            addr: addr.into(),
            port,
        }
    }
}

/// An internetworking address.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Address {
    Ipv4(Ipv4Address),
    Ipv6(Ipv6Address),
}

impl Address {
    /// Create an address wrapping an IPv4 address with the given octets.
    pub fn v4(a0: u8, a1: u8, a2: u8, a3: u8) -> Address {
        Address::Ipv4(Ipv4Address::new(a0, a1, a2, a3))
    }

    /// Create an address wrapping an IPv6 address with the given octets.
    #[allow(clippy::too_many_arguments)]
    pub fn v6(a0: u16, a1: u16, a2: u16, a3: u16, a4: u16, a5: u16, a6: u16, a7: u16) -> Address {
        Address::Ipv6(Ipv6Address::new(a0, a1, a2, a3, a4, a5, a6, a7))
    }

    /// Return the protocol version.
    pub fn version(&self) -> Version {
        match self {
            Address::Ipv4(_) => Version::Ipv4,
            Address::Ipv6(_) => Version::Ipv6,
        }
    }

    /// Return an address as a sequence of octets, in big-endian.
    pub fn as_bytes(&self) -> &[u8] {
        match *self {
            Address::Ipv4(ref addr) => addr.as_bytes(),
            Address::Ipv6(ref addr) => addr.as_bytes(),
        }
    }

    /// Query whether the address is a valid unicast address.
    pub fn is_unicast(&self) -> bool {
        match *self {
            Address::Ipv4(addr) => addr.is_unicast(),
            Address::Ipv6(addr) => addr.is_unicast(),
        }
    }

    /// Query whether the address is a valid multicast address.
    pub fn is_multicast(&self) -> bool {
        match *self {
            Address::Ipv4(addr) => addr.is_multicast(),
            Address::Ipv6(addr) => addr.is_multicast(),
        }
    }

    /// Query whether the address is the broadcast address.
    pub fn is_broadcast(&self) -> bool {
        match *self {
            Address::Ipv4(addr) => addr.is_broadcast(),
            Address::Ipv6(_) => false,
        }
    }

    /// Query whether the address falls into the "unspecified" range.
    pub fn is_unspecified(&self) -> bool {
        match *self {
            Address::Ipv4(addr) => addr.is_unspecified(),
            Address::Ipv6(addr) => addr.is_unspecified(),
        }
    }

    /// If `self` is a CIDR-compatible subnet mask, return `Some(prefix_len)`,
    /// where `prefix_len` is the number of leading zeroes. Return `None` otherwise.
    pub fn prefix_len(&self) -> Option<u8> {
        let mut ones = true;
        let mut prefix_len = 0;
        for byte in self.as_bytes() {
            let mut mask = 0x80;
            for _ in 0..8 {
                let one = *byte & mask != 0;
                if ones {
                    // Expect 1s until first 0
                    if one {
                        prefix_len += 1;
                    } else {
                        ones = false;
                    }
                } else if one {
                    // 1 where 0 was expected
                    return None;
                }
                mask >>= 1;
            }
        }
        Some(prefix_len)
    }
}

impl From<Ipv4Address> for Address {
    fn from(addr: Ipv4Address) -> Self {
        Address::Ipv4(addr)
    }
}

impl From<Ipv6Address> for Address {
    fn from(addr: Ipv6Address) -> Self {
        Address::Ipv6(addr)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Address::Ipv4(addr) => write!(f, "{}", addr),
            Address::Ipv6(addr) => write!(f, "{}", addr),
        }
    }
}

/// A four-octet IPv4 address.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct Ipv4Address(pub [u8; IPV4_ADDR_SIZE]);

impl Ipv4Address {
    /// An unspecified address.
    pub const UNSPECIFIED: Ipv4Address = Ipv4Address([0x00; IPV4_ADDR_SIZE]);

    /// The broadcast address.
    pub const BROADCAST: Ipv4Address = Ipv4Address([0xff; IPV4_ADDR_SIZE]);

    /// All multicast-capable nodes
    pub const MULTICAST_ALL_SYSTEMS: Ipv4Address = Ipv4Address([224, 0, 0, 1]);

    /// All multicast-capable routers
    pub const MULTICAST_ALL_ROUTERS: Ipv4Address = Ipv4Address([224, 0, 0, 2]);

    /// Construct an IPv4 address from parts.
    pub const fn new(a0: u8, a1: u8, a2: u8, a3: u8) -> Ipv4Address {
        Ipv4Address([a0, a1, a2, a3])
    }

    /// Construct an IPv4 address from a sequence of octets, in big-endian.
    ///
    /// # Panics
    /// The function panics if `data` is not four octets long.
    pub fn from_bytes(data: &[u8]) -> Ipv4Address {
        let mut bytes = [0; IPV4_ADDR_SIZE];
        bytes.copy_from_slice(data);
        Ipv4Address(bytes)
    }

    /// Return an IPv4 address as a sequence of octets, in big-endian.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Query whether the address is an unicast address.
    pub fn is_unicast(&self) -> bool {
        !(self.is_broadcast() || self.is_multicast() || self.is_unspecified())
    }

    /// Query whether the address is the broadcast address.
    pub fn is_broadcast(&self) -> bool {
        self.0[0..4] == [255; IPV4_ADDR_SIZE]
    }

    /// Query whether the address is a multicast address.
    pub const fn is_multicast(&self) -> bool {
        self.0[0] & 0xf0 == 224
    }

    /// Query whether the address falls into the "unspecified" range.
    pub const fn is_unspecified(&self) -> bool {
        self.0[0] == 0
    }

    /// Query whether the address falls into the "link-local" range.
    pub fn is_link_local(&self) -> bool {
        self.0[0..2] == [169, 254]
    }

    /// Query whether the address falls into the "loopback" range.
    pub const fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    /// Convert to an `IpAddress`.
    ///
    /// Same as `.into()`, but works in `const`.
    pub const fn into_address(self) -> Address {
        Address::Ipv4(self)
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes = self.0;
        write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

/// Size of IPv4 adderess in octets.
///
/// [RFC 8200 § 2]: https://www.rfc-editor.org/rfc/rfc791#section-3.2
pub const IPV4_ADDR_SIZE: usize = 4;

/// Size of IPv6 adderess in octets.
///
/// [RFC 8200 § 2]: https://www.rfc-editor.org/rfc/rfc4291#section-2
pub const IPV6_ADDR_SIZE: usize = 16;

/// Size of IPv4-mapping prefix in octets.
///
/// [RFC 8200 § 2]: https://www.rfc-editor.org/rfc/rfc4291#section-2
pub const IPV4_MAPPED_PREFIX_SIZE: usize = IPV6_ADDR_SIZE - IPV4_ADDR_SIZE;

/// Internet protocol version.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Version {
    Ipv4,
    Ipv6,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Version::Ipv4 => write!(f, "IPv4"),
            Version::Ipv6 => write!(f, "IPv6"),
        }
    }
}

/// A sixteen-octet IPv6 address.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ipv6Address(pub [u8; IPV6_ADDR_SIZE]);

impl Ipv6Address {
    /// The [unspecified address].
    ///
    /// [unspecified address]: https://tools.ietf.org/html/rfc4291#section-2.5.2
    pub const UNSPECIFIED: Ipv6Address = Ipv6Address([0x00; IPV6_ADDR_SIZE]);

    /// The link-local [all nodes multicast address].
    ///
    /// [all nodes multicast address]: https://tools.ietf.org/html/rfc4291#section-2.7.1
    pub const LINK_LOCAL_ALL_NODES: Ipv6Address = Ipv6Address([
        0xff, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01,
    ]);

    /// The link-local [all routers multicast address].
    ///
    /// [all routers multicast address]: https://tools.ietf.org/html/rfc4291#section-2.7.1
    pub const LINK_LOCAL_ALL_ROUTERS: Ipv6Address = Ipv6Address([
        0xff, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x02,
    ]);

    /// The [loopback address].
    ///
    /// [loopback address]: https://tools.ietf.org/html/rfc4291#section-2.5.3
    pub const LOOPBACK: Ipv6Address = Ipv6Address([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01,
    ]);

    /// The prefix used in [IPv4-mapped addresses].
    ///
    /// [IPv4-mapped addresses]: https://www.rfc-editor.org/rfc/rfc4291#section-2.5.5.2
    pub const IPV4_MAPPED_PREFIX: [u8; IPV4_MAPPED_PREFIX_SIZE] =
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

    /// Construct an IPv6 address from parts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a0: u16,
        a1: u16,
        a2: u16,
        a3: u16,
        a4: u16,
        a5: u16,
        a6: u16,
        a7: u16,
    ) -> Ipv6Address {
        let mut addr = [0u8; IPV6_ADDR_SIZE];
        NetworkEndian::write_u16(&mut addr[..2], a0);
        NetworkEndian::write_u16(&mut addr[2..4], a1);
        NetworkEndian::write_u16(&mut addr[4..6], a2);
        NetworkEndian::write_u16(&mut addr[6..8], a3);
        NetworkEndian::write_u16(&mut addr[8..10], a4);
        NetworkEndian::write_u16(&mut addr[10..12], a5);
        NetworkEndian::write_u16(&mut addr[12..14], a6);
        NetworkEndian::write_u16(&mut addr[14..], a7);
        Ipv6Address(addr)
    }

    /// Construct an IPv6 address from a sequence of octets, in big-endian.
    ///
    /// # Panics
    /// The function panics if `data` is not sixteen octets long.
    pub fn from_bytes(data: &[u8]) -> Ipv6Address {
        let mut bytes = [0; IPV6_ADDR_SIZE];
        bytes.copy_from_slice(data);
        Ipv6Address(bytes)
    }

    /// Construct an IPv6 address from a sequence of words, in big-endian.
    ///
    /// # Panics
    /// The function panics if `data` is not 8 words long.
    pub fn from_parts(data: &[u16]) -> Ipv6Address {
        assert!(data.len() >= 8);
        let mut bytes = [0; IPV6_ADDR_SIZE];
        for (word_idx, chunk) in bytes.chunks_mut(2).enumerate() {
            NetworkEndian::write_u16(chunk, data[word_idx]);
        }
        Ipv6Address(bytes)
    }

    /// Write a IPv6 address to the given slice.
    ///
    /// # Panics
    /// The function panics if `data` is not 8 words long.
    pub fn write_parts(&self, data: &mut [u16]) {
        assert!(data.len() >= 8);
        for (i, chunk) in self.0.chunks(2).enumerate() {
            data[i] = NetworkEndian::read_u16(chunk);
        }
    }

    /// Return an IPv6 address as a sequence of octets, in big-endian.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Query whether the IPv6 address is an [unicast address].
    ///
    /// [unicast address]: https://tools.ietf.org/html/rfc4291#section-2.5
    pub fn is_unicast(&self) -> bool {
        !(self.is_multicast() || self.is_unspecified())
    }

    /// Query whether the IPv6 address is a [multicast address].
    ///
    /// [multicast address]: https://tools.ietf.org/html/rfc4291#section-2.7
    pub fn is_multicast(&self) -> bool {
        self.0[0] == 0xff
    }

    /// Query whether the IPv6 address is the [unspecified address].
    ///
    /// [unspecified address]: https://tools.ietf.org/html/rfc4291#section-2.5.2
    pub fn is_unspecified(&self) -> bool {
        self.0 == [0x00; IPV6_ADDR_SIZE]
    }

    /// Query whether the IPv6 address is in the [link-local] scope.
    ///
    /// [link-local]: https://tools.ietf.org/html/rfc4291#section-2.5.6
    pub fn is_link_local(&self) -> bool {
        self.0[0..8] == [0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    }

    /// Query whether the IPv6 address is the [loopback address].
    ///
    /// [loopback address]: https://tools.ietf.org/html/rfc4291#section-2.5.3
    pub fn is_loopback(&self) -> bool {
        *self == Self::LOOPBACK
    }

    /// Query whether the IPv6 address is an [IPv4 mapped IPv6 address].
    ///
    /// [IPv4 mapped IPv6 address]: https://tools.ietf.org/html/rfc4291#section-2.5.5.2
    pub fn is_ipv4_mapped(&self) -> bool {
        self.0[..IPV4_MAPPED_PREFIX_SIZE] == Self::IPV4_MAPPED_PREFIX
    }

    /// Convert an IPv4 mapped IPv6 address to an IPv4 address.
    pub fn as_ipv4(&self) -> Option<Ipv4Address> {
        if self.is_ipv4_mapped() {
            Some(Ipv4Address::from_bytes(&self.0[IPV4_MAPPED_PREFIX_SIZE..]))
        } else {
            None
        }
    }

    /// Helper function used to mask an address given a prefix.
    ///
    /// # Panics
    /// This function panics if `mask` is greater than 128.
    pub(super) fn mask(&self, mask: u8) -> [u8; IPV6_ADDR_SIZE] {
        assert!(mask <= 128);
        let mut bytes = [0u8; IPV6_ADDR_SIZE];
        let idx = (mask as usize) / 8;
        let modulus = (mask as usize) % 8;
        let (first, second) = self.0.split_at(idx);
        bytes[0..idx].copy_from_slice(first);
        if idx < IPV6_ADDR_SIZE {
            let part = second[0];
            bytes[idx] = part & (!(0xff >> modulus) as u8);
        }
        bytes
    }

    /// The solicited node for the given unicast address.
    ///
    /// # Panics
    /// This function panics if the given address is not
    /// unicast.
    pub fn solicited_node(&self) -> Ipv6Address {
        assert!(self.is_unicast());
        Ipv6Address([
            0xff, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xFF,
            self.0[13], self.0[14], self.0[15],
        ])
    }

    /// Convert to an `IpAddress`.
    ///
    /// Same as `.into()`, but works in `const`.
    pub const fn into_address(self) -> Address {
        Address::Ipv6(self)
    }
}

impl fmt::Display for Ipv6Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_ipv4_mapped() {
            return write!(
                f,
                "::ffff:{}.{}.{}.{}",
                self.0[IPV4_MAPPED_PREFIX_SIZE + 0],
                self.0[IPV4_MAPPED_PREFIX_SIZE + 1],
                self.0[IPV4_MAPPED_PREFIX_SIZE + 2],
                self.0[IPV4_MAPPED_PREFIX_SIZE + 3]
            );
        }

        // The string representation of an IPv6 address should
        // collapse a series of 16 bit sections that evaluate
        // to 0 to "::"
        //
        // See https://tools.ietf.org/html/rfc4291#section-2.2
        // for details.
        enum State {
            Head,
            HeadBody,
            Tail,
            TailBody,
        }
        let mut words = [0u16; 8];
        self.write_parts(&mut words);
        let mut state = State::Head;
        for word in words.iter() {
            state = match (*word, &state) {
                // Once a u16 equal to zero write a double colon and
                // skip to the next non-zero u16.
                (0, &State::Head) | (0, &State::HeadBody) => {
                    write!(f, "::")?;
                    State::Tail
                }
                // Continue iterating without writing any characters until
                // we hit a non-zero value.
                (0, &State::Tail) => State::Tail,
                // When the state is Head or Tail write a u16 in hexadecimal
                // without the leading colon if the value is not 0.
                (_, &State::Head) => {
                    write!(f, "{:x}", word)?;
                    State::HeadBody
                }
                (_, &State::Tail) => {
                    write!(f, "{:x}", word)?;
                    State::TailBody
                }
                // Write the u16 with a leading colon when parsing a value
                // that isn't the first in a section
                (_, &State::HeadBody) | (_, &State::TailBody) => {
                    write!(f, ":{:x}", word)?;
                    state
                }
            }
        }
        Ok(())
    }
}

/// Convert the given IPv4 address into a IPv4-mapped IPv6 address
impl From<Ipv4Address> for Ipv6Address {
    fn from(address: Ipv4Address) -> Self {
        let mut b = [0_u8; IPV6_ADDR_SIZE];
        b[..Self::IPV4_MAPPED_PREFIX.len()].copy_from_slice(&Self::IPV4_MAPPED_PREFIX);
        b[Self::IPV4_MAPPED_PREFIX.len()..].copy_from_slice(&address.0);
        Self(b)
    }
}

macro_rules! write_slice {
    ($src:expr, $dst:expr, $ty:ty, $size:expr, $write:expr) => {{
        assert!($size == ::core::mem::size_of::<$ty>());
        assert_eq!($size * $src.len(), $dst.len());

        for (&n, chunk) in $src.iter().zip($dst.chunks_mut($size)) {
            $write(chunk, n);
        }
    }};
}

/// Copies $size bytes from a number $n to a &mut [u8] $dst. $ty represents the
/// numeric type of $n and $which must be either to_be or to_le, depending on
/// which endianness one wants to use when writing to $dst.
///
/// This macro is only safe to call when $ty is a numeric type and $size ==
/// size_of::<$ty>() and where $dst is a &mut [u8].
macro_rules! unsafe_write_num_bytes {
    ($ty:ty, $size:expr, $n:expr, $dst:expr, $which:ident) => {{
        assert!($size <= $dst.len());
        unsafe {
            // N.B. https://github.com/rust-lang/rust/issues/22776
            let bytes = *(&$n.$which() as *const _ as *const [u8; $size]);
            copy_nonoverlapping((&bytes).as_ptr(), $dst.as_mut_ptr(), $size);
        }
    }};
}

/// Copies a &[u8] $src into a &mut [<numeric>] $dst for the endianness given
/// by $which (must be either to_be or to_le).
///
/// This macro is only safe to call when $src and $dst are &[u8] and &mut [u8],
/// respectively. The macro will panic if $src.len() != $size * $dst.len(),
/// where $size represents the size of the integers encoded in $src.
macro_rules! unsafe_read_slice {
    ($src:expr, $dst:expr, $size:expr, $which:ident) => {{
        assert_eq!($src.len(), $size * $dst.len());

        unsafe {
            copy_nonoverlapping($src.as_ptr(), $dst.as_mut_ptr() as *mut u8, $src.len());
        }
        for v in $dst.iter_mut() {
            *v = v.$which();
        }
    }};
}

/// Copies a &[$ty] $src into a &mut [u8] $dst, where $ty must be a numeric
/// type. This panics if size_of::<$ty>() * $src.len() != $dst.len().
///
/// This macro is only safe to call when $src is a slice of numeric types and
/// $dst is a &mut [u8] and where $ty represents the type of the integers in
/// $src.
macro_rules! unsafe_write_slice_native {
    ($src:expr, $dst:expr, $ty:ty) => {{
        let size = core::mem::size_of::<$ty>();
        assert_eq!(size * $src.len(), $dst.len());

        unsafe {
            copy_nonoverlapping($src.as_ptr() as *const u8, $dst.as_mut_ptr(), $dst.len());
        }
    }};
}

/// Defines big-endian serialization.
///
/// Note that this type has no value constructor. It is used purely at the
/// type level.
///
/// # Examples
///
/// Write and read `u32` numbers in big endian order:
///
/// ```rust
/// use byteorder::{ByteOrder, BigEndian};
///
/// let mut buf = [0; 4];
/// BigEndian::write_u32(&mut buf, 1_000_000);
/// assert_eq!(1_000_000, BigEndian::read_u32(&buf));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BigEndian {}

impl Default for BigEndian {
    fn default() -> BigEndian {
        panic!("BigEndian default")
    }
}

pub type NetworkEndian = BigEndian;

impl BigEndian {
    #[inline]
    fn read_u16(buf: &[u8]) -> u16 {
        u16::from_be_bytes(buf[..2].try_into().unwrap())
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u32(buf: &[u8]) -> u32 {
        u32::from_be_bytes(buf[..4].try_into().unwrap())
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u64(buf: &[u8]) -> u64 {
        u64::from_be_bytes(buf[..8].try_into().unwrap())
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u128(buf: &[u8]) -> u128 {
        u128::from_be_bytes(buf[..16].try_into().unwrap())
    }

    #[inline]
    #[allow(dead_code)]
    fn read_uint(buf: &[u8], nbytes: usize) -> u64 {
        assert!(1 <= nbytes && nbytes <= 8 && nbytes <= buf.len());
        let mut out = 0u64;
        let ptr_out = &mut out as *mut u64 as *mut u8;
        unsafe {
            copy_nonoverlapping(buf.as_ptr(), ptr_out.offset((8 - nbytes) as isize), nbytes);
        }
        out.to_be()
    }

    #[inline]
    #[allow(dead_code)]
    fn read_uint128(buf: &[u8], nbytes: usize) -> u128 {
        assert!(1 <= nbytes && nbytes <= 16 && nbytes <= buf.len());
        let mut out: u128 = 0;
        let ptr_out = &mut out as *mut u128 as *mut u8;
        unsafe {
            copy_nonoverlapping(buf.as_ptr(), ptr_out.offset((16 - nbytes) as isize), nbytes);
        }
        out.to_be()
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u16(buf: &mut [u8], n: u16) {
        unsafe_write_num_bytes!(u16, 2, n, buf, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u32(buf: &mut [u8], n: u32) {
        unsafe_write_num_bytes!(u32, 4, n, buf, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u64(buf: &mut [u8], n: u64) {
        unsafe_write_num_bytes!(u64, 8, n, buf, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u128(buf: &mut [u8], n: u128) {
        unsafe_write_num_bytes!(u128, 16, n, buf, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn write_uint(buf: &mut [u8], n: u64, nbytes: usize) {
        assert!(pack_size(n) <= nbytes && nbytes <= 8);
        assert!(nbytes <= buf.len());
        unsafe {
            let bytes = *(&n.to_be() as *const u64 as *const [u8; 8]);
            copy_nonoverlapping(
                bytes.as_ptr().offset((8 - nbytes) as isize),
                buf.as_mut_ptr(),
                nbytes,
            );
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn write_uint128(buf: &mut [u8], n: u128, nbytes: usize) {
        assert!(pack_size128(n) <= nbytes && nbytes <= 16);
        assert!(nbytes <= buf.len());
        unsafe {
            let bytes = *(&n.to_be() as *const u128 as *const [u8; 16]);
            copy_nonoverlapping(
                bytes.as_ptr().offset((16 - nbytes) as isize),
                buf.as_mut_ptr(),
                nbytes,
            );
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u16_into(src: &[u8], dst: &mut [u16]) {
        unsafe_read_slice!(src, dst, 2, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u32_into(src: &[u8], dst: &mut [u32]) {
        unsafe_read_slice!(src, dst, 4, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u64_into(src: &[u8], dst: &mut [u64]) {
        unsafe_read_slice!(src, dst, 8, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn read_u128_into(src: &[u8], dst: &mut [u128]) {
        unsafe_read_slice!(src, dst, 16, to_be);
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u16_into(src: &[u16], dst: &mut [u8]) {
        if cfg!(target_endian = "big") {
            unsafe_write_slice_native!(src, dst, u16);
        } else {
            write_slice!(src, dst, u16, 2, Self::write_u16);
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u32_into(src: &[u32], dst: &mut [u8]) {
        if cfg!(target_endian = "big") {
            unsafe_write_slice_native!(src, dst, u32);
        } else {
            write_slice!(src, dst, u32, 4, Self::write_u32);
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u64_into(src: &[u64], dst: &mut [u8]) {
        if cfg!(target_endian = "big") {
            unsafe_write_slice_native!(src, dst, u64);
        } else {
            write_slice!(src, dst, u64, 8, Self::write_u64);
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn write_u128_into(src: &[u128], dst: &mut [u8]) {
        if cfg!(target_endian = "big") {
            unsafe_write_slice_native!(src, dst, u128);
        } else {
            write_slice!(src, dst, u128, 16, Self::write_u128);
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn from_slice_u16(numbers: &mut [u16]) {
        if cfg!(target_endian = "little") {
            for n in numbers {
                *n = n.to_be();
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn from_slice_u32(numbers: &mut [u32]) {
        if cfg!(target_endian = "little") {
            for n in numbers {
                *n = n.to_be();
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn from_slice_u64(numbers: &mut [u64]) {
        if cfg!(target_endian = "little") {
            for n in numbers {
                *n = n.to_be();
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn from_slice_u128(numbers: &mut [u128]) {
        if cfg!(target_endian = "little") {
            for n in numbers {
                *n = n.to_be();
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn from_slice_f32(numbers: &mut [f32]) {
        if cfg!(target_endian = "little") {
            for n in numbers {
                unsafe {
                    let int = *(n as *const f32 as *const u32);
                    *n = *(&int.to_be() as *const u32 as *const f32);
                }
            }
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn from_slice_f64(numbers: &mut [f64]) {
        if cfg!(target_endian = "little") {
            for n in numbers {
                unsafe {
                    let int = *(n as *const f64 as *const u64);
                    *n = *(&int.to_be() as *const u64 as *const f64);
                }
            }
        }
    }
}

#[inline]
fn pack_size(n: u64) -> usize {
    if n < 1 << 8 {
        1
    } else if n < 1 << 16 {
        2
    } else if n < 1 << 24 {
        3
    } else if n < 1 << 32 {
        4
    } else if n < 1 << 40 {
        5
    } else if n < 1 << 48 {
        6
    } else if n < 1 << 56 {
        7
    } else {
        8
    }
}

#[inline]
fn pack_size128(n: u128) -> usize {
    if n < 1 << 8 {
        1
    } else if n < 1 << 16 {
        2
    } else if n < 1 << 24 {
        3
    } else if n < 1 << 32 {
        4
    } else if n < 1 << 40 {
        5
    } else if n < 1 << 48 {
        6
    } else if n < 1 << 56 {
        7
    } else if n < 1 << 64 {
        8
    } else if n < 1 << 72 {
        9
    } else if n < 1 << 80 {
        10
    } else if n < 1 << 88 {
        11
    } else if n < 1 << 96 {
        12
    } else if n < 1 << 104 {
        13
    } else if n < 1 << 112 {
        14
    } else if n < 1 << 120 {
        15
    } else {
        16
    }
}
