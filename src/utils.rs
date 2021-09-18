use std::num::ParseIntError;

// Parses hex address starting with 0x
pub fn parse_address(s: &str) -> Result<u64, ParseIntError> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16)
}


pub fn parse_address_without_0x(s: &str) -> Result<u64, ParseIntError> {
    u64::from_str_radix(s, 16)
}
