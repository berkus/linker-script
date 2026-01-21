// Example usage of the linker script pest grammar
//

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "linkrs.pest"]
pub struct LinkrsParser;

// Prototype AST types
#[derive(Debug)]
pub enum Item {
    Const(ConstDecl),
    MemoryMap(MemoryMap),
    ElfSegments(ElfSegments),
    Section(Section),
    Discard(Discard),
    ProvideSymbols(ProvideSymbols),
}

#[derive(Debug)]
pub struct ConstDecl {
    pub public: bool,
    pub name: String,
    pub type_ann: Option<String>,
    pub value: Expr,
}

#[derive(Debug)]
pub struct MemoryMap {
    pub regions: Vec<Region>,
}

#[derive(Debug)]
pub struct Region {
    pub name: String,
    pub permissions: Permissions,
    pub start: Expr,
    pub size: Expr,
}

#[derive(Debug, Default)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

#[derive(Debug)]
pub struct ElfSegments {
    pub segments: Vec<Segment>,
}

#[derive(Debug)]
pub struct Segment {
    pub name: String,
    pub segment_type: SegmentType,
    pub permissions: Permissions,
}

#[derive(Debug)]
pub enum SegmentType {
    Load,
    Dynamic,
    Interp,
    Note,
    Phdr,
    Tls,
    Null,
}

#[derive(Debug)]
pub struct Section {
    pub name: String,
    pub place_in: Option<String>,
    pub load_from: Option<String>,
    pub output_to: Option<String>,
    pub permissions: Option<Permissions>,
    pub occupies_file_space: Option<bool>,
    pub address: Option<AddressBlock>,
    pub file_position: Option<FilePosition>,
    pub contents: Option<Contents>,
    pub assertions: Vec<Assertion>,
    pub no_cross_refs: Vec<String>,
}

#[derive(Debug)]
pub struct AddressBlock {
    pub start: Option<Expr>,
    pub size: Option<Expr>,
    pub alignment: Option<Expr>,
    pub follows: Option<String>,
    pub virtual_base: Option<Expr>,
    pub region: Option<String>,
    pub load_from_region: Option<String>,
}

#[derive(Debug)]
pub struct FilePosition {
    pub start: FilePositionStart,
}

#[derive(Debug)]
pub enum FilePositionStart {
    Origin,
    Expr(Expr),
}

#[derive(Debug)]
pub struct Contents {
    pub items: Vec<ContentsItem>,
}

#[derive(Debug)]
pub enum ContentsItem {
    Symbol(SymbolDef),
    Input(InputStmt),
    Keep(InputStmt),
    AlignTo(Expr),
    AdvanceBy(Expr),
    FillPaddingWith(Expr),
    Cfg {
        predicate: CfgPredicate,
        item: Box<ContentsItem>,
    },
}

#[derive(Debug)]
pub struct SymbolDef {
    pub public: bool,
    pub name: String,
    pub value: LocationExpr,
}

#[derive(Debug)]
pub struct LocationExpr {
    pub accessor: Option<LocationAccessor>,
}

#[derive(Debug)]
pub enum LocationAccessor {
    Physical,
    Virtual,
}

#[derive(Debug)]
pub struct InputStmt {
    pub from: Option<String>, // glob pattern for file filter
    pub patterns: Vec<String>,
    pub sort_by: Option<SortKey>,
}

#[derive(Debug)]
pub enum SortKey {
    Name,
    Address,
    Alignment,
}

#[derive(Debug)]
pub enum CfgPredicate {
    Feature(String),
    Not(Box<CfgPredicate>),
    All(Vec<CfgPredicate>),
    Any(Vec<CfgPredicate>),
}

#[derive(Debug)]
pub struct Assertion {
    pub condition: Expr,
    pub message: String,
}

#[derive(Debug)]
pub struct Discard {
    pub patterns: Vec<InputStmt>,
}

#[derive(Debug)]
pub struct ProvideSymbols {
    pub symbols: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(u64),
    Ident(String),
    Here,
    Size,
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    UnaryMinus(Box<Expr>),
    Member {
        expr: Box<Expr>,
        field: String,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

// Simple test
#[cfg(test)]
mod tests {
    use {super::*, pest::Parser};

    #[test]
    fn test_parse_memory_map() {
        let input = r#"
memory_map {
    region FLASH {
        permissions: Read | Execute,
        start: 0x0800_0000,
        size: 256K,
    }
}
"#
        .trim();
        let result = LinkrsParser::parse(Rule::memory_map, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_section() {
        let input = r#"
section .text {
    place_in: FLASH,
    output_to: segment(flash),

    contents {
        input(.text*)
    }
}
"#
        .trim();
        let result = LinkrsParser::parse(Rule::section, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_const() {
        let input = "const PAGE_SIZE: usize = 64K;";
        let result = LinkrsParser::parse(Rule::const_decl, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_pub_const() {
        let input = "pub const KERNEL_VIRT_BASE: Address = 0xffff_ffff_0000_0000;";
        let result = LinkrsParser::parse(Rule::const_decl, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_section_with_address_block() {
        let input = r#"
section nucleus_text {
    permissions: Read | Execute,

    address {
        follows: init_thread_text,
        virtual_base: KERNEL_VIRT_BASE,
    }

    contents {
        input(.text*)
        align_to(2048);
        pub symbol __EXCEPTION_VECTORS_START = here();
        keep(input(.vectors))
    }

    assert_no_cross_references_to(init_thread_text, init_thread_stack);
}
"#
        .trim();
        let result = LinkrsParser::parse(Rule::section, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_provide_symbols() {
        let input = r#"
provide_symbols {
    current_el0_synchronous = current_el0_synchronous,
    current_el0_fiq = default_exception_handler,
}
"#
        .trim();
        let result = LinkrsParser::parse(Rule::provide_symbols, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_cfg_attr() {
        let input = r#"
contents {
    #[cfg(feature = "debug")]
    input(.debug_text*)
}
"#
        .trim();
        let result = LinkrsParser::parse(Rule::contents_block, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_assertion() {
        let input = r#"assert(size() < 64K, "text section too large");"#;
        let result = LinkrsParser::parse(Rule::assert_stmt, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_full_embedded() {
        let input = r#"
/// Define the physical memory regions available on this target
memory_map {
    region FLASH {
        permissions: Read | Execute,
        start: 0x0800_0000,
        size: 256K,
    }

    region RAM {
        permissions: Read | Write | Execute,
        start: 0x2000_0000,
        size: 64K,
    }
}

/// ELF program headers
elf_segments {
    segment flash {
        type: Load,
        permissions: Read | Execute,
    }

    segment ram {
        type: Load,
        permissions: Read | Write,
    }
}

section .text {
    place_in: FLASH,
    output_to: segment(flash),

    contents {
        input(.text*)
    }
}

discard {
    input(.comment)
    input(.note*)
}
"#;
        let result = LinkrsParser::parse(Rule::file, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }
}
