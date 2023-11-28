use std::io::{self, Cursor, Read};

use crate::parser::{FuncIdx, TypeIdx, ValType, ExprBytes, MemArg};

#[derive(Debug)]
pub struct LabelIdx(pub u32);

#[derive(Debug, Copy, Clone)]
pub struct LocalIdx(pub u32);

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
    BreakIf(LabelIdx) = 0x0E,
    Return = 0x0F,
    Call(FuncIdx) = 0x10,
    LocalGet(LocalIdx) = 0x11,
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
}

pub struct InstructionParser<'a> {
    bytes: Cursor<&'a [u8]>,
}

impl<'a> InstructionParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes: Cursor::new(bytes),
        }
    }

    fn parse_byte(&mut self) -> Result<u8, io::Error> {
        let mut byte = [0];
        self.bytes.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn parse_opcode(&mut self) -> Option<u8> {
        self.parse_byte().ok()
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

    fn parse_funcidx(&mut self) -> Result<FuncIdx, io::Error> {
        let idx = self.parse_u32()?;
        Ok(FuncIdx(idx))
    }

    fn parse_localidx(&mut self) -> Result<LocalIdx, io::Error> {
        let idx = self.parse_u32()?;
        Ok(LocalIdx(idx))
    }
}

pub fn parse_instructions(bytes: &ExprBytes) -> Result<Vec<Inst>, io::Error> {
    let mut parser = InstructionParser::new(&bytes.0);
    let mut is = vec![];
    while let Some(op) = parser.parse_opcode() {
        let inst = match op {
            0x00 => Inst::Unreachable,
            0x01 => Inst::Nop,
            0x10 => Inst::Call(parser.parse_funcidx()?),
            0x1a => Inst::Drop,
            0x20 => Inst::LocalGet(parser.parse_localidx()?),
            0x41 => Inst::I32Const(parser.parse_i32()?),
            0x6a => Inst::I32Add,
            0x92 => Inst::F32Add,
            0x0B => break,

            x => panic!("unknown opcode {x:x}"),
        };
        is.push(inst);
    }
    Ok(is)
}
