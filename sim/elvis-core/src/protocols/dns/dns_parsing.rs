use crate::{protocols::ipv4::Ipv4Address, Message};
use thiserror::Error as ThisError;

use super::domain_name::DomainName;

pub enum DnsMessageType {
    // Indicates the message is a request for information.
    QUERY,
    // Indicates the message is responding to a request.
    RESPONSE,
}

#[derive(Debug)]
/// A struct defining a simplified DNS message, implementation based upon
/// specification from RFC 1025 s.4. Currently holds fields for the 'Header',
/// 'Question', and 'Answer' message sections defined in RFC 1035. The
/// 'Authority' and 'Additional' fields may be added in the future as DNS
/// becomes more robust.
pub struct DnsMessage {
    pub header: DnsHeader,
    pub question: DnsQuestion,
    pub answers: Vec<DnsResourceRecord>,
    pub q_labels: Vec<String>,
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
        let mut q_labels: Vec<String> = Vec::new();

        // TODO COMMENT
        let mut label_len = next()?;
        let mut label_byte: u8;
        while label_len != 0 {
            let mut q_label: Vec<u8> = Vec::new();
            for _ in 0..label_len {
                label_byte = next()?;
                q_label.push(label_byte);
                qname.push(label_byte);
            }
            label_len = next()?;
            if label_len != 0 {
                qname.push(b'.');
            }
            q_labels.append(
                &mut Vec::from(
                    [String::from_utf8(q_label).expect("Err on parsing question name labels")]
                )
            );
        }        
        let qtype = ((next()? as u16) << 8) | (next()? as u16);
        let qclass = ((next()? as u16) << 8) | (next()? as u16);
        
        // parsing bytes for the DnsResourceRecord for DnsMessage answer
        let mut answer_vec: Vec<DnsResourceRecord> = Vec::new();
        let mut rec_count = 0;
        println!("{:?}", ancount);
        while rec_count < ancount {
            let mut name = Vec::new();
    
            // TODO COMMENT
            label_len = next()?;
            while label_len != 0 {
                let mut a_label: Vec<u8> = Vec::new();
                for _ in 0..label_len {
                    label_byte = next()?;
                    a_label.push(label_byte);
                    name.push(label_byte);
                }
                label_len = next()?;
                if label_len != 0 {
                    name.push(b'.');
                }
            }
            let rec_type = ((next()? as u16) << 8) | (next()? as u16);
            let class = ((next()? as u16) << 8) | (next()? as u16);
            let mut ttl = next()? as u32;
            ttl = (ttl << 8) | next()? as u32;
            ttl = (ttl << 8) | next()? as u32;
            ttl = (ttl << 8) | next()? as u32;
            let rdlength = ((next()? as u16) << 8) | (next()? as u16);
            
            let mut rdata: Vec<u8> = Vec::new();
            let mut i: u16 = 0;
            while i < rdlength {
                rdata.push(next()?);
                i += 1;
            }

            
            let answer: DnsResourceRecord = DnsResourceRecord {
                name_as_labels: DomainName::from(name.clone()),
                name,
                rec_type,
                class,
                ttl,
                rdlength,
                rdata,
            };
            answer_vec.append(&mut Vec::from([answer]));
            println!("Made it here");
            
            rec_count += 1;
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

        let answers: Vec<DnsResourceRecord> = answer_vec;

        Ok(DnsMessage {
            header,
            question,
            answers,
            q_labels,
        })
    }

    // TODO COMMENT
    pub fn encode_label(mut domain_name: Vec<u8>) -> Vec<u8> {

        let mut label_size = 0;
        let mut i = 0;
        while i < domain_name.len() {
            if domain_name[i] == b'.' {
                domain_name.insert(i - label_size, label_size as u8);
                domain_name.remove(i + 1);
                label_size = 0;
            } else if i == domain_name.len() - 1 {
                label_size += 1;
                domain_name.insert(i - label_size + 1, label_size as u8);
                i += 1;
                label_size = 0;
            } else {
                label_size += 1;
            }

            i += 1;
        }

        domain_name.insert(domain_name.len(), 0);

        domain_name
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
        answers: Vec<DnsResourceRecord>,
    ) -> Result<Self, ParseError> {
        let q_labels: Vec<String> = Vec::new();
        Ok(Self {
            header,
            question,
            answers,
            q_labels,
        })
    }

    pub fn to_message(self) -> Result<Message, ParseError> {
        let mut message_vec: Vec<u8> = Vec::new();
        message_vec.append(&mut DnsHeader::build(self.header.to_owned()));
        message_vec.append(&mut DnsQuestion::build(self.question.to_owned()));
        for i in 0..self.header.ancount as usize {
            message_vec.append(&mut DnsResourceRecord::build(self.answers[i].to_owned()));
        }

        Ok(Message::from(message_vec))
    }
}

#[derive(Debug, Clone)]
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
    pub fn new(message_id: u16, message_type: DnsMessageType, answer_count: u16) -> DnsHeader {
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

        // Answer Count now relevant.
        let ancount = answer_count;

        // Remaining fields of header left as 0x0. Included for completeness.
        let qdcount = 0x0;
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

/// An enum defining several of the '[QTYPE]'s that a DNS query can have.
/// This is the super-set containing '[TYPE]', but only includes the qtypes
/// for organization purposes.
pub enum DnsQtypes {
    AXFR,   // A request for the transfer of an entire zone.
    QALL,   // A request for ALL records, [QALL] is alias for "*".
}

#[derive(Debug, Clone)]
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
    pub fn new(domain_name: Vec<u8>, qtype: u16) -> DnsQuestion {
        DnsQuestion {
            qname: domain_name,
            qtype,
            qclass: 1,
        }
    }

    fn build(question: DnsQuestion) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();
        my_vec.append(&mut DnsMessage::encode_label(question.qname.clone()));

        my_vec.extend_from_slice(&question.qtype.to_be_bytes());
        my_vec.extend_from_slice(&question.qclass.to_be_bytes());
        my_vec
    }
}

/// An enum defining several of the '[TYPE]'s that a DNS resource
/// record could possibly be. These are a subset of [QTYPES].
pub enum DnsRTypes {
    A,      // An Ipv4 host address
    NS,     // A Name Space resource record
    CNAME,  // A cannonical domain name
    PTR,    // A pointer to another part of the DNS
    SOA,    // Start of zone of Authority
}

#[derive(Clone, Debug, PartialEq)]
/// A struct defining the resource records used for the 'Answer', 'Authority',
/// and 'Additional' fields of a DNS message as specified in RFC 1035. Some
/// fields remain unsupported and are present for completeness.
pub struct DnsResourceRecord {
    // name defined as Vec<u8> rather than string for future expansion on label system.
    pub name_as_labels: DomainName,
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
    // types, specifically 'AAAA' as ipv6 comes into the simulation 
    // and 'NS' for authoritative namespaces.
    pub fn new(
        name_as_bytes: Vec<u8>,
        time_to_live: u32,
        record_data: Ipv4Address,
        rec_type: u16
    ) -> DnsResourceRecord {
        DnsResourceRecord {
            name_as_labels: DomainName::from(name_as_bytes.clone()),
            name: name_as_bytes,
            rec_type: 1,
            class: 1,
            ttl: time_to_live,
            rdlength: record_data.to_bytes().len() as u16,
            rdata: Vec::from(record_data.to_bytes()),
        }
    }

    pub fn to_ipv4(&self) -> Ipv4Address {
        Ipv4Address::from([
            self.rdata[0],
            self.rdata[1],
            self.rdata[2],
            self.rdata[3]
        ])
    }

    pub fn build(mut answer: DnsResourceRecord) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();
        my_vec.append(&mut DnsMessage::encode_label(answer.name.clone()));

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
            DnsHeader::new(1337, DnsMessageType::RESPONSE, 1),
            DnsQuestion::new(Vec::from("google.com"), DnsRTypes::A as u16),
            Vec::from([DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
                DnsRTypes::A as u16
            )]),
        )
        .unwrap();

        let head = message.header;

        assert_eq!(head.id, 1337);
        assert_eq!(head.properties, 32768);
        assert_eq!(head.qdcount, 0);
        assert_eq!(head.ancount, 1);
        assert_eq!(head.nscount, 0);
        assert_eq!(head.arcount, 0);
    }

    #[test]
    fn read_dns_header() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::QUERY, 1),
            DnsQuestion::new(Vec::from("google.com"), DnsRTypes::A as u16),
            Vec::from([DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
                DnsRTypes::A as u16
            )]),
        )
        .unwrap();

        let message_as_bytes: Vec<u8> = DnsMessage::to_message(message).unwrap().to_vec();

        let head_final: DnsHeader = DnsMessage::from_bytes(message_as_bytes.iter().cloned())
            .unwrap()
            .header;

        assert_eq!(head_final.id, 1337);
        assert_eq!(head_final.properties, 0);
        assert_eq!(head_final.qdcount, 0);
        assert_eq!(head_final.ancount, 1);
        assert_eq!(head_final.nscount, 0);
        assert_eq!(head_final.arcount, 0);
    }

    #[test]
    fn create_dns_question() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE, 1),
            DnsQuestion::new(Vec::from("google.com"), DnsRTypes::A as u16),
            Vec::from([DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
                DnsRTypes::A as u16
            )]),
        )
        .unwrap();

        let question = message.question;

        let domain_string = String::from_utf8(question.qname).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(question.qtype, DnsRTypes::A as u16);
        assert_eq!(question.qclass, 1);
    }

    #[test]
    fn read_dns_question() {
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE, 1),
            DnsQuestion::new(Vec::from("google.com"), DnsRTypes::A as u16),
            Vec::from([DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
                DnsRTypes::A as u16
            )]),
        )
        .unwrap();

        let message_as_bytes: Vec<u8> = DnsMessage::to_message(message).unwrap().to_vec();

        let question_final: DnsQuestion = DnsMessage::from_bytes(message_as_bytes.iter().cloned())
            .unwrap()
            .question;

        let domain_string = String::from_utf8(question_final.qname).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(question_final.qtype, DnsRTypes::A as u16);
        assert_eq!(question_final.qclass, 1);
    }

    #[test]
    fn create_dns_answer() {
        // TODO: re-write tests for multi-rr messages
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE, 1),
            DnsQuestion::new(Vec::from("google.com"), DnsRTypes::A as u16),
            Vec::from([DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
                DnsRTypes::A as u16
            )]),
        )
        .unwrap();
        // let rdata_len: u16 = message.answers.rdlength;

        let answer = message.answers[0].to_owned();

        let domain_string = String::from_utf8(answer.name).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(answer.rec_type, 1);
        assert_eq!(answer.class, 1);
        assert_eq!(answer.ttl, 1600);
        // assert_eq!(answer.rdlength, rdata_len);
        assert_eq!(answer.rdata, Vec::from([10u8, 11, 12, 13]));
    }

    #[test]
    fn read_dns_answer() {
        // TODO: re-write tests for multi-rr messages
        let message: DnsMessage = DnsMessage::new(
            DnsHeader::new(1337, DnsMessageType::RESPONSE, 1),
            DnsQuestion::new(Vec::from("google.com"), DnsRTypes::A as u16),
            Vec::from([DnsResourceRecord::new(
                Vec::from("google.com"),
                1600,
                Ipv4Address::new([10u8, 11, 12, 13]),
                DnsRTypes::A as u16
            )]),
        )
        .unwrap();
        // let rdata_len: u16 = message.answer.rdlength;

        let message_as_bytes: Vec<u8> = DnsMessage::to_message(message).unwrap().to_vec();

        let answer_final: DnsResourceRecord =
            DnsMessage::from_bytes(message_as_bytes.iter().cloned())
                .unwrap()
                .answers[0].to_owned();

        let domain_string = String::from_utf8(answer_final.name).unwrap();

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(answer_final.rec_type, 1);
        assert_eq!(answer_final.class, 1);
        assert_eq!(answer_final.ttl, 1600);
        // assert_eq!(answer_final.rdlength, rdata_len);
        assert_eq!(answer_final.rdata, Vec::from([10u8, 11, 12, 13]));
    }
}
