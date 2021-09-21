use tracing::debug;

use crate::function::{
    FormalParameter, FormalParameterKind, Function, MemoryParam, ParamFindingFailure, Register,
};
use ddbug_parser::FileHash;

pub fn get_functions_dwarf(filename: &str) -> crate::defs::Result<Vec<Function>> {
    let mut funcs = Vec::new();
    ddbug_parser::File::parse(filename, |file| {
        let file_hash = FileHash::new(file);
        for unit in file.units() {
            for function in unit.functions() {
                if let Some(name) = function.name() {
                    let details = function.details(&file_hash);
                    let params = details
                        .parameters()
                        .into_iter()
                        .map(|p| parse_dwarf_param(p, &file_hash))
                        .collect::<Vec<_>>();
                    if let Some(address) = function.address() {
                        funcs.push(Function {
                            name: name.to_string(),
                            parameters: params,
                            address,
                        })
                    }
                }
            }
        }
        Ok(())
    })?;

    Ok(funcs)
}

fn parse_dwarf_param(
    param: &ddbug_parser::Parameter,
    file: &ddbug_parser::FileHash,
) -> std::result::Result<FormalParameter, ParamFindingFailure> {
    let regs: Vec<_> = param.registers().collect();
    if regs.len() != 0 {
        debug!(?regs);
        // TODO: len > 1: struct by value?
        return Ok(FormalParameter {
            name: param.name().map(|s| s.to_string()),
            kind: FormalParameterKind::Register(Register(regs[0].1 .0)),
        });
    }
    let fl: Vec<_> = param.frame_locations().collect();
    if fl.len() != 0 {
        debug!(?fl);
        // TODO: investigate when len > 1
        if let Some(size) = param.byte_size(file) {
            return Ok(FormalParameter {
                name: param.name().map(|s| s.to_string()),
                kind: FormalParameterKind::Memory(MemoryParam {
                    offset: fl[0].offset,
                    size,
                }),
            });
        }
        return Err(ParamFindingFailure::DwarfNoSize);
    }
    Err(ParamFindingFailure::DwarfNoFrameLocNoReg)
}
