use core::mem;

use crate::error::Error::CheckedMulFailed;
use crate::{error, SectorId};
use crate::{ClusterId, RawFatEntry};

pub fn get_params(
    sector_size: usize,
    fat_start_sector: SectorId,
    cluster_id: ClusterId,
) -> error::Result<(SectorId, usize)> {
    // this should be 512 / 32 = 18
    let fat_entries_per_sector = sector_size / mem::size_of::<RawFatEntry>();
    // In which sector is this cid contained. Cid: 222 / 18 = 12.3333
    //TODO: check floor
    let containing_sector = (f64::from(cluster_id) / fat_entries_per_sector as f64) as u32;
    // The sector is 12, now let's calculate the offset in that sector: 222 % 18 = 6.
    // TODO: check floor
    let offset_in_sector = ((f64::from(cluster_id) % fat_entries_per_sector as f64) as usize)
        .checked_mul(mem::size_of::<RawFatEntry>())
        .ok_or(CheckedMulFailed)?; //Todo: make nicer.

    let sector = SectorId(fat_start_sector + containing_sector);

    Ok((sector, offset_in_sector))
}
