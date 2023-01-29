use super::Dns;



/// A DNS header, as described in RFC1035 p25 s4.1.1
pub(super) struct DnsHeader {
    /// A 16 bit identifier assigned by the program that
    /// generates any kind of query.  This identifier is copied
    /// the corresponding reply and can be used by the requester
    ///  to match up replies to outstanding queries.
    pub id: u16,
    /// A one bit field that specifies whether this message is a
    /// query (0), or a response (1).
    pub qr: u8,
    /// A four bit field that specifies kind of query in this
    /// message.  This value is set by the originator of a query
    /// and copied into the response.  The values are:
    /// 0               a standard query (QUERY)
    /// 1               an inverse query (IQUERY)
    /// 2               a server status request (STATUS)
    pub opcode: u8,
    /// Authoritative Answer - this bit is valid in responses,
    /// and specifies that the responding name server is an
    /// authority for the domain name in question section.
    pub aa: u8,
    /// TrunCation - specifies that this message was truncated
    /// due to length greater than that permitted on the
    /// transmission channel.
    pub tc: u8,
    /// Recursion Desired - this bit may be set in a query and
    /// is copied into the response.  If RD is set, it directs
    /// the name server to pursue the query recursively.
    /// Recursive query support is optional.
    pub rd: u8,
    /// Recursion Available - this be is set or cleared in a
    /// response, and denotes whether recursive query support is
    /// available in the name server.
    pub ra: u8,
    /// Reserved for future use.  Must be zero in all queries
    /// and responses.
    pub z: u8,
    /// Response code - this 4 bit field is set as part of
    /// responses.  The values have the following
    /// interpretation:
    /// 0 - No error condition
    /// 1 Format error     - The name server was
    ///                      unable to interpret the query.
    /// 2 Server failure   - The name server was
    ///                      unable to process this query due to a
    ////                     problem with the name server.
    ///  3 Name Error      - Meaningful only for
    ///                      responses from an authoritative name
    ///                      server, this code signifies that the
    ///                      domain name referenced in the query does
    ///                      not exist.
    ///  4 Not Implemented - The name server does
    ///                      not support the requested kind of query.
    ///  5 Refused         - The name server refuses to
    ///                      perform the specified operation for
    ///                      policy reasons.  For example, a name
    ///                      server may not wish to provide the
    ///                      information to the particular requester,
    ///                      or a name server may not wish to perform
    ///                      a particular operation (e.g., zonetransfer) 
    ///                      for particular data.
    pub rcode: u8,
    /// an unsigned 16 bit integer specifying the number of
    /// entries in the question section.
    pub qdcount: u16,
    /// an unsigned 16 bit integer specifying the number of
    /// resource records in the answer section.
    pub ancount: u16,
    /// an unsigned 16 bit integer specifying the number of name
    /// server resource records in the authority records
    /// section.
    pub nscount: u16,
    /// an unsigned 16 bit integer specifying the number of
    /// resource records in the additional records section.
    pub arcount: u16,
    

}