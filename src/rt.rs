use std::{ops::{Index, self, IndexMut}, rc::Rc, cell::RefCell};

use crate::{repr::{LocalIdx, ResultType, Inst, self}, instance::{Store, ModuleInst, FuncInst}};


pub struct Locals {
    pub locals: Vec<Val>,
}

impl Locals {
    pub (crate) fn empty() -> Self {
        Self { locals: vec![] }
    }
}

impl Index<LocalIdx> for Locals {
    type Output = Val;

    fn index(&self, index: LocalIdx) -> &Self::Output {
        self.locals.index(index.0 as usize)
    }
}

impl IndexMut<LocalIdx> for Locals {
    fn index_mut(&mut self, index: LocalIdx) -> &mut Self::Output {
        self.locals.index_mut(index.0 as usize)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Val {
    I32(i32),
    F32(f32),
    I64(i64),
}

#[derive(Default, Debug)]
pub struct Stack {
    items: Vec<Val>,
}

impl Stack {
    pub (crate) fn new() -> Self {
        Self { items: vec![] }
    }
    fn push(&mut self, item: Val) {
        self.items.push(item);
    }

    pub (crate) fn pop(&mut self) -> Result<Val, Error> {
        self.items.pop().ok_or(Error::StackEmpty)
    }

    fn peek(&self) -> Result<Val, Error> {
        self.items.last().copied().ok_or(Error::StackEmpty)
    }
}

#[derive(Debug)]
pub enum Error {
    StackEmpty,
    SegFault,
    FunctionNotFound,
    LocalNotFound,
    WrongValType,
    OobAccess { addr: usize, len: usize },
}

impl From<Error> for Exception {
    fn from(value: Error) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Debug)]
pub enum Exception {
    Runtime(Error),
    Break(usize),
    Return,
}


pub struct Machine<'a> {
    pub stack: Stack,
    pub store: &'a mut Store,
}


fn binop_i32(stack: &mut Stack, op: impl FnOnce(i32, i32) -> i32) -> Result<(), Exception> {
    let Val::I32(c2) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let Val::I32(c1) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let res = op(c1, c2);
    stack.push(Val::I32(res));
    Ok(())
}

fn unop_i32(stack: &mut Stack, op: impl FnOnce(i32) -> i32) -> Result<(), Exception> {
    let Val::I32(val) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let res = op(val);
    stack.push(Val::I32(res));
    Ok(())
}

fn i32gt_u(a: i32, b: i32) -> i32 {
    let a = a as u32;
    let b = b as u32;
    return if a > b { 1 } else { 0 }
}

fn i32lt_u(a: i32, b: i32) -> i32 {
    let a = a as u32;
    let b = b as u32;
    return if a < b { 1 } else { 0 }
}

fn i32ge_u(a: i32, b: i32) -> i32 {
    let a = a as u32;
    let b = b as u32;
    return if a >= b { 1 } else { 0 }
}

fn i32le_u(a: i32, b: i32) -> i32 {
    let a = a as u32;
    let b = b as u32;
    return if a <= b { 1 } else { 0 }
}

fn i32shr_u(a: i32, b: i32) -> i32 {
    let a = a as u32;
    let b = b as u32;
    let res = a >> b;
    return res as i32;
}



impl Machine<'_> {
    pub fn call(&mut self, func_addr: usize) -> Result<(), Exception> {
        let func = self.store.funcs[func_addr].clone();
        match func.as_ref() {
            FuncInst::Local { typ, module, code } => {
                let mut locals = get_locals(&mut self.stack, &typ.from, &code.locals)?;
                match self.execute(module.clone(), &code.body, &mut locals) {
                    Ok(()) => {}
                    Err(Exception::Return) => {}
                    Err(e) => return Err(e),
                }
                // TODO: check stack return effect
            },
            FuncInst::External { typ, func } => {
                todo!()
            },
        }
        Ok(())
    }
    pub fn execute(
        &mut self,
        module: Rc<RefCell<ModuleInst>>,
        instructions: &[Inst],
        locals: &mut Locals,
    ) -> Result<(), Exception> {
        for inst in instructions {
            println!("{:?}", inst);
            match inst {
                Inst::Unreachable => todo!(),
                Inst::Nop => todo!(),
                Inst::Block(instructions) => {
                    match self.execute(module.clone(), &*instructions, locals) {
                        Ok(()) => {},
                        Err(Exception::Break(0)) => return Ok(()),
                        Err(Exception::Break(n)) => return Err(Exception::Break(n-1)),
                        Err(e) => return Err(e)
                    }
                },
                Inst::Loop(_) => todo!(),
                Inst::IfElse(_, _) => todo!(),
                Inst::Break(b) => return Err(Exception::Break(b.0 as usize)),
                Inst::BreakIf(b) => {
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    if c <= 0 {
                        return Err(Exception::Break(b.0 as usize));
                    }
                },
                Inst::Return => return Err(Exception::Return),
                Inst::Call(func) => {
                    let func_addr = module.borrow().func_addrs[func.0 as usize];
                    self.call(func_addr)?
                }
                Inst::Select => {
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let val2 = self.stack.pop()?;
                    let val1 = self.stack.pop()?;
                    if c != 0 {
                        self.stack.push(val1);
                    } {
                        self.stack.push(val2);
                    }
                }
                Inst::LocalGet(idx) => {
                    let local = locals[*idx];
                    self.stack.push(local);
                }
                Inst::LocalSet(idx) => {
                    let val = self.stack.pop()?;
                    locals[*idx] = val;
                }
                Inst::LocalTee(idx) => {
                    let val = self.stack.peek()?;
                    locals[*idx] = val;
                }
                Inst::I32Add => binop_i32(&mut self.stack, ops::Add::add)?,
                Inst::I32Sub => binop_i32(&mut self.stack, ops::Sub::sub)?,
                Inst::I32GT_U => binop_i32(&mut self.stack, i32gt_u)?,
                Inst::I32LT_U => binop_i32(&mut self.stack, i32lt_u)?,
                Inst::I32GE_U => binop_i32(&mut self.stack, i32ge_u)?,
                Inst::I32LE_U => binop_i32(&mut self.stack, i32le_u)?,
                Inst::I32And => binop_i32(&mut self.stack, ops::BitAnd::bitand)?,
                Inst::I32Shr_U => binop_i32(&mut self.stack, i32shr_u)?,
                Inst::I32Shl => binop_i32(&mut self.stack, ops::Shr::shr)?,
                Inst::I32Or => binop_i32(&mut self.stack, ops::BitOr::bitor)?,
                Inst::I32Xor => binop_i32(&mut self.stack, ops::BitXor::bitxor)?,
                Inst::I32Eq => binop_i32(&mut self.stack, |a, b| if a == b { 1 } else { 0 })?,
                Inst::I32Eqz => unop_i32(&mut self.stack, |b| if b == 0 { 1 } else { 0 })?,
                Inst::F32Add => todo!(),
                Inst::I32Const(v) => self.stack.push(Val::I32(*v)),
                Inst::I64Const(v) => self.stack.push(Val::I64(*v)),
                Inst::Drop => {
                    self.stack.pop()?;
                }
                Inst::I32Load(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr];
                    let Val::I32(i) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = i as usize + memarg.offset as usize;
                    const N: usize = 32;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let val = &mem.data[ea..ea+N/8];
                    let val = i32::from_le_bytes(val.try_into().unwrap());
                    self.stack.push(Val::I32(val))
                }
                Inst::I64Load(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr];
                    let Val::I32(i) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = i as usize + memarg.offset as usize;
                    const N: usize = 64;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let val = &mem.data[ea..ea+N/8];
                    let val = i64::from_le_bytes(val.try_into().unwrap());
                    self.stack.push(Val::I64(val))
                }
                Inst::I32Store(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr];
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let Val::I32(i) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = i as usize + memarg.offset as usize;
                    const N: usize = 32;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let bytes = c.to_le_bytes();
                    mem.data[ea..ea+N/8].copy_from_slice(&bytes);
                }
                Inst::I32Store8(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr];
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let Val::I32(i) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = i as usize + memarg.offset as usize;
                    const N: usize = 8;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let bytes = (c as i8).to_le_bytes();
                    mem.data[ea..ea+N/8].copy_from_slice(&bytes);   
                }
                Inst::I64Store(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr];
                    let Val::I64(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let Val::I32(i) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = i as usize + memarg.offset as usize;
                    const N: usize = 64;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let bytes = c.to_le_bytes();
                    mem.data[ea..ea+N/8].copy_from_slice(&bytes);
                }
                x => todo!("{:?}", x),
            }
        }
        Ok(())
    }
}


fn default_value(t: repr::ValType) -> Val {
    match t {
        repr::ValType::I32 => Val::I32(0),
        repr::ValType::I64 => Val::I64(0),
        repr::ValType::F32 => Val::F32(0.0),
        repr::ValType::F64 => todo!(),
        repr::ValType::V128 => todo!(),
        repr::ValType::FuncRef => todo!(),
        repr::ValType::ExternRef => todo!(),
    }
}


fn get_locals(stack: &mut Stack, from: &ResultType, locals: &[repr::Locals]) -> Result<Locals, Exception> {
    let mut vars = vec![];
    for param in from.types.iter() {
        println!("param: {param:?}");
        let arg = stack.pop()?;
        // TODO: assert type
        vars.push(arg);
    }
    vars.reverse();
    for extra in locals {
        for _ in 0..extra.n {
            vars.push(default_value(extra.t));    
        }
    }
    Ok(Locals { locals: vars })
}