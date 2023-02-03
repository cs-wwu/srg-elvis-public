use super::Segment;

#[derive(Debug, Clone)]
pub struct Transmit {
    pub segment: Segment,
    pub needs_transmit: bool,
}

impl Transmit {
    pub fn new(segment: Segment) -> Self {
        Self {
            segment,
            needs_transmit: true,
        }
    }
}
