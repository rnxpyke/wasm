use std::ops::Index;

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Clone)]
pub struct ResultType {
    pub types: Vec<ValType>,
}

#[derive(Debug, Clone)]
pub struct FuncType {
    pub from: ResultType,
    pub to: ResultType,
}

#[derive(Debug, Copy, Clone)]
pub struct TypeIdx(pub(crate) u32);

#[derive(Debug, Copy, Clone)]
pub struct FuncIdx(pub u32);

#[derive(Debug, Copy, Clone)]
pub struct TableIdx(pub(crate) u32);

pub struct MemIdx(pub(crate) u32);

pub struct GlobalIdx(pub(crate) u32);

#[derive(Debug, Clone)]
pub struct Locals {
    pub n: u32,
    pub t: ValType,
}

pub struct ExprBytes(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct Func {
    pub typ: TypeIdx,
    pub locals: Vec<Locals>,
    pub body: Vec<Inst>,
}

#[derive(Copy, Clone)]
pub struct TableType {
    pub(crate) reftype: Reftype,
    pub(crate) limits: Limits,
}

#[derive(Copy, Clone)]
pub struct MemType {
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

#[derive(Clone, Copy, Debug)]
pub enum Reftype {
    Funcref,
    Externref,
}

#[derive(Copy, Clone)]
pub struct Limits {
    pub(crate) min: u32,
    pub(crate) max: Option<u32>,
}

#[derive(Default)]
pub struct Module {
    pub types: Vec<FuncType>,
    pub funcs: Vec<Func>,
    pub tables: Vec<TableType>,
    pub mems: Vec<MemType>,
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

#[derive(Debug, Clone)]
pub struct LabelIdx(pub(crate) u32);

#[derive(Debug, Copy, Clone)]
pub struct LocalIdx(pub(crate) u32);

pub enum BlockType {
    Empty,
    Inline(ValType),
    Type(TypeIdx),
}

#[derive(Clone)]
pub struct Expr {
    pub instructions: Vec<Inst>,
}

impl From<Vec<Inst>> for Expr {
    fn from(value: Vec<Inst>) -> Self {
        Self {
            instructions: value,
        }
    }
}

impl AsRef<[Inst]> for Expr {
    fn as_ref(&self) -> &[Inst] {
        &self.instructions
    }
}

impl core::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Expr").finish()
    }
}

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum Inst {
    /// Control Instructions
    Unreachable = 0x00,
    Nop = 0x01,
    Block(Expr) = 0x02,
    Loop(Expr) = 0x03,
    IfElse(Expr, Expr) = 0x04,
    Break(LabelIdx) = 0x0C,
    BreakIf(LabelIdx) = 0x0D,
    BreakTable(Vec<LabelIdx>, LabelIdx),
    Return = 0x0F,
    Call(FuncIdx) = 0x10,
    CallIndirect(TypeIdx, TableIdx) = 0x11,

    /// Parametric Instructions
    Drop,
    Select,

    /// Variable Instructions
    LocalGet(LocalIdx),
    LocalSet(LocalIdx),
    LocalTee(LocalIdx),

    /// Memory instructions
    I32Load(MemArg),
    I64Load(MemArg),
    I32Store(MemArg),
    I32Store8(MemArg),
    I32Load8U(MemArg),
    I32Load16U(MemArg),
    I32Store16(MemArg),
    I64Store(MemArg),
    F64Store(MemArg),
    F64Load(MemArg),
    F32Load(MemArg),
    I32Load8S(MemArg),
    I32Load16S(MemArg),
    I64Store8(MemArg),
    I64Store16(MemArg),
    I64Store32(MemArg),
    I64Load32U(MemArg),
    MemorySize,
    MemoryGrow,

    /// Numeric const instructions
    I32Const(i32),
    I64Const(i64),
    F64Const(f64),

    /// Numeric instructions
    /// 1. I32 compare
    I32Eqz,
    I32Eq,
    I32Ne,
    I32GeS,
    I32LtS,
    I32LtU,
    I32LeU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32GeU,

    /// 2. I64 compare
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,

    /// 3. F32 compare

    /// 4. F64 compare
    F64Eq,
    F64Ne,
    F64Le,
    F64Ge,
    F64Lt,
    F64Gt,

    /// 5. I32 math
    I32Clz,
    I32Ctz,
    I32Add,
    I32Sub,
    I32Mul,
    I32And,
    I32Or,
    I32Xor,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Popcnt,
    I32Shl,
    I32Rotr,

    /// 6. I64 math
    I64Mul,
    I64Add,
    I64Or,
    I64ShrU,
    I64Xor,
    I64Shl,
    I64And,

    /// 7. F32 math
    F32Add,

    /// 8. F64 math
    F64Add,
    F64Sub,
    F64Mul,
    F64Abs,
    F64Neg,
    F64Div,
    F64Min,
    F64Max,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,

    /// 9. convert
    I32WrapI64,
    F64ReinterpretI64,
    F64ConvertI64U,
    I64ExtendI32U,
}
