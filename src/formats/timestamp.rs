use crate::defbit;

/// Tenths of a second. Range 0-199 inclusive,
/// as represented in FAT32 on-disk structures.
#[repr(transparent)]
#[derive(Default, Copy, Clone)]
pub struct Milliseconds(u8);

defbit!(
    VfatTimestamp,
    u32,
    [
        YEAR[31 - 25],
        MONTH[24 - 21],
        DAY[20 - 16],
        HOURS[15 - 11],
        MINUTES[10 - 5],
        SECONDS[4 - 0],
    ]
);
///
///15-11 Hours (0-23)
// 10-5 Minutes (0-59)
// 4-0 Seconds/2 (0-29)
impl VfatTimestamp {
    pub fn year(&self) -> u32 {
        self.get_value(Self::YEAR) + 1980_u32
    }
    pub fn month(&self) -> u32 {
        self.get_value(Self::MONTH)
    }
    pub fn day(&self) -> u32 {
        self.get_value(Self::DAY)
    }
    pub fn hour(&self) -> u32 {
        self.get_value(Self::HOURS)
    }
    pub fn minute(&self) -> u32 {
        self.get_value(Self::MINUTES)
    }
    /// Seconds are stored as number of 2-second intervals.
    /// Range: 0..29 29 represents 58 seconds
    pub fn second(&self) -> u32 {
        self.get_value(Self::SECONDS) * 2
    }
}
