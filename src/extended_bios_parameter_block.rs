use crate::const_assert_size;
use binrw::BinRead;

const BIOS_PARAMETER_BLOCK_SIZE: usize = 36;
const EXTENDED_BIOS_PARAMETER_BLOCK_SIZE: usize = 476;

// TODO: Impl debug.
#[derive(Debug, Copy, Clone, BinRead)]
pub struct BiosParameterBlock {
    /// These bytes are EB XX 90 -> JMP SHORT XX NOP
    _jump_instr: [u8; 3],
    oem_identifier: u64,
    /// Number of bytes per sector, in little-endian format
    pub bytes_per_sector: u16,
    /// Numbr of sectors per cluster:
    pub sectors_per_cluster: u8,
    /// Number of reserved sectors. Includes boot records.
    pub reserved_sectors: u16,
    /// Number of File Allocation Tables (FAT's) on the storage media. Often 2
    pub fat_amount: u8,
    max_num_directory_entries: u16,
    // Total logical sectors (if zero, use total_logical_sectors_gt_u16 field instead)
    total_logical_sectors: u16,
    fat_id: u8,
    /// Number of sectors per FAT. 0 for FAT32; use 32-bit value in extended bpb instead
    pub sectors_per_fat: u16,
    num_sectors_per_track: u16,
    num_heads_on_storage: u16,
    num_hiden_sectors: u32,
    /// Total logical sectors if greater than 65535; otherwise, see num_sectors_per_fat.
    pub total_logical_sectors_gt_u16: u32,
}
//const_assert_size!(BiosParameterBlock, 512 - EXTENDED_BIOS_PARAMETER_BLOCK_SIZE);

#[derive(Debug, Copy, Clone, BinRead)]
pub struct ExtendedBiosParameterBlock {
    pub sectors_per_fat: u32,
    flags: u16,
    fat_version: u16,
    /// Cluster of the root (`/`) directory
    pub root_cluster: u32,
    fsinfo_sector: u16,
    backup_boot_sector: u16,
    _reserved: [u8; 12],
    drive_number: u8,
    _reserved2: u8,
    signatore: u8,
    volumeid_serial_number: u32,
    pub volume_label_string: [u8; 11],
    system_identifier_string: [u8; 8],
    boot_code: [u8; 420],
    bootable_partition_sign: u16,
}

const_assert_size!(
    ExtendedBiosParameterBlock,
    EXTENDED_BIOS_PARAMETER_BLOCK_SIZE
);

#[derive(Debug, Clone, BinRead)]
pub struct FullExtendedBIOSParameterBlock {
    pub bpb: BiosParameterBlock,
    pub extended: ExtendedBiosParameterBlock,
}
impl FullExtendedBIOSParameterBlock {
    pub fn get_fat_size(&self) -> u32 {
        self.extended.sectors_per_fat * self.bpb.bytes_per_sector as u32
    }
    pub fn sectors_occupied_by_all_fats(&self) -> u32 {
        self.bpb.fat_amount as u32 * self.bpb.sectors_per_fat as u32
    }
}

//const_assert_size!(FullExtendedBIOSParameterBlock, 512);
