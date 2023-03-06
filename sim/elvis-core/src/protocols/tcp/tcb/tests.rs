#![allow(unused_must_use)]

use super::*;

const PEER_A_ID: ConnectionId = ConnectionId {
    local: Socket {
        address: Ipv4Address::new([0, 0, 0, 0]),
        port: 0xcafe,
    },
    remote: Socket {
        address: Ipv4Address::new([0, 0, 0, 1]),
        port: 0xdead,
    },
};

const PEER_B_ID: ConnectionId = PEER_A_ID.reverse();

#[test]
fn basic_synchronization() {
    // Based on 3.5 Figure 6:
    //
    //     TCP Peer A                                            TCP Peer B
    // 1.  CLOSED                                                LISTEN
    // 2.  SYN-SENT    --> <SEQ=100><CTL=SYN>                --> SYN-RECEIVED
    // 3.  ESTABLISHED <-- <SEQ=300><ACK=101><CTL=SYN,ACK>   <-- SYN-RECEIVED
    // 4.  ESTABLISHED --> <SEQ=101><ACK=301><CTL=ACK>       --> ESTABLISHED
    // 5.  ESTABLISHED --> <SEQ=101><ACK=301><CTL=ACK><DATA> --> ESTABLISHED

    // 2
    let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
    assert_eq!(peer_a.state, State::SynSent);
    let peer_a_syn = peer_a.segments().remove(0);
    assert_eq!(peer_a_syn.header.seq, 100);
    assert!(peer_a_syn.header.ctl.syn());

    let mut peer_b = segment_arrives_listen(
        peer_a_syn,
        PEER_B_ID.local.address,
        PEER_B_ID.remote.address,
        300,
        1500,
    )
    .unwrap()
    .tcb()
    .unwrap();
    assert_eq!(peer_b.state, State::SynReceived);

    // 3
    let peer_b_syn_ack = peer_b.segments().remove(0);
    assert_eq!(peer_b_syn_ack.header.seq, 300);
    assert_eq!(peer_b_syn_ack.header.ack, 101);
    assert!(peer_b_syn_ack.header.ctl.syn());
    assert!(peer_b_syn_ack.header.ctl.ack());

    peer_a.segment_arrives(peer_b_syn_ack);
    assert_eq!(peer_a.state, State::Established);

    // 4
    let peer_a_ack = peer_a.segments().remove(0);
    assert_eq!(peer_a_ack.header.seq, 101);
    assert_eq!(peer_a_ack.header.ack, 301);
    assert!(peer_a_ack.header.ctl.ack());

    peer_b.segment_arrives(peer_a_ack);
    assert_eq!(peer_b.state, State::Established);

    // 5 TODO(hardint): Needs data segment transmission to work
}

#[test]
fn simultaneous_initiation() {
    // Based on 3.5 Figure 7:
    //
    //     TCP Peer A                                       TCP Peer B
    // 1.  CLOSED                                           CLOSED
    // 2.  SYN-SENT     --> <SEQ=100><CTL=SYN>              ...
    // 3.  SYN-RECEIVED <-- <SEQ=300><CTL=SYN>              <-- SYN-SENT
    // 4.               ... <SEQ=100><CTL=SYN>              --> SYN-RECEIVED
    // 5.  SYN-RECEIVED --> <SEQ=100><ACK=301><CTL=SYN,ACK> ...
    // 6.  ESTABLISHED  <-- <SEQ=300><ACK=101><CTL=SYN,ACK> <-- SYN-RECEIVED
    // 7.               ... <SEQ=100><ACK=301><CTL=SYN,ACK> --> ESTABLISHED

    // 2
    let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
    assert_eq!(peer_a.state, State::SynSent);
    let a_syn = peer_a.segments().remove(0);
    assert_eq!(a_syn.header.seq, 100);
    assert!(a_syn.header.ctl.syn());

    // 3
    let mut peer_b = Tcb::open(PEER_B_ID, 300, 1500);
    assert_eq!(peer_b.state, State::SynSent);
    let b_syn = peer_b.segments().remove(0);
    assert_eq!(b_syn.header.seq, 300);
    assert!(b_syn.header.ctl.syn());

    peer_a.segment_arrives(b_syn);
    assert_eq!(peer_a.state, State::SynReceived);

    // 4
    peer_b.segment_arrives(a_syn);
    assert_eq!(peer_b.state, State::SynReceived);

    // 5
    let a_syn_ack = peer_a.segments().remove(0);
    assert!(a_syn_ack.header.ctl.syn());
    assert!(a_syn_ack.header.ctl.ack());
    assert_eq!(a_syn_ack.header.seq, 100);
    assert_eq!(a_syn_ack.header.ack, 301);

    // 6
    let b_syn_ack = peer_b.segments().remove(0);
    assert!(b_syn_ack.header.ctl.syn());
    assert!(b_syn_ack.header.ctl.ack());
    assert_eq!(b_syn_ack.header.seq, 300);
    assert_eq!(b_syn_ack.header.ack, 101);

    peer_a.segment_arrives(b_syn_ack);
    assert_eq!(peer_a.state, State::Established);

    // 7
    peer_b.segment_arrives(a_syn_ack);
    assert_eq!(peer_b.state, State::Established);
}

#[test]
fn old_duplicate_syn() {
    // Based on 3.5 Figure 8:
    //
    //     TCP Peer A                                           TCP Peer B
    // 1.  CLOSED                                               LISTEN
    // 2.  SYN-SENT    --> <SEQ=100><CTL=SYN>               ...
    // 3.  (duplicate) ... <SEQ=90><CTL=SYN>                --> SYN-RECEIVED
    // 4.  SYN-SENT    <-- <SEQ=300><ACK=91><CTL=SYN,ACK>   <-- SYN-RECEIVED
    // 5.  SYN-SENT    --> <SEQ=91><CTL=RST>                --> LISTEN
    // 6.              ... <SEQ=100><CTL=SYN>               --> SYN-RECEIVED
    // 7.  ESTABLISHED <-- <SEQ=400><ACK=101><CTL=SYN,ACK>  <-- SYN-RECEIVED
    // 8.  ESTABLISHED --> <SEQ=101><ACK=401><CTL=ACK>      --> ESTABLISHED

    // 2
    let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
    let peer_a_syn = peer_a.segments().remove(0);
    assert!(peer_a_syn.header.ctl.syn());
    assert_eq!(peer_a_syn.header.seq, 100);

    // 3
    const GHOST_ID: ConnectionId = ConnectionId {
        local: Socket {
            address: Ipv4Address::new([123, 45, 67, 89]),
            port: 0xbabe,
        },
        remote: PEER_B_ID.local,
    };
    let mut ghost = Tcb::open(GHOST_ID, 90, 1500);
    let ghost_syn = ghost.segments().remove(0);
    assert!(ghost_syn.header.ctl.syn());
    assert_eq!(ghost_syn.header.seq, 90);

    let mut peer_b = segment_arrives_listen(
        ghost_syn,
        GHOST_ID.remote.address,
        GHOST_ID.local.address,
        300,
        1500,
    )
    .unwrap()
    .tcb()
    .unwrap();

    // 4
    let peer_b_syn_ack = peer_b.segments().remove(0);
    assert!(peer_b_syn_ack.header.ctl.syn());
    assert!(peer_b_syn_ack.header.ctl.ack());
    assert_eq!(peer_b_syn_ack.header.seq, 300);
    assert_eq!(peer_b_syn_ack.header.ack, 91);

    peer_a.segment_arrives(peer_b_syn_ack);
    assert_eq!(peer_a.state, State::SynSent);

    // 5
    let peer_a_rst = peer_a.segments().remove(0);
    assert!(peer_a_rst.header.ctl.rst());
    assert_eq!(peer_a_rst.header.seq, 91);

    let receive_result = peer_b.segment_arrives(peer_a_rst);
    assert_eq!(receive_result, SegmentArrivesResult::Close);

    // 6
    let mut peer_b = segment_arrives_listen(
        peer_a_syn,
        PEER_B_ID.local.address,
        PEER_B_ID.remote.address,
        400,
        1500,
    )
    .unwrap()
    .tcb()
    .unwrap();

    // 7
    let peer_b_syn_ack = peer_b.segments().remove(0);
    assert!(peer_b_syn_ack.header.ctl.syn());
    assert!(peer_b_syn_ack.header.ctl.ack());
    assert_eq!(peer_b_syn_ack.header.seq, 400);
    assert_eq!(peer_b_syn_ack.header.ack, 101);

    peer_a.segment_arrives(peer_b_syn_ack);
    assert_eq!(peer_a.state, State::Established);

    // 8
    let peer_a_ack = peer_a.segments().remove(0);
    assert!(peer_a_ack.header.ctl.ack());
    assert_eq!(peer_a_ack.header.seq, 101);
    assert_eq!(peer_a_ack.header.ack, 401);
}

// TODO(hardint): Add tests for the exchanges in figures 9 through 11 about
// half-open connections

fn established_pair(peer_a_iss: u32, peer_b_iss: u32) -> (Tcb, Tcb) {
    let mut peer_a = Tcb::open(PEER_A_ID, peer_a_iss, 1500);
    let peer_a_syn = peer_a.segments().remove(0);
    let mut peer_b = segment_arrives_listen(
        peer_a_syn,
        PEER_B_ID.local.address,
        PEER_B_ID.remote.address,
        peer_b_iss,
        1500,
    )
    .unwrap()
    .tcb()
    .unwrap();
    let peer_b_syn_ack = peer_b.segments().remove(0);
    peer_a.segment_arrives(peer_b_syn_ack);
    let peer_a_ack = peer_a.segments().remove(0);
    peer_b.segment_arrives(peer_a_ack);
    assert_eq!(peer_a.state, State::Established);
    assert_eq!(peer_b.state, State::Established);
    (peer_a, peer_b)
}

#[test]
fn normal_close_sequence() {
    // This test implements the following exchange from 3.6, Figure 12:
    //
    //     TCP Peer A                                           TCP Peer B
    //
    // 1.  ESTABLISHED                                          ESTABLISHED
    //
    // 2.  (Close)
    //     FIN-WAIT-1  --> <SEQ=100><ACK=300><CTL=FIN,ACK>  --> CLOSE-WAIT
    //
    // 3.  FIN-WAIT-2  <-- <SEQ=300><ACK=101><CTL=ACK>      <-- CLOSE-WAIT
    //
    // 4.                                                       (Close)
    //     TIME-WAIT   <-- <SEQ=300><ACK=101><CTL=FIN,ACK>  <-- LAST-ACK
    //
    // 5.  TIME-WAIT   --> <SEQ=101><ACK=301><CTL=ACK>      --> CLOSED
    //
    // 6.  (2 MSL)
    //     CLOSED
    //
    // NOTE: MSL = Maximum Segment Lifetime

    // 1
    let (mut peer_a, mut peer_b) = established_pair(99, 299);

    // 2
    peer_a.close();
    assert_eq!(peer_a.state, State::FinWait1);

    let peer_a_fin = peer_a.segments().remove(0);
    assert!(peer_a_fin.header.ctl.fin());
    assert!(peer_a_fin.header.ctl.ack());
    assert_eq!(peer_a_fin.header.seq, 100);
    assert_eq!(peer_a_fin.header.ack, 300);

    peer_b.segment_arrives(peer_a_fin);
    assert_eq!(peer_b.state, State::CloseWait);

    // 3
    let peer_b_ack = peer_b.segments().remove(0);
    assert!(peer_b_ack.header.ctl.ack());
    assert_eq!(peer_b_ack.header.seq, 300);
    assert_eq!(peer_b_ack.header.ack, 101);

    peer_a.segment_arrives(peer_b_ack);
    assert_eq!(peer_a.state, State::FinWait2);

    // 4
    peer_b.close();
    assert_eq!(peer_b.state, State::LastAck);

    let peer_b_fin = peer_b.segments().remove(0);
    assert!(peer_b_fin.header.ctl.fin());
    assert!(peer_b_fin.header.ctl.ack());
    assert_eq!(peer_b_fin.header.seq, 300);
    assert_eq!(peer_b_fin.header.ack, 101);

    peer_a.segment_arrives(peer_b_fin);
    assert_eq!(peer_a.state, State::TimeWait);

    // 5
    let peer_a_ack = peer_a.segments().remove(0);
    assert!(peer_a_ack.header.ctl.ack());
    assert_eq!(peer_a_ack.header.seq, 101);
    assert_eq!(peer_a_ack.header.ack, 301);

    let receive_result = peer_b.segment_arrives(peer_a_ack);
    assert_eq!(receive_result, SegmentArrivesResult::Close);

    let timeout = peer_a.advance_time(MSL.mul_f32(2.1));
    assert_eq!(timeout, AdvanceTimeResult::CloseConnection);
}

#[test]
fn simultaneous_close_sequence() {
    // This test implements the following exchange from 3.6, Figure 13:
    //
    //     TCP Peer A                                           TCP Peer B
    //
    // 1.  ESTABLISHED                                          ESTABLISHED
    //
    // 2.  (Close)                                              (Close)
    //     FIN-WAIT-1  --> <SEQ=100><ACK=300><CTL=FIN,ACK>  ... FIN-WAIT-1
    //                 <-- <SEQ=300><ACK=100><CTL=FIN,ACK>  <--
    //                 ... <SEQ=100><ACK=300><CTL=FIN,ACK>  -->
    //
    // 3.  CLOSING     --> <SEQ=101><ACK=301><CTL=ACK>      ... CLOSING
    //                 <-- <SEQ=301><ACK=101><CTL=ACK>      <--
    //                 ... <SEQ=101><ACK=301><CTL=ACK>      -->
    //
    // 4.  TIME-WAIT                                            TIME-WAIT
    //     (2 MSL)                                              (2 MSL)
    //     CLOSED                                               CLOSED

    // 1
    let (mut peer_a, mut peer_b) = established_pair(99, 299);

    // 2
    peer_a.close();
    assert_eq!(peer_a.state, State::FinWait1);
    let fin_ack_a = peer_a.segments().remove(0);
    assert_eq!(fin_ack_a.header.seq, 100);
    assert_eq!(fin_ack_a.header.ack, 300);
    assert!(fin_ack_a.header.ctl.fin());
    assert!(fin_ack_a.header.ctl.ack());

    peer_b.close();
    assert_eq!(peer_b.state, State::FinWait1);
    let fin_ack_b = peer_b.segments().remove(0);
    assert_eq!(fin_ack_b.header.seq, 300);
    assert_eq!(fin_ack_b.header.ack, 100);
    assert!(fin_ack_b.header.ctl.fin());
    assert!(fin_ack_b.header.ctl.ack());

    // 3
    peer_a.segment_arrives(fin_ack_b);
    assert_eq!(peer_a.state, State::Closing);
    let ack_a = peer_a.segments().remove(0);
    assert_eq!(ack_a.header.seq, 101);
    assert_eq!(ack_a.header.ack, 301);
    assert!(ack_a.header.ctl.ack());

    peer_b.segment_arrives(fin_ack_a);
    assert_eq!(peer_b.state, State::Closing);
    let ack_b = peer_b.segments().remove(0);
    assert_eq!(ack_b.header.seq, 301);
    assert_eq!(ack_b.header.ack, 101);
    assert!(ack_b.header.ctl.ack());

    // 4
    peer_a.segment_arrives(ack_b);
    assert_eq!(peer_a.state, State::TimeWait);
    assert_eq!(
        peer_a.advance_time(MSL.mul_f32(2.1)),
        AdvanceTimeResult::CloseConnection
    );

    peer_b.segment_arrives(ack_a);
    assert_eq!(peer_b.state, State::TimeWait);
    assert_eq!(
        peer_b.advance_time(MSL.mul_f32(2.1)),
        AdvanceTimeResult::CloseConnection
    );
}

#[test]
fn message_send() {
    let expected = b"Hello, world!";
    let (mut peer_a, mut peer_b) = established_pair(100, 300);
    peer_a.send(Message::new(expected));
    for outgoing in peer_a.segments() {
        peer_b.segment_arrives(outgoing);
    }
    let received = peer_b.receive();
    assert_eq!(&expected[..], &received.to_vec());
}

#[test]
fn message_segmentation() {
    let expected: Vec<_> = std::iter::repeat(0)
        .enumerate()
        .map(|(i, _)| i as u8)
        .take(4000)
        .collect();
    let (mut peer_a, mut peer_b) = established_pair(100, 300);
    peer_a.send(Message::new(expected.clone()));
    let mut count = 0;
    for outgoing in peer_a.segments() {
        count += 1;
        peer_b.segment_arrives(outgoing);
    }
    let received = peer_b.receive();
    assert_eq!(count, 3);
    assert_eq!(expected, received.to_vec());
}

#[test]
fn large_message_transmission() {
    let expected: Vec<_> = std::iter::repeat(0)
        .enumerate()
        .map(|(i, _)| i as u8)
        .take(8000) // This is beyond our receive window now
        .collect();
    let (mut peer_a, mut peer_b) = established_pair(100, 300);
    peer_a.send(Message::new(expected.clone()));
    let mut received = vec![];
    while received.len() != expected.len() {
        for outgoing in peer_a.segments() {
            peer_b.segment_arrives(outgoing);
        }
        received.extend(peer_b.receive().iter());
        for outgoing in peer_b.segments() {
            peer_a.segment_arrives(outgoing);
        }
        peer_a.advance_time(Duration::from_millis(1));
        peer_b.advance_time(Duration::from_millis(1));
    }
    assert_eq!(expected, received);
}

#[test]
fn message_retransmission() {
    let expected: Vec<_> = (0..8000).map(|i| i as u8).collect();
    let (mut peer_a, mut peer_b) = established_pair(100, 300);
    peer_a.send(Message::new(expected.clone()));
    let mut received = vec![];
    while received.len() < expected.len() {
        for outgoing in peer_a.segments() {
            if rand::random::<f32>() < 0.5 {
                peer_b.segment_arrives(outgoing);
            }
        }
        received.extend(peer_b.receive().iter());
        for outgoing in peer_b.segments() {
            if rand::random::<f32>() < 0.5 {
                peer_a.segment_arrives(outgoing);
            }
        }
        peer_a.advance_time(Duration::from_millis(1));
        peer_b.advance_time(Duration::from_millis(1));
    }
    assert_eq!(expected, received);
}

#[test]
fn out_of_order_delivery() {
    let expected: Vec<_> = std::iter::repeat(0)
        .enumerate()
        .map(|(i, _)| i as u8)
        .take(4000)
        .collect();
    let (mut peer_a, mut peer_b) = established_pair(100, 300);
    peer_a.send(Message::new(expected.clone()));
    let segments = peer_a.segments();
    for outgoing in segments.into_iter().rev() {
        peer_b.segment_arrives(outgoing);
    }
    let received = peer_b.receive();
    assert_eq!(expected, received.to_vec());
}

#[test]
fn loss_during_initiation() {
    let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
    peer_a.segments();
    peer_a.advance_time(Duration::from_secs(1));
    let peer_a_syn = peer_a.segments();
    assert_eq!(peer_a_syn.len(), 1);
    let peer_a_syn = peer_a_syn.into_iter().next().unwrap();

    let mut peer_b = segment_arrives_listen(
        peer_a_syn.clone(),
        PEER_B_ID.local.address,
        PEER_B_ID.remote.address,
        300,
        1500,
    )
    .unwrap()
    .tcb()
    .unwrap();
    peer_b.segments();
    peer_b.advance_time(Duration::from_secs(1));
    let peer_b_syn_ack = peer_b.segments();
    assert_eq!(peer_b_syn_ack.len(), 1);
    let peer_b_syn_ack = peer_b_syn_ack.into_iter().next().unwrap();
    assert!(peer_b_syn_ack.header.ctl.syn());
    assert!(peer_b_syn_ack.header.ctl.ack());
    // Lost packet arrives
    assert_eq!(peer_b.segment_arrives(peer_a_syn), SegmentArrivesResult::Ok);

    assert_eq!(
        peer_a.segment_arrives(peer_b_syn_ack.clone()),
        SegmentArrivesResult::Ok
    );
    assert_eq!(peer_a.state, State::Established);
    // Lost, new ACK not generated
    let _peer_a_ack = peer_a.segments();

    assert_eq!(
        // Peer B probes again
        peer_a.segment_arrives(peer_b_syn_ack.clone()),
        SegmentArrivesResult::Ok
    );
    peer_a.advance_time(Duration::from_secs(1));
    let peer_a_ack = peer_a.segments();

    assert_eq!(peer_a_ack.len(), 1);
    let peer_a_ack = peer_a_ack.into_iter().next().unwrap();
    assert!(peer_a_ack.header.ctl.ack());
    assert!(!peer_a_ack.header.ctl.syn());
    // Lost packet arrives
    assert_eq!(
        peer_a.segment_arrives(peer_b_syn_ack),
        SegmentArrivesResult::Ok
    );

    assert_eq!(peer_b.segment_arrives(peer_a_ack), SegmentArrivesResult::Ok);
    assert_eq!(peer_b.state, State::Established);
}

#[test]
fn send_before_established() {
    let mut peer_a = Tcb::open(PEER_A_ID, 100, 1500);
    peer_a.send(Message::new("Hello!"));
    let peer_a_syn = peer_a.segments().remove(0);
    let mut peer_b = segment_arrives_listen(
        peer_a_syn,
        PEER_B_ID.local.address,
        PEER_B_ID.remote.address,
        300,
        1500,
    )
    .unwrap()
    .tcb()
    .unwrap();
    peer_b.send(Message::new("Hi!"));
    for segment in peer_b.segments() {
        peer_a.segment_arrives(segment);
    }
    for segment in peer_a.segments() {
        peer_b.segment_arrives(segment);
    }
    assert_eq!(peer_a.state, State::Established);
    assert_eq!(peer_b.state, State::Established);
    assert_eq!(peer_a.receive().to_vec(), b"Hi!");
    assert_eq!(peer_b.receive().to_vec(), b"Hello!");
}

#[test]
#[ignore]
fn tcp_gig_isolation() {
    let expected: Vec<_> = std::iter::repeat(0)
        .enumerate()
        .map(|(i, _)| i as u8)
        .take(1_000_000_000)
        .collect();
    let (mut peer_a, mut peer_b) = established_pair(100, 300);
    peer_a.send(Message::new(expected.clone()));
    let mut received_bytes = 0;
    while received_bytes < expected.len() {
        for outgoing in peer_a.segments() {
            peer_b.segment_arrives(outgoing);
        }
        received_bytes += peer_b.receive().len();
        for outgoing in peer_b.segments() {
            peer_a.segment_arrives(outgoing);
        }
        peer_a.advance_time(Duration::from_secs(1));
        peer_b.advance_time(Duration::from_secs(1));
    }
    assert_eq!(received_bytes, 1_000_000_000);
}
