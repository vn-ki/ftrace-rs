use object::{read::ObjectSymbol, read::SymbolSection, Object, ObjectSection, SymbolKind};

#[derive(Debug)]
pub struct Function {
    pub address: u64,
    pub name: String,
}

pub fn get_functions<'a>(obj: &'a object::File) -> Vec<Function> {
    let text_section_idx = obj.section_by_name(".text").unwrap().index();
    let mut funcs = vec![];
    for symbol in obj.symbols() {
        if matches!(symbol.kind(), SymbolKind::Text) {
            match symbol.section() {
                SymbolSection::Section(idx) if idx == text_section_idx => {
                    funcs.push(Function {
                        address: symbol.address(),
                        // TODO: fix this
                        name: symbol.name().unwrap().into(),
                    });
                }
                _ => {}
            }
        }
    }
    funcs
}
