use crate::{protocols::ipv4::Ipv4Address, protocols::utility::BytesExt, Message};
use thiserror::Error as ThisError;

/// An enumeration representing the specific type or functionality of a DHCP Message
#[derive(Debug, PartialEq)]
pub enum MessageType {
    Discover = 1,
    Offer,
    Request,
    Decline,
    Ack,
    Nack,
    Release,
}

impl TryFrom<u8> for MessageType {
    type Error = ParseError;

    fn try_from(msg_type: u8) -> Result<Self, ParseError> {
        if msg_type > 7 {
            Err(ParseError::InvalidDhcpType)
        } else {
            Ok(match msg_type {
                1 => MessageType::Discover,
                2 => MessageType::Offer,
                3 => MessageType::Request,
                4 => MessageType::Decline,
                5 => MessageType::Ack,
                6 => MessageType::Nack,
                7 => MessageType::Release,
                _ => unreachable!(),
            })
        }
    }
}

/// A struct describing a DHCP message
#[derive(Debug, PartialEq)]
pub struct DhcpMessage {
    /// Whether message was request or reply
    pub op: u8,
    /// Network hardware type
    htype: u8,
    /// Length of hardware address
    hlen: u8,
    /// Number of machines the message has been passed to
    hops: u8,
    ///Diskless machines use this to map requests to responses
    transaction_id: u32,
    /// Number of seconds since client boot
    seconds: u16,
    /// Highest order bit denotes whether to respond via hardware broadcast or unicast
    flags: u8,
    /// Clients who know their IP put it here, blank otherwise
    client_ip: Ipv4Address,
    /// Server puts IP being granted here, blank otherwise
    pub your_ip: Ipv4Address,
    /// Address of sever client wants to request information from (blank if no server need be specified)
    server_ip: Ipv4Address,
    ///TODO: Still have no clue how to describe this, or if Ipv4 is correct to use. Probably not -JB 2/10
    router_ip: Ipv4Address,
    /// same as above
    client_hardware_address: u16,
    /// Name of server client wants to request information from (blank if no server need be specified)
    server_name: String,
    /// Name of boot file
    boot_file: String,
    /// Specifies what type of message (Request, ACK, etc.)
    pub msg_type: MessageType,
}

impl DhcpMessage {
    /// Parses a message into a DhcpMessage struct using a byte iterator
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        const HTS: ParseError = ParseError::HeaderTooShort;
        let op = bytes.next_u8().ok_or(HTS)?;
        let htype = bytes.next_u8().ok_or(HTS)?;
        let hlen = bytes.next_u8().ok_or(HTS)?;
        let hops = bytes.next_u8().ok_or(HTS)?;

        let transaction_id = bytes.next_u32_be().ok_or(HTS)?;

        let seconds = bytes.next_u16_be().ok_or(HTS)?;

        let flags = bytes.next_u8().ok_or(HTS)?;

        let client_ip = bytes.next_ipv4addr().ok_or(HTS)?;
        let your_ip = bytes.next_ipv4addr().ok_or(HTS)?;
        let server_ip = bytes.next_ipv4addr().ok_or(HTS)?;
        let router_ip = bytes.next_ipv4addr().ok_or(HTS)?;

        let client_hardware_address = bytes.next_u16_be().ok_or(HTS)?;
        let msg_type = MessageType::try_from(bytes.next_u8().ok_or(HTS)?).unwrap();
        let mut server_name = Vec::new();
        let mut current = bytes.next_u8().ok_or(HTS)?;
        while current != b'\0' {
            server_name.push(current);
            current = bytes.next_u8().ok_or(HTS)?
        }
        let server_name = String::from_utf8(server_name).unwrap();

        let mut boot_file = Vec::new();
        current = bytes.next_u8().ok_or(HTS)?;
        while current != b'\0' {
            boot_file.push(current);
            current = bytes.next_u8().ok_or(HTS)?
        }
        let boot_file = String::from_utf8(boot_file).unwrap();

        Ok(Self {
            op,
            htype,
            hlen,
            hops,
            transaction_id,
            seconds,
            flags,
            client_ip,
            your_ip,
            server_ip,
            router_ip,
            client_hardware_address,
            server_name,
            boot_file,
            msg_type,
        })
    }

    /// Converts a given DHCP struct into a message.
    pub fn to_message(message: DhcpMessage) -> Result<Message, ParseError> {
        let mut vec_message = vec![message.op, message.htype, message.hlen, message.hops];

        vec_message.extend_from_slice(&message.transaction_id.to_be_bytes());

        vec_message.extend_from_slice(&message.seconds.to_be_bytes());

        vec_message.push(message.flags);

        let client = Ipv4Address::to_bytes(message.client_ip);
        vec_message.extend(client);

        let your = Ipv4Address::to_bytes(message.your_ip);
        vec_message.extend(your);

        let server = Ipv4Address::to_bytes(message.server_ip);
        vec_message.extend(server);

        let router = Ipv4Address::to_bytes(message.router_ip);
        vec_message.extend(router);

        vec_message.extend_from_slice(&message.client_hardware_address.to_be_bytes());

        vec_message.push(message.msg_type as u8);

        vec_message.extend(message.server_name.as_bytes());
        vec_message.push(b'\0');
        vec_message.extend(message.boot_file.as_bytes());
        vec_message.push(b'\0');

        let ret_message = Message::new(vec_message);
        Ok(ret_message)
    }
}

impl Default for DhcpMessage {
    fn default() -> Self {
        Self {
            op: 50,
            htype: 50,
            hlen: 50,
            hops: 50,
            transaction_id: 0000,
            seconds: 00,
            flags: 0,
            client_ip: Ipv4Address::new([0, 0, 0, 0]),
            your_ip: Ipv4Address::new([0, 0, 0, 0]),
            server_ip: Ipv4Address::new([0, 0, 0, 0]),
            router_ip: Ipv4Address::new([0, 0, 0, 0]),
            client_hardware_address: 00,
            server_name: "Null".to_string(),
            boot_file: "BootFile".to_string(),
            msg_type: MessageType::Discover,
        }
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The DHCP message is incomplete")]
    HeaderTooShort,
    #[error("Invalid DHCP message type")]
    InvalidDhcpType,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_basic_header() -> anyhow::Result<()> {
        let mut msg = [
            1, 1, 1, 1, 0, 0, 0, 2, 1, 33, 1, 5, 5, 5, 5, 6, 6, 6, 2, 7, 7, 7, 2, 8, 8, 8, 2, 0,
            99, 1,
        ]
        .to_vec();
        msg.extend("Serv".as_bytes());
        msg.push(b'\0');
        msg.extend("BootFileBootFile".as_bytes());
        msg.push(b'\0');
        let mess = Message::new(msg);
        tracing::info!("{:?}", mess);
        let parsed = DhcpMessage::from_bytes(mess.iter())?;
        tracing::info!("{:?}", parsed);
        let test_struct = DhcpMessage {
            //struct to confirm we're getting expected values
            op: 1,
            htype: 1,
            hlen: 1,
            hops: 1,
            transaction_id: 2,
            seconds: 289,
            flags: 1,
            client_ip: Ipv4Address::new([5, 5, 5, 5]),
            your_ip: Ipv4Address::new([6, 6, 6, 2]),
            server_ip: Ipv4Address::new([7, 7, 7, 2]),
            router_ip: Ipv4Address::new([8, 8, 8, 2]),
            client_hardware_address: 99,
            server_name: "Serv".to_string(),
            boot_file: "BootFileBootFile".to_string(),
            msg_type: MessageType::Discover,
        };

        assert_eq!(parsed, test_struct);

        let redo = DhcpMessage::to_message(parsed)?; //this takes the struct parsed above and turns it back into a message
        let mut msg = [
            1,
            1,
            1,
            1,
            0,
            0,
            0,
            2,
            1,
            33,
            1,
            5,
            5,
            5,
            5,
            6,
            6,
            6,
            2,
            7,
            7,
            7,
            2,
            8,
            8,
            8,
            2,
            0,
            99,
            MessageType::Discover as u8,
        ]
        .to_vec();
        //remakes vector for comparison
        msg.extend("Serv".as_bytes());
        msg.push(b'\0');
        msg.extend("BootFileBootFile".as_bytes());
        msg.push(b'\0');
        tracing::info!("{:?}", msg);
        assert_eq!(redo, Message::new(msg));
        Ok(())
    }
}
