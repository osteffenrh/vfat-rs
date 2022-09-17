use crate::ClusterId;
use core::{fmt, mem};

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct RawFatEntry(u32);

impl RawFatEntry {
    pub fn as_buff(self) -> [u8; mem::size_of::<Self>()] {
        self.0.to_le_bytes()
    }

    pub fn get(&self) -> u32 {
        self.0
    }

    pub fn new_ref(buff: &[u8]) -> Self {
        let mut temp = [0u8; 4];
        temp.copy_from_slice(buff);
        Self::new(temp)
    }

    pub fn new(buff: [u8; mem::size_of::<u32>()]) -> Self {
        RawFatEntry(u32::from_le_bytes(buff))
    }
}
impl From<u32> for RawFatEntry {
    fn from(val: u32) -> Self {
        Self(val)
    }
}
impl fmt::Debug for RawFatEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawFatEntry(0x{:X})", self.0)
    }
}

/// A fat32 row entry. Each entry represents a cluster. This is the "high level" view
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FatEntry {
    /// Entry 0, formatted as 0xFFFFFFFN
    Id(u32),
    /// A free, unused cluster. 0x00
    Unused,
    /// 0x01: reserved
    Reserved(u32),
    /// A data cluster; value points to next cluster in chain.
    DataCluster(u32),
    /// Last cluster in chain. Should be, but may not be, the EndOfChainMarker (e.g. entry 1).
    LastCluster(u32),
}

impl FatEntry {
    pub(crate) fn from_chain(next: ClusterId) -> Self {
        Self::DataCluster(next.into())
    }
    pub fn as_buff(self) -> [u8; mem::size_of::<RawFatEntry>()] {
        let raw_fat: RawFatEntry = self.into();
        raw_fat.as_buff()
    }
}

impl From<RawFatEntry> for FatEntry {
    fn from(val: RawFatEntry) -> Self {
        use FatEntry::*;
        let lower_28_bits_mask: u32 = (1 << 28) - 1;
        // The upper 4 bits are ignored.
        let val = val.0 & lower_28_bits_mask;
        match val {
            0x0 => Unused,
            //0x1 => Reserved(val),
            0x0000002..=0xFFFFFEF => DataCluster(val),
            //0xFFFFFF0..=0xFFFFFF7 => Reserved(val),
            0xFFFFFF8..=0xFFFFFFF => LastCluster(val),
            val => Reserved(val),
        }
    }
}

impl From<FatEntry> for RawFatEntry {
    fn from(fat_entry: FatEntry) -> Self {
        use FatEntry::*;
        RawFatEntry(match fat_entry {
            Unused => 0x0,
            Reserved(i) => i,
            DataCluster(i) => i,
            LastCluster(i) => i,
            Id(i) => i,
        })
    }
}
