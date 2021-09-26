use std::str::FromStr;

pub enum FuncSource {
    Dwarf,
    Heuristic,
}

impl FromStr for FuncSource {
    type Err = &'static str;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "dwarf" => Ok(Self::Dwarf),
            "heuristic" => Ok(Self::Heuristic),
            _ => Err("no such source"),
        }
    }
}
