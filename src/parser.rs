use std::{
    io::{self, BufRead, BufReader, Cursor, ErrorKind},
    ops::Index,
};

use crate::bytecode::{BlockType, Inst, LocalIdx, LabelIdx};

pub struct Parser {
    pub stream: Box<dyn BufRead>,
}

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

#[derive(Copy, Clone)]
pub struct TypeIdx(u32);

#[derive(Debug, Copy, Clone)]
pub struct FuncIdx(pub u32);
pub struct TableIdx(u32);
pub struct MemIdx(u32);
pub struct GlobalIdx(u32);

pub struct Locals {
    pub n: u32,
    pub t: ValType,
}

pub struct ExprBytes(pub Vec<u8>);

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
    reftype: Reftype,
    limits: Limits,
}

pub struct Mem {
    limits: Limits,
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
    init: Vec<u8>,
    mode: Datamode,
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
    pub module: String,
    pub nm: String,
    desc: ImportDesc,
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
    min: u32,
    max: Option<u32>,
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
    align: u32,
    offset: u32,
}

impl Parser {
    fn parse_magic(&mut self) -> Result<(), io::Error> {
        let mut magic = [0u8; 4];
        self.stream.read_exact(&mut magic)?;
        if magic != [0x00, 0x61, 0x73, 0x6d] {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid magic"));
        }
        Ok(())
    }

    fn parse_version(&mut self) -> Result<(), io::Error> {
        let mut magic = [0u8; 4];
        self.stream.read_exact(&mut magic)?;
        if magic != [0x01, 0x00, 0x00, 0x00] {
            return Err(io::Error::new(io::ErrorKind::Other, "invalid version"));
        }
        Ok(())
    }

    fn parse_byte(&mut self) -> Result<u8, io::Error> {
        let mut byte = [0];
        self.stream.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn read_bytes(&mut self, bytes: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0; bytes];
        self.stream.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn parse_u32(&mut self) -> Result<u32, io::Error> {
        let mut result: u32 = 0;
        let mut shift: u32 = 0;
        // 5 = 32/7 rounded up
        for _ in 0..5 {
            let byte = self.parse_byte()?;
            const HIGHMASK: u8 = 0b1000_0000;
            result |= ((byte & !HIGHMASK) as u32) << shift;
            if byte & HIGHMASK == 0 {
                break;
            }
            shift += 7;
        }
        Ok(result)
    }

    fn parse_section_header(&mut self) -> Result<(SectionId, u32), io::Error> {
        let typ = self.parse_byte()?;
        let id = SectionId::try_from(typ)
            .map_err(|_e| io::Error::new(ErrorKind::InvalidInput, "unknown section id"))?;
        let size = self.parse_u32()?;
        Ok((id, size))
    }

    fn parse_valtype(&mut self) -> Result<ValType, io::Error> {
        let typ = self.parse_byte()?;
        let typ = ValType::try_from(typ)
            .map_err(|_e| io::Error::new(ErrorKind::InvalidInput, "unknown value type"))?;
        Ok(typ)
    }

    fn parse_resulttype(&mut self) -> Result<ResultType, io::Error> {
        let elems = self.parse_u32()?;
        let mut vals = vec![];
        for _ in 0..elems {
            let val = self.parse_valtype()?;
            vals.push(val);
        }
        return Ok(ResultType { types: vals });
    }

    fn parse_functype(&mut self) -> Result<FuncType, io::Error> {
        let header = self.parse_byte()?;
        assert_eq!(header, 0x60);
        let from = self.parse_resulttype()?;
        let to = self.parse_resulttype()?;
        Ok(FuncType { from, to })
    }

    fn parse_typeidx(&mut self) -> Result<TypeIdx, io::Error> {
        let idx = self.parse_u32()?;
        Ok(TypeIdx(idx))
    }

    fn parse_funcidx(&mut self) -> Result<FuncIdx, io::Error> {
        let idx = self.parse_u32()?;
        Ok(FuncIdx(idx))
    }

    fn parse_name(&mut self) -> Result<String, io::Error> {
        let size = self.parse_u32()?;
        let bytes = self.read_bytes(size as usize)?;
        let name = String::from_utf8(bytes).expect("invalid utf8");
        Ok(name)
    }

    fn parse_export_desc(&mut self) -> Result<ExportDesc, io::Error> {
        let typ = self.parse_byte()?;
        let idx = self.parse_u32()?;
        let desc = match typ {
            0 => ExportDesc::Func(FuncIdx(idx)),
            1 => ExportDesc::Table(TableIdx(idx)),
            2 => ExportDesc::Mem(MemIdx(idx)),
            3 => ExportDesc::Global(GlobalIdx(idx)),
            _ => panic!("invalid export desc"),
        };
        Ok(desc)
    }

    fn parse_export(&mut self) -> Result<Export, io::Error> {
        let name = self.parse_name()?;
        let desc = self.parse_export_desc()?;
        Ok(Export { name, desc })
    }

    fn parse_local(&mut self) -> Result<Locals, io::Error> {
        let n = self.parse_u32()?;
        let t = self.parse_valtype()?;
        Ok(Locals { n, t })
    }

    fn parse_code(&mut self, func_types: &[TypeIdx]) -> Result<Vec<Func>, io::Error> {
        let elems = self.parse_u32()?;
        let mut funcs = vec![];
        for func in 0..elems {
            let typidx = func_types[func as usize];
            let size = self.parse_u32()?;
            let mut locals = vec![];
            let local_count = self.parse_u32()?;
            for _ in 0..local_count {
                locals.push(self.parse_local()?);
            }
            let expr = self.parse_expr()?;

            funcs.push(Func::Local {
                typ: typidx,
                locals,
                body: expr,
            });
        }
        Ok(funcs)
    }

    fn parse_import_desc(&mut self) -> Result<ImportDesc, io::Error> {
        let typ = self.parse_byte()?;
        match typ {
            0x00 => {
                let idx = self.parse_typeidx()?;
                Ok(ImportDesc::Func(idx))
            }
            0x01 => todo!(),
            0x02 => todo!(),
            0x03 => todo!(),
            _ => panic!("invalid import desc"),
        }
    }

    fn parse_reftype(&mut self) -> Result<Reftype, io::Error> {
        let byte = self.parse_byte()?;
        let typ = match byte {
            0x70 => Reftype::Funcref,
            0x6F => Reftype::Externref,
            _ => panic!("invalid reftype"),
        };
        Ok(typ)
    }

    fn parse_limits(&mut self) -> Result<Limits, io::Error> {
        let byte = self.parse_byte()?;
        let limits = match byte {
            0x00 => {
                let min = self.parse_u32()?;
                Limits { min, max: None }
            }
            0x01 => {
                let min = self.parse_u32()?;
                let max = self.parse_u32()?;
                Limits {
                    min,
                    max: Some(max),
                }
            }
            _ => panic!("invalid limits"),
        };
        Ok(limits)
    }

    fn parse_tabletype(&mut self) -> Result<Table, io::Error> {
        let reftype = self.parse_reftype()?;
        let limits = self.parse_limits()?;
        Ok(Table { reftype, limits })
    }

    fn parse_memtype(&mut self) -> Result<Mem, io::Error> {
        let limits = self.parse_limits()?;
        Ok(Mem { limits })
    }

    fn parse_elem(&mut self) -> Result<Elem, io::Error> {
        todo!()
    }

    fn parse_blocktype(&mut self) -> Result<BlockType, io::Error> {
        let typ = match self.peek_byte()? {
            0x40 => {
                self.stream.consume(1);
                BlockType::Empty
            }
            0x7F | 0x7E | 0x7D | 0x7C | 0x7B | 0x70 | 0x67 => {
                BlockType::Inline(self.parse_valtype()?)
            }
            _ => todo!(),
        };
        Ok(typ)
    }

    fn peek_byte(&mut self) -> Result<u8, io::Error> {
        Ok(self.stream.fill_buf()?[0])
    }

    fn parse_block(&mut self) -> Result<(BlockType, Vec<Inst>), io::Error> {
        let bt = self.parse_blocktype()?;
        let insts = self.parse_expr()?;
        Ok((bt, insts))
    }

    fn parse_if(&mut self) -> Result<(BlockType, Vec<Inst>, Vec<Inst>), io::Error> {
        let bt = self.parse_blocktype()?;
        let mut ifis = vec![];
        loop {
            match self.peek_byte()? {
                0x05 => {
                    break;
                }
                0x0b => todo!(),
                _ => panic!(),
            }
            ifis.push(self.parse_instr()?);
        }
        let mut elseis = self.parse_block()?;
        todo!()
    }

    // TODO: check if correct
    fn parse_i32(&mut self) -> Result<i32, io::Error> {
        let mut result: i32 = 0;
        let mut shift = 0;
        loop {
            let byte = self.parse_byte()?;
            result |= ((byte & 0x7f) as i32) << shift;
            shift += 7;
            if (0x80 & byte) == 0 {
                if shift < 32 && (byte & 0x40) != 0 {
                    return Ok(result | (!0 << shift));
                }
                return Ok(result);
            }
        }
    }

    fn parse_i64(&mut self) -> Result<i64, io::Error> {
        let mut result: i64 = 0;
        let mut shift = 0;
        loop {
            let byte = self.parse_byte()?;
            result |= ((byte & 0x7f) as i64) << shift;
            shift += 7;
            if (0x80 & byte) == 0 {
                if shift < 32 && (byte & 0x40) != 0 {
                    return Ok(result | (!0 << shift));
                }
                return Ok(result);
            }
        }
    }

    fn parse_memarg(&mut self) -> Result<MemArg, io::Error> {
        let align = self.parse_u32()?;
        let offset = self.parse_u32()?;
        Ok(MemArg { align, offset })
    }

    fn parse_labelidx(&mut self) -> Result<LabelIdx, io::Error> {
        Ok(LabelIdx(self.parse_u32()?))
    }

    fn parse_f64(&mut self) -> Result<f64, io::Error> {
        let mut bytes = [0u8; 8];
        self.stream.read_exact(&mut bytes)?;
        Ok(f64::from_le_bytes(bytes))
    }

    fn parse_instr(&mut self) -> Result<Inst, io::Error> {
        let byte = self.parse_byte()?;
        println!("0x{byte:x}");
        let inst = match byte {
            0x00 => Inst::Unreachable,
            0x01 => Inst::Nop,
            0x02 => {
                let (bt, i) = self.parse_block()?;
                Inst::Block(i)
            }
            0x03 => {
                let (bt, i) = self.parse_block()?;
                Inst::Loop(i)
            }
            0x04 => {
                let (bt, then, els) = self.parse_if()?;
                todo!()
            }
            0x0C => Inst::Break(self.parse_labelidx()?),
            0x0F => Inst::Return,
            0x10 => Inst::Call(self.parse_funcidx()?),
            0x0d => Inst::BreakIf(self.parse_labelidx()?),
            0x20 => Inst::LocalGet(self.parse_localidx()?),
            0x21 => Inst::LocalSet(self.parse_localidx()?),
            0x22 => Inst::LocalTee(self.parse_localidx()?),
            0x28 => Inst::I32Load(self.parse_memarg()?),
            0x29 => Inst::I64Load(self.parse_memarg()?),
            0x2d => Inst::I32Load8U(self.parse_memarg()?),
            0x2f => Inst::I32Load16U(self.parse_memarg()?),
            0x36 => Inst::I32Store(self.parse_memarg()?),
            0x37 => Inst::I64Store(self.parse_memarg()?),
            0x3a => Inst::I32Store8(self.parse_memarg()?),
            0x3b => Inst::I32Store16(self.parse_memarg()?),
            0x41 => Inst::I32Const(self.parse_i32()?),
            0x42 => Inst::I64Const(self.parse_i64()?),
            0x44 => Inst::F64Const(self.parse_f64()?),

            0x45 => Inst::I32Eqz,
            0x46 => Inst::I32Eq,
            0x47 => Inst::I32Ne,
            0x48 => Inst::I32LT_S,
            0x49 => Inst::I32LT_U,
            0x4c => Inst::I32GE_S,

            0x64 => Inst::F64Gt,
            0x6a => Inst::I32Add,
            0x6b => Inst::I32Sub,
            0x6c => Inst::I32Mul,
            0x78 => Inst::I32Rotr,
            0x74 => Inst::I32Shl,
            0x7c => Inst::I64Add,
            0x7e => Inst::I64Mul,
            0x84 => Inst::I64Or,
            0x85 => Inst::I64Xor,
            0x86 => Inst::I64Shl,
            0x88 => Inst::I64ShrU,

            0xa7 => Inst::I32WrapI64,
            0xad => Inst::I64ExtendI32U,
            x => panic!("unknown op: 0x{x:x?}"),
        };
        Ok(inst)
    }

    fn parse_expr(&mut self) -> Result<Vec<Inst>, io::Error> {
        let mut is = vec![];
        loop {
            match self.peek_byte()? {
                0x0B => {
                    self.stream.consume(1);
                    break;
                }
                _ => {}
            };
            is.push(self.parse_instr()?);
        }
        Ok(is)
    }

    fn parse_data(&mut self) -> Result<Data, io::Error> {
        let kind = self.parse_u32()?;
        let data = match kind {
            0 => {
                let expr = self.parse_expr()?;
                let byte_size = self.parse_u32()?;
                let bytes = self.read_bytes(byte_size as usize)?;
                Data {
                    init: bytes,
                    mode: Datamode::Active {
                        memory: MemIdx(0),
                        offset: expr,
                    },
                }
            }
            1 => {
                let count = self.parse_u32()?;
                let buf = self.read_bytes(count as usize)?;
                Data {
                    init: buf,
                    mode: Datamode::Passive,
                }
            }
            2 => {
                todo!("active data")
            }
            _ => panic!("invalid data kind"),
        };
        Ok(data)
    }

    pub fn parse_module(&mut self) -> Result<Module, io::Error> {
        let mut module = Module::default();
        let mut func_types = vec![];
        self.parse_magic()?;
        self.parse_version()?;

        while !self.stream.fill_buf()?.is_empty() {
            let (typ, size) = self.parse_section_header()?;

            match typ {
                SectionId::Custom => {
                    let mut content = vec![0u8; size as usize];
                    self.stream
                        .read_exact(&mut content)
                        .expect("failed to read section content");
                }
                SectionId::Type => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let functype = self.parse_functype()?;
                        module.types.push(functype);
                    }
                }
                SectionId::Import => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let nm_1 = self.parse_name()?;
                        let nm_2 = self.parse_name()?;
                        let desc = self.parse_import_desc()?;
                        let import = Import {
                            module: nm_1,
                            nm: nm_2,
                            desc,
                        };
                        module.imports.push(import.clone());
                        let idx = module.imports.len() - 1;
                        match import.desc {
                            ImportDesc::Func(typ) => {
                                module.funcs.push(Func::External { typ, import: idx })
                            }
                            ImportDesc::Table {} => todo!(),
                            ImportDesc::Mem {} => todo!(),
                            ImportDesc::Global {} => todo!(),
                        }
                    }
                }
                SectionId::Function => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let typidx = self.parse_typeidx()?;
                        func_types.push(typidx);
                    }
                }
                SectionId::Table => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let tabletyp = self.parse_tabletype()?;
                        module.tables.push(tabletyp)
                    }
                }
                SectionId::Memory => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let memtype = self.parse_memtype()?;
                        module.mems.push(memtype);
                    }
                }
                SectionId::Global => todo!(),
                SectionId::Export => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let export = self.parse_export()?;
                        module.exports.push(export);
                    }
                }
                SectionId::Start => {
                    let idx = self.parse_funcidx()?;
                    module.start = Some(idx)
                }
                SectionId::Element => {
                    let mut content = vec![0u8; size as usize];
                    self.stream
                        .read_exact(&mut content)
                        .expect("failed to read section content");
                    // TODO
                }
                SectionId::Code => {
                    module.funcs.extend(self.parse_code(&func_types)?);
                }
                SectionId::Data => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let data = self.parse_data()?;
                        module.datas.push(data)
                    }
                }
                SectionId::DataCount => todo!(),
            }
        }

        Ok(module)
    }

    fn parse_localidx(&mut self) -> Result<LocalIdx, io::Error> {
        Ok(LocalIdx(self.parse_u32()?))
    }
}

pub fn parse_stream(stream: Box<dyn BufRead>) -> Result<Module, io::Error> {
    let mut parser = Parser {
        stream: Box::new(stream),
    };
    let module = parser.parse_module()?;
    Ok(module)
}

pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<Module, io::Error> {
    let fd = std::fs::File::open(path.as_ref())?;
    parse_stream(Box::new(BufReader::new(fd)))
}

#[cfg(test)]
static EMPTY_MOD: &'static [u8] = include_bytes!("../examples/nothing.wasm");

#[cfg(test)]
static ADD_MOD: &'static [u8] = include_bytes!("../examples/add.wasm");

#[cfg(test)]
fn parse_bytes(bytes: &'static [u8]) -> io::Result<Module> {
    use std::io::BufReader;

    let reader = BufReader::new(bytes);
    let mut parser = Parser {
        stream: Box::new(reader),
    };
    let module = parser.parse_module()?;
    Ok(module)
}

#[cfg(test)]
#[test]
fn parse_empty() {
    parse_bytes(EMPTY_MOD).expect("could not parse empty module");
}

#[cfg(test)]
#[test]
fn parse_add() {
    parse_bytes(ADD_MOD).expect("could not parse add module");
}
