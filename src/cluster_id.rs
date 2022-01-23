use core::{fmt, ops};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ClusterId(pub(crate) u32);

impl ClusterId {
    pub(crate) fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn new(id: u32) -> Self {
        ClusterId(id)
    }

    pub fn into_high_low(self) -> (u16, u16) {
        ((self.0 >> 16) as u16, (self.0 & 0xFFFF) as u16)
    }

    pub fn from_high_low(high: u16, low: u16) -> Self {
        let raw_bytes: [u8; 4] = [
            (high >> 8) as u8,
            (high & 0xFF) as u8,
            (low >> 8) as u8,
            (low & 0xFF) as u8,
        ];
        ClusterId::new(u32::from_be_bytes(raw_bytes))
    }
}

impl fmt::Display for ClusterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> ops::Sub<T> for ClusterId
where
    T: Into<i64>,
{
    type Output = i64;

    fn sub(self, other: T) -> Self::Output {
        self.0 as i64 - other.into()
    }
}
