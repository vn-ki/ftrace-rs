#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("*nix error")]
    Nix(#[from] nix::Error),

    #[error("I/O error")]
    IO(#[from] std::io::Error),

    #[error("Object file parse error")]
    ObjectFile(#[from] object::Error),

    #[error("gmili DWARF error")]
    Gimli(#[from] gimli::Error),

    #[error("ddbug DWARF error")]
    Ddbug(#[from] ddbug_parser::Error),
}

#[derive(Debug)]
pub enum ParamFindingFailure {
    DwarfNoSize,
    DwarfNoFrameLocNoReg,
}
