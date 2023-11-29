use std::ops::Index;


#[derive(Debug)]
#[repr(u8)]
pub enum ValType {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
    V128 = 0x7B,
    FuncRef = 0x70,
    ExternRef = 0x6F,
}

pub struct UnkownValType(u8);

impl TryFrom<u8> for ValType {
    type Error = UnkownValType;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let typ = match value {
            0x7F => ValType::I32,
            0x7E => ValType::I64,
            0x7D => ValType::F32,
            0x7C => ValType::F64,
            0x7B => ValType::V128,
            0x70 => ValType::FuncRef,
            0x6F => ValType::ExternRef,
            x => return Err(UnkownValType(x)),
        };
        Ok(typ)
    }
}

pub struct ResultType {
    pub types: Vec<ValType>,
}

pub struct FuncType {
    pub from: ResultType,
    pub to: ResultType,
}

#[derive(Debug, Copy, Clone)]
pub struct TypeIdx(pub (crate) u32);

#[derive(Debug, Copy, Clone)]
pub struct FuncIdx(pub (crate) u32);

#[derive(Debug, Copy, Clone)]
pub struct TableIdx(pub (crate) u32);

pub struct MemIdx(pub (crate) u32);

pub struct GlobalIdx(pub (crate) u32);

#[derive(Debug)]
pub struct Locals {
    pub n: u32,
    pub t: ValType,
}

pub struct ExprBytes(pub Vec<u8>);

#[derive(Debug)]
pub enum Func {
    Local {
        typ: TypeIdx,
        locals: Vec<Locals>,
        body: Vec<Inst>,
    },
    External {
        typ: TypeIdx,
        import: usize,
    },
}

impl Func {
    pub fn body(&self) -> Option<&Vec<Inst>> {
        match self {
            Func::Local { body, .. } => Some(&body),
            Func::External { .. } => None,
        }
    }

    pub fn typ(&self) -> TypeIdx {
        match self {
            Func::Local { typ, .. } => *typ,
            Func::External { typ, .. } => *typ,
        }
    }
}

pub struct Table {
    pub(crate) reftype: Reftype,
    pub(crate) limits: Limits,
}

pub struct Mem {
    pub(crate) limits: Limits,
}

pub struct Global {}

pub enum ElemMode {
    Passive,
    Active { table: TableIdx, offset: ExprBytes },
    Declarative,
}

pub struct Elem {
    typ: Reftype,
    init: Vec<ExprBytes>,
    mode: ElemMode,
}

pub enum Datamode {
    Passive,
    Active { memory: MemIdx, offset: Vec<Inst> },
}

pub struct Data {
    pub(crate) init: Vec<u8>,
    pub(crate) mode: Datamode,
}

#[derive(Clone)]
pub enum ImportDesc {
    Func(TypeIdx),
    Table {},
    Mem {},
    Global {},
}

#[derive(Clone)]
pub struct Import {
    pub(crate) module: String,
    pub(crate) nm: String,
    pub(crate) desc: ImportDesc,
}

pub enum ExportDesc {
    Func(FuncIdx),
    Table(TableIdx),
    Mem(MemIdx),
    Global(GlobalIdx),
}

pub struct Export {
    pub name: String,
    pub desc: ExportDesc,
}

pub enum Reftype {
    Funcref,
    Externref,
}

pub struct Limits {
    pub(crate) min: u32,
    pub(crate) max: Option<u32>,
}

#[derive(Default)]
pub struct Module {
    pub types: Vec<FuncType>,
    pub funcs: Vec<Func>,
    pub tables: Vec<Table>,
    pub mems: Vec<Mem>,
    pub globals: Vec<Global>,
    pub elems: Vec<Elem>,
    pub datas: Vec<Data>,
    pub start: Option<FuncIdx>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
}

impl Index<FuncIdx> for Module {
    type Output = Func;

    fn index(&self, index: FuncIdx) -> &Self::Output {
        &self.funcs[index.0 as usize]
    }
}

impl Index<TypeIdx> for Module {
    type Output = FuncType;

    fn index(&self, index: TypeIdx) -> &Self::Output {
        &self.types[index.0 as usize]
    }
}

#[repr(u8)]
pub enum SectionId {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
    DataCount = 12,
}

pub struct UnkownSection(u8);

impl TryFrom<u8> for SectionId {
    type Error = UnkownSection;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use SectionId::*;
        let id = match value {
            0 => Custom,
            1 => Type,
            2 => Import,
            3 => Function,
            4 => Table,
            5 => Memory,
            6 => Global,
            7 => Export,
            8 => Start,
            9 => Element,
            10 => Code,
            11 => Data,
            12 => DataCount,
            x => return Err(UnkownSection(x)),
        };
        Ok(id)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MemArg {
    pub(crate) align: u32,
    pub(crate) offset: u32,
}


#[derive(Debug)]
pub struct LabelIdx(pub (crate) u32);

#[derive(Debug, Copy, Clone)]
pub struct LocalIdx(pub(crate) u32);

pub enum BlockType {
    Empty,
    Inline(ValType),
    Type(TypeIdx),
}

#[derive(Debug)]
#[repr(u8)]
pub enum Inst {
    Unreachable = 0x00,
    Nop = 0x01,
    Block(Vec<Inst>) = 0x02,
    Loop(Vec<Inst>) = 0x03,
    IfElse(Vec<Inst>, Vec<Inst>) = 0x04,
    Break(LabelIdx) = 0x0C,
    BreakTable(Vec<LabelIdx>, LabelIdx),
    BreakIf(LabelIdx) = 0x0E,
    Return = 0x0F,
    Call(FuncIdx) = 0x10,
    CallIndirect(TypeIdx, TableIdx) = 0x11,
    LocalGet(LocalIdx),
    I32Add,
    F32Add,
    I32Const(i32),
    I64Const(i64),
    Drop,
    I32Load(MemArg),
    I32Sub,
    LocalTee(LocalIdx),
    I32Store(MemArg),
    LocalSet(LocalIdx),
    I32Eqz,
    I64Store(MemArg),
    F64Const(f64),
    I64Load(MemArg),
    I32Store8(MemArg),
    I32Load8U(MemArg),
    I32Load16U(MemArg),
    I32Store16(MemArg),
    F64Store(MemArg),
    I32Mul,
    I32GE_S,
    I32Shl,
    F64Gt,
    I64Or,
    I64Mul,
    I64Add,
    I64ShrU,
    I64Xor,
    I32WrapI64,
    I32Rotr,
    I32Eq,
    I32Ne,
    I32LT_S,
    I32LT_U,
    I64ExtendI32U,
    I64Shl,
    I64And,
    F64ReinterpretI64,
    F64Add,
    F64Sub,
    F64Mul,
    F64Abs,
    F64Neg,
    F64Div,
    F64Min,
    F64Max,
    F64Load(MemArg),
    F64ConvertI64U,
    Select,
    F64Le,
    F64Ge,
    F64Lt,
    F64Eq,
    F64Ne,
    I32And,
    I32Or,
    I32Xor,
    I32LE_U,
    I32GT_S,
    I32GT_U,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    I32Div_S,
    I32Div_U,
    I32Rem_S,
    I32Rem_U,
    I32LE_S,
    I32GE_U,
    I32Shr_S,
    I32Shr_U,
    I32Rotl,
    I64Load32U(MemArg),
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    F32Load(MemArg),
    I32Load8S(MemArg),
    I32Load16S(MemArg),
    I64Store8(MemArg),
    I64Store16(MemArg),
    I64Store32(MemArg),
    MemorySize,
    MemoryGrow,
}
