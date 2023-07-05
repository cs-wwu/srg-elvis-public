// use crate::Message;
use crate::{message, protocols::ipv4::Ipv4Address};
// use message::Message;
// use Ipv4Address;


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
pub(super) struct DnsMessage {
    pub header: DnsHeader,
    question: DnsQuestion,
    answer: DnsResourceRecord,
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
        while current != b'\0' {
            qname.push(current);
            current = next()?
        }
        // let qname = String::from_utf8(qname).unwrap();
        let qtype = ((next()? as u16) << 8) | (next()? as u16);
        let qclass = ((next()? as u16) << 8) | (next()? as u16);

        // parsing bytes for the DnsResourceRecord for DnsMessage answer
        let mut name = Vec::new();
        let mut rdata: Vec<u8> = Vec::new();
        let mut current = next()?;
        while current != b'\0' {
            name.push(current);
            current = next()?
        }
        // let name = String::from_utf8(name).unwrap();
        let rec_type = ((next()? as u16) << 8) | (next()? as u16);
        let class = ((next()? as u16) << 8) | (next()? as u16);
        let mut ttl = next()? as u32;
        ttl = (ttl << 8) | next()? as u32;
        ttl = (ttl << 8) | next()? as u32;
        ttl = (ttl << 8) | next()? as u32;
        let rdlength = ((next()? as u16) << 8) | (next()? as u16);

        let mut i = 0;
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

        Ok(
            DnsMessage { 
                header,
                question,
                answer,
            }
        )
    }

    pub fn new(
        header: DnsHeader,
        question: DnsQuestion,
        answer: DnsResourceRecord,
    ) -> Result<Self, ParseError> {

        Ok(
            Self {
                header,
                question,
                answer,
            }
        )
    }

    pub fn to_message(message: DnsMessage) {

    }
}

// /// The number of `u32` words in a basic DNS header
// const BASE_WORDS: u8 = 6;
// /// The number of `u8` bytes in a basic IPv4 header
// const BASE_OCTETS: u16 = BASE_WORDS as u16 * 2;

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

        Ok(
            DnsHeader {
                id,
                properties,
                qdcount,
                ancount,
                nscount,
                arcount,
            }
        )
    }

    pub fn new(
        message_id: u16,
        message_type: DnsMessageType
    ) -> DnsHeader {
        // Set to nothing for now
        let id = message_id;

        // as binary: 0 0000 0 0 0 0 000 0000
        // Leading bit denotes query or response, remaining fields present for 
        // completeness
        let mut properties = 0x0;
        match message_type {
            DnsMessageType::QUERY       => properties |= 0x0,
            DnsMessageType::RESPONSE    => properties |= 0x8000,
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
    // qname defined as string for ease of parsing.
    qname: Vec<u8>,
    qtype: u16,
    qclass: u16,
}

impl DnsQuestion {
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };
        
        let mut qname = Vec::new();
        let mut current = next()?;
        while current != b' ' {
            qname.push(current);
            current = next()?;
        }
        let qtype = ((next()? as u16) << 8) | (next()? as u16);
        let qclass = ((next()? as u16) << 8) | (next()? as u16);

        Ok(
            DnsQuestion {
                qname,
                qtype,
                qclass,
            }
        )
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

}

/// A struct defining the resource records used for the 'Answer', 'Authority',
/// and 'Additional' fields of a DNS message as specified in RFC 1035. Some
/// fields remain unsupported and are present for completeness.
pub struct DnsResourceRecord {
    // name defined as string for ease of parsing.
    name: Vec<u8>,
    rec_type: u16,
    class: u16,
    ttl: u32,
    rdlength: u16,
    rdata: Vec<u8>,
}

impl DnsResourceRecord {
    pub fn from_bytes(mut bytes: impl Iterator<Item = u8>) -> Result<Self, ParseError> {
        let mut next =
            || -> Result<u8, ParseError> { bytes.next().ok_or(ParseError::HeaderTooShort) };
        
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

        Ok(
            DnsResourceRecord {
                name,
                rec_type,
                class,
                ttl,
                rdlength,
                rdata,
            }
        )
    }

    pub fn build(answer: DnsResourceRecord) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();
        my_vec.append(&mut answer.name.clone());

        // Byte signaling end of string in message byte string.
        // More robust name encoding system is TODO.
        my_vec.append(&mut Vec::from([b' ']));

        my_vec.extend_from_slice(&answer.rec_type.to_be_bytes());
        my_vec.extend_from_slice(&answer.class.to_be_bytes());
        my_vec.extend_from_slice(&answer.ttl.to_be_bytes());
        my_vec.extend_from_slice(&answer.rdlength.to_be_bytes());
        my_vec.append(&mut answer.rdata.clone());
        my_vec
    }
}



#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("The message or section is incomplete")]
    HeaderTooShort,
}

#[derive(Debug, ThisError, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    #[error("The message or section is invalid")]
    HeaderBadFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_dns_header() {
        let head: DnsHeader = DnsHeader::new(
            1337,
            DnsMessageType::RESPONSE,
        );

        println!("{:?}", head.id);
        println!("{:?}", head.properties);
        println!("{:?}", head.qdcount);
        println!("{:?}", head.ancount);
        println!("{:?}", head.nscount);
        println!("{:?}", head.arcount);

        assert_eq!(head.id, 1337);
        assert_eq!(head.properties, 32768);
        assert_eq!(head.qdcount, 0);
        assert_eq!(head.ancount, 0);
        assert_eq!(head.nscount, 0);
        assert_eq!(head.arcount, 0);
    }

    #[test]
    fn read_dns_header() {
        let head_init: DnsHeader = DnsHeader::new(
            1337,
            DnsMessageType::QUERY,
        );

        let head_as_bytes: Vec<u8> = DnsHeader::build(head_init);

        let head_final: DnsHeader = 
            DnsHeader::from_bytes(head_as_bytes.iter().cloned()).unwrap();

            println!("{:?}", head_final.id);
            println!("{:?}", head_final.properties);
            println!("{:?}", head_final.qdcount);
            println!("{:?}", head_final.ancount);
            println!("{:?}", head_final.nscount);
            println!("{:?}", head_final.arcount);
    
            assert_eq!(head_final.id, 1337);
            assert_eq!(head_final.properties, 0);
            assert_eq!(head_final.qdcount, 0);
            assert_eq!(head_final.ancount, 0);
            assert_eq!(head_final.nscount, 0);
            assert_eq!(head_final.arcount, 0);
    }

    #[test]
    fn build_dns_question() {
        let domain: String = "google.com".to_string();
        let question: DnsQuestion = DnsQuestion {
            qname: domain.into_bytes(),
            qtype: 100,
            qclass: 20,
        };

        let domain_string = String::from_utf8(question.qname).unwrap();
        println!("{:?}", domain_string);
        println!("{:?}", question.qtype);
        println!("{:?}", question.qclass);

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(question.qtype, 100);
        assert_eq!(question.qclass, 20);
    }

    #[test]
    fn read_dns_question() {
        let domain: String = "google.com".to_string();
        let question_init: DnsQuestion = DnsQuestion {
            qname: domain.into_bytes(),
            qtype: 13,
            qclass: 37,
        };

        let question_as_bytes: Vec<u8> = DnsQuestion::build(question_init);

        let question_final: DnsQuestion = 
            DnsQuestion::from_bytes(question_as_bytes.iter().cloned()).unwrap();

        let domain_string = String::from_utf8(question_final.qname).unwrap();
        println!("{:?}", domain_string);
        println!("{:?}", question_final.qtype);
        println!("{:?}", question_final.qclass);
    
        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(question_final.qtype, 13);
        assert_eq!(question_final.qclass, 37);
    }

    #[test]
    fn build_dns_answer() {
        let domain: String = "google.com".to_string();
        let rdata_vec: Vec<u8> = Vec::from([10u8, 11, 12, 13]);
        let rdata_len: u16 = rdata_vec.len() as u16;
        let answer: DnsResourceRecord = DnsResourceRecord {
            name: domain.into_bytes(),
            rec_type: 1,
            class: 1,
            ttl: 1600,
            rdlength: rdata_len,
            rdata: rdata_vec.clone(),
        };

        let domain_string = String::from_utf8(answer.name).unwrap();
        println!("{:?}", domain_string);
        println!("{:?}", answer.rec_type);
        println!("{:?}", answer.class);
        println!("{:?}", answer.ttl);
        println!("{:?}", answer.rdlength);
        println!("{:?}", answer.rdata);

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(answer.rec_type, 1);
        assert_eq!(answer.class, 1);
        assert_eq!(answer.ttl, 1600);
        assert_eq!(answer.rdlength, rdata_len);
        assert_eq!(answer.rdata, rdata_vec.clone());
    }

    #[test]
    fn read_dns_answer() {
        let domain: String = "google.com".to_string();
        let rdata_vec: Vec<u8> = Vec::from([10u8, 11, 12, 13]);
        let rdata_len: u16 = rdata_vec.len() as u16;
        let answer_init: DnsResourceRecord = DnsResourceRecord {
            name: domain.into_bytes(),
            rec_type: 1,
            class: 1,
            ttl: 1600,
            rdlength: rdata_len,
            rdata: rdata_vec.clone(),
        };

        let answer_as_bytes: Vec<u8> = DnsResourceRecord::build(answer_init);

        let answer_final: DnsResourceRecord = DnsResourceRecord::from_bytes(answer_as_bytes.iter().cloned()).unwrap();

        let domain_string = String::from_utf8(answer_final.name).unwrap();
        println!("{:?}", domain_string);
        println!("{:?}", answer_final.rec_type);
        println!("{:?}", answer_final.class);
        println!("{:?}", answer_final.ttl);
        println!("{:?}", answer_final.rdlength);
        println!("{:?}", answer_final.rdata);

        assert_eq!(domain_string, "google.com".to_string());
        assert_eq!(answer_final.rec_type, 1);
        assert_eq!(answer_final.class, 1);
        assert_eq!(answer_final.ttl, 1600);
        assert_eq!(answer_final.rdlength, rdata_len);
        assert_eq!(answer_final.rdata, rdata_vec.clone());
    }
}