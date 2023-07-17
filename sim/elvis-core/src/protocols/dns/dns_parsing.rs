use crate::{protocols::ipv4::Ipv4Address, Message};
use thiserror::Error as ThisError;

pub enum DnsMessageType {
    // Indicates the message is a request for information.
    QUERY,
    // Indicates the message is responding to a request.
    RESPONSE,
}

/// A struct defining a simplified DNS message, implementation based upon
/// specification from RFC 1025 s.4. Currently holds fields for the 'Header',
/// 'Question', and 'Answer' message sections defined in RFC 1035. The
/// 'Authority' and 'Additional' fields may be added in the future as DNS
/// becomes more robust.
pub struct DnsMessage {
    pub header: DnsHeader,
    pub question: DnsQuestion,
    pub answer: DnsResourceRecord,
}

impl DnsMessage {
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };

        // parsing bytes for the DnsHeader
        let id = ((next()? as u16) << 8) | (next()? as u16);
        let properties = ((next()? as u16) << 8) | (next()? as u16);
        let qdcount = ((next()? as u16) << 8) | (next()? as u16);
        let ancount = ((next()? as u16) << 8) | (next()? as u16);
        let nscount = ((next()? as u16) << 8) | (next()? as u16);
        let arcount = ((next()? as u16) << 8) | (next()? as u16);

        // parsing bytes for the DnsQuestion
        let mut qname = Vec::new();
        let mut current = next()?;
        while current != b' ' {
            qname.push(current);
            current = next()?
        }
        let qtype = ((next()? as u16) << 8) | (next()? as u16);
        let qclass = ((next()? as u16) << 8) | (next()? as u16);

        // parsing bytes for the DnsResourceRecord for DnsMessage answer
        let mut name = Vec::new();
        let mut rdata: Vec<u8> = Vec::new();
        let mut current = next()?;
        while current != b' ' {
            name.push(current);
            current = next()?
        }
        let rec_type = ((next()? as u16) << 8) | (next()? as u16);
        let class = ((next()? as u16) << 8) | (next()? as u16);
        let mut ttl = next()? as u32;
        ttl = (ttl << 8) | next()? as u32;
        ttl = (ttl << 8) | next()? as u32;
        ttl = (ttl << 8) | next()? as u32;
        let rdlength = ((next()? as u16) << 8) | (next()? as u16);

        let mut i: u16 = 0;
        while i < rdlength {
            rdata.push(next()?);
            i += 1;
        }

        let header: DnsHeader = DnsHeader {
            id,
            properties,
            qdcount,
            ancount,
            nscount,
            arcount,
        };

        let question: DnsQuestion = DnsQuestion {
            qname,
            qtype,
            qclass,
        };

        let answer: DnsResourceRecord = DnsResourceRecord {
            name,
            rec_type,
            class,
            ttl,
            rdlength,
            rdata,
        };

        Ok(DnsMessage {
            header,
            question,
            answer,
        })
    }

    pub fn _get_type(&self) -> DnsMessageType {
        if (self.header.properties & (1 << 15)) == DnsMessageType::QUERY as u16 {
            DnsMessageType::QUERY
        } else {
            DnsMessageType::RESPONSE
        }
    }

    pub fn new(
        header: DnsHeader,
        question: DnsQuestion,
        answer: DnsResourceRecord,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            header,
            question,
            answer,
        })
    }

    pub fn to_message(self) -> Result<Message, ParseError> {
        let mut message_vec: Vec<u8> = Vec::new();
        message_vec.append(&mut DnsHeader::build(self.header));
        message_vec.append(&mut DnsQuestion::build(self.question));
        message_vec.append(&mut DnsResourceRecord::build(self.answer));

        Ok(Message::from(message_vec))
    }
}

/// A DNS header, as described in RFC 1035 p25 s4.1.1
pub struct DnsHeader {
    /// A 16 bit identifier assigned by the program that
    /// generates any kind of query.  This identifier is copied
    /// the corresponding reply and can be used by the requester
    ///  to match up replies to outstanding queries.
    pub id: u16,
    /// the 16 bit string that holds the following fields:
    /// QR, Opcode, AA, TC, RD, RA, Z, RCODE
    /// in the format 0 0000 0 0 0 0 000 0000
    pub properties: u16,
    /// the number of entries in the question section.
    pub qdcount: u16,
    /// the number of resource records in the answer section.
    pub ancount: u16,
    /// the number of name server resource records in the authority records section.
    pub nscount: u16,
    /// the number of resource records in the additional records section.
    pub arcount: u16,
}

impl DnsHeader {
    pub fn new(message_id: u16, message_type: DnsMessageType) -> DnsHeader {
        // Set to nothing for now
        let id = message_id;

        // as binary: 0 0000 0 0 0 0 000 0000
        // Leading bit denotes query or response, remaining fields present for
        // completeness
        let mut properties = 0x0;
        match message_type {
            DnsMessageType::QUERY => properties |= 0x0,
            DnsMessageType::RESPONSE => properties |= 0x8000,
        }

        // Remaining fields of header left as 0x0. Included for completeness.
        let qdcount = 0x0;
        let ancount = 0x0;
        let nscount = 0x0;
        let arcount = 0x0;

        DnsHeader {
            id,
            properties,
            qdcount,
            ancount,
            nscount,
            arcount,
        }
    }

    pub fn build(header: DnsHeader) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();
        my_vec.extend_from_slice(&header.id.to_be_bytes());
        my_vec.extend_from_slice(&header.properties.to_be_bytes());
        my_vec.extend_from_slice(&header.qdcount.to_be_bytes());
        my_vec.extend_from_slice(&header.ancount.to_be_bytes());
        my_vec.extend_from_slice(&header.nscount.to_be_bytes());
        my_vec.extend_from_slice(&header.arcount.to_be_bytes());
        my_vec
    }
}

/// A struct defining the question field of a DNS message as per the
/// RFC 1035 specification. Some fields will not be supported in current DNS,
/// they are present for completeness.
pub struct DnsQuestion {
    pub qname: Vec<u8>,
    qtype: u16,
    qclass: u16,
}

impl DnsQuestion {
    // Currently only supports making 'A' (address) type queries using a
    // String for the domain name. TODO: add in functionality for additional
    // types, specifically 'AAAA' as ipv6 comes into the simulation.
    pub fn new(domain_name: Vec<u8>) -> DnsQuestion {
        DnsQuestion {
            qname: domain_name,
            qtype: 1,
            qclass: 1,
        }
    }

    pub fn build(question: DnsQuestion) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();
        my_vec.append(&mut question.qname.clone());

        // Byte signaling end of string in message byte string.
        // More robust name encoding system is TODO.
        my_vec.append(&mut Vec::from([b' ']));

        my_vec.extend_from_slice(&question.qtype.to_be_bytes());
        my_vec.extend_from_slice(&question.qclass.to_be_bytes());
        my_vec
    }

    pub fn query_name(&self) -> Result<String, ParseError> {
        let name = String::from_utf8(self.qname.clone()).unwrap();
        Ok(name)
    }
}

/// A struct defining the resource records used for the 'Answer', 'Authority',
/// and 'Additional' fields of a DNS message as specified in RFC 1035. Some
/// fields remain unsupported and are present for completeness.
pub struct DnsResourceRecord {
    // name defined as string for ease of parsing.
    pub name: Vec<u8>,
    pub rec_type: u16,
    class: u16,
    pub ttl: u32,
    rdlength: u16,
    pub rdata: Vec<u8>,
}

impl DnsResourceRecord {
    // Currently only supports making 'A' (address) type records using a
    // String for the domain name. TODO: add in functionality for additional
    // types, specifically 'AAAA' as ipv6 comes into the simulation.
    pub fn new(
        domain_name: Vec<u8>,
        time_to_live: u32,
        record_data: Ipv4Address,
    ) -> DnsResourceRecord {
        DnsResourceRecord {
            name: domain_name,
            rec_type: 1,
            class: 1,
            ttl: time_to_live,
            rdlength: record_data.to_bytes().len() as u16,
            rdata: Vec::from(record_data.to_bytes()),
        }
    }

    pub fn build(mut answer: DnsResourceRecord) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();
        my_vec.append(&mut answer.name.clone());

        // Byte signaling end of string in message byte string.
        // More robust name encoding system is TODO.
        my_vec.append(&mut Vec::from([b' ']));

        my_vec.extend_from_slice(&answer.rec_type.to_be_bytes());
        my_vec.extend_from_slice(&answer.class.to_be_bytes());
        my_vec.extend_from_slice(&answer.ttl.to_be_bytes());
        my_vec.extend_from_slice(&answer.rdlength.to_be_bytes());
        my_vec.append(&mut answer.rdata);
        my_vec
    }
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The message or section is incomplete")]
    HeaderTooShort,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_dns_header() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE),
            DnsQuestion::new(Vec::from("google.com")),
            DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
            ),
        )
        .unwrap();

        let head = message.header;

        assert_eq!(head.id, 1337);
        assert_eq!(head.properties, 32768);
        assert_eq!(head.qdcount, 0);
        assert_eq!(head.ancount, 0);
        assert_eq!(head.nscount, 0);
        assert_eq!(head.arcount, 0);
    }

    #[test]
    fn read_dns_header() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::QUERY),
            DnsQuestion::new(Vec::from("google.com")),
            DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
            ),
        )
        .unwrap();

        let message_as_bytes: Vec<u8> = DnsMessage::to_message(message).unwrap().to_vec();

        let head_final: DnsHeader = DnsMessage::from_bytes(message_as_bytes.iter().cloned())
            .unwrap()
            .header;

        assert_eq!(head_final.id, 1337);
        assert_eq!(head_final.properties, 0);
        assert_eq!(head_final.qdcount, 0);
        assert_eq!(head_final.ancount, 0);
        assert_eq!(head_final.nscount, 0);
        assert_eq!(head_final.arcount, 0);
    }

    #[test]
    fn create_dns_question() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE),
            DnsQuestion::new(Vec::from("google.com")),
            DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
            ),
        )
        .unwrap();

        let question = message.question;

        let domain_string = String::from_utf8(question.qname).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(question.qtype, 1);
        assert_eq!(question.qclass, 1);
    }

    #[test]
    fn read_dns_question() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE),
            DnsQuestion::new(Vec::from("google.com")),
            DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
            ),
        )
        .unwrap();

        let message_as_bytes: Vec<u8> = DnsMessage::to_message(message).unwrap().to_vec();

        let question_final: DnsQuestion = DnsMessage::from_bytes(message_as_bytes.iter().cloned())
            .unwrap()
            .question;

        let domain_string = String::from_utf8(question_final.qname).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(question_final.qtype, 1);
        assert_eq!(question_final.qclass, 1);
    }

    #[test]
    fn create_dns_answer() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE),
            DnsQuestion::new(Vec::from("google.com")),
            DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
            ),
        )
        .unwrap();
        let rdata_len: u16 = message.answer.rdlength;

        let answer = message.answer;

        let domain_string = String::from_utf8(answer.name).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(answer.rec_type, 1);
        assert_eq!(answer.class, 1);
        assert_eq!(answer.ttl, 1600);
        assert_eq!(answer.rdlength, rdata_len);
        assert_eq!(answer.rdata, Vec::from([10u8, 11, 12, 13]));
    }

    #[test]
    fn read_dns_answer() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE),
            DnsQuestion::new(Vec::from("google.com")),
            DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
            ),
        )
        .unwrap();
        let rdata_len: u16 = message.answer.rdlength;

        let message_as_bytes: Vec<u8> = DnsMessage::to_message(message).unwrap().to_vec();

        let answer_final: DnsResourceRecord =
            DnsMessage::from_bytes(message_as_bytes.iter().cloned())
                .unwrap()
                .answer;

        let domain_string = String::from_utf8(answer_final.name).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(answer_final.rec_type, 1);
        assert_eq!(answer_final.class, 1);
        assert_eq!(answer_final.ttl, 1600);
        assert_eq!(answer_final.rdlength, rdata_len);
        assert_eq!(answer_final.rdata, Vec::from([10u8, 11, 12, 13]));
    }
}
