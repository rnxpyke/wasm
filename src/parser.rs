use std::{io::{self, BufRead, ErrorKind, Cursor, BufReader}, ops::Index, slice::SliceIndex};

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
            x => return Err(UnkownValType(x))
        };
        Ok(typ)
    }
}

pub struct ResultType {
    pub types: Vec<ValType>
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

pub struct Func {
    pub typ: TypeIdx,
    pub locals: Vec<Locals>,
    pub body: Vec<u8>,
}

pub struct Table {}

pub struct Mem {}

pub struct Global {}

pub struct Elem {}

pub struct Data {}


pub struct Import {}

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
    DataCount = 12 
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
            x => return Err(UnkownSection(x))
        };
        Ok(id)
    }
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
        let id = SectionId::try_from(typ).map_err(|_e| io::Error::new(ErrorKind::InvalidInput, "unknown section id"))?;
        let size = self.parse_u32()?;
        Ok((id, size))
    }

    fn parse_valtype(&mut self) -> Result<ValType, io::Error> {
        let typ = self.parse_byte()?;
        let typ = ValType::try_from(typ).map_err(|_e| io::Error::new(ErrorKind::InvalidInput, "unknown value type"))?;
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
        let to  = self.parse_resulttype()?;
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
        let mut bytes = vec![0; size as usize];
        self.stream.read_exact(&mut bytes)?;
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
            _ => panic!("invalid export desc")
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
            let mut func_bytes = vec![0; size as usize];
            self.stream.read_exact(&mut func_bytes)?;
            let mut inner_parser = Parser { stream: Box::new(Cursor::new(func_bytes))};
            let mut locals = vec![];
            let local_count = inner_parser.parse_u32()?;
            for _ in 0..local_count {
                locals.push(inner_parser.parse_local()?);
            }
            let mut expr = vec![];
            inner_parser.stream.read_to_end(&mut expr)?;

            funcs.push(Func { typ: typidx, locals, body: expr });
        }
        Ok(funcs)
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
                    self.stream.read_exact(&mut content).expect("failed to read section content");
                },
                SectionId::Type => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let functype = self.parse_functype()?;
                        module.types.push(functype);
                    }
                },
                SectionId::Import => todo!(),
                SectionId::Function => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let typidx = self.parse_typeidx()?;
                        func_types.push(typidx);
                    }
                },
                SectionId::Table => todo!(),
                SectionId::Memory => todo!(),
                SectionId::Global => todo!(),
                SectionId::Export => {
                    let elems = self.parse_u32()?;
                    for _ in 0..elems {
                        let export = self.parse_export()?;
                        module.exports.push(export);
                    }
                },
                SectionId::Start => {
                    let idx = self.parse_funcidx()?;
                    module.start = Some(idx)

                },
                SectionId::Element => todo!(),
                SectionId::Code => {
                    module.funcs = self.parse_code(&func_types)?;
                },
                SectionId::Data => todo!(),
                SectionId::DataCount => todo!(),
            }
        }

        Ok(module)
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
