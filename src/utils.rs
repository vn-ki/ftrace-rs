use std::num::ParseIntError;

use crate::defs::MemoryRegion;

// Parses hex address starting with 0x
pub fn parse_address(s: &str) -> Result<u64, ParseIntError> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16)
}

pub fn parse_address_without_0x(s: &str) -> Result<u64, ParseIntError> {
    u64::from_str_radix(s, 16)
}

pub fn get_base_region<'a>(vmmap: &'a [MemoryRegion], filename: &str) -> Option<&'a MemoryRegion> {
    vmmap
        .into_iter()
        .filter(|region| matches!(region.filename, Some(ref file) if file == filename))
        .min_by_key(|region| region.start)
}
