use std::{ops::{Index, self, IndexMut}, rc::Rc, cell::RefCell, sync::atomic::AtomicUsize};

use crate::{repr::{LocalIdx, ResultType, Inst, self, MemArg}, instance::{Store, ModuleInst, FuncInst, FuncAddr}};


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
    Reference(Ref),
}

#[derive(Copy, Clone, Debug)]
pub enum Ref {
    Null(repr::Reftype),
    Func(usize),
    Extern(usize),
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
        println!("\tpush: {:?}", item);
        self.items.push(item);
    }

    pub (crate) fn pop(&mut self) -> Result<Val, Error> {
        let val = self.items.pop().ok_or(Error::StackEmpty)?;
        println!("\tpop: {:?}", val);
        return Ok(val);
    }

    fn peek(&self) -> Result<Val, Error> {
        let val = self.items.last().copied().ok_or(Error::StackEmpty)?;
        println!("\tpeeked: {:?}", val);
        return Ok(val);
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
    InvalidAlignment,
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
    println!("\t{:?} {:?} -> {:?}", c1, c2, res);
    stack.push(Val::I32(res));
    Ok(())
}

fn unop_i32(stack: &mut Stack, op: impl FnOnce(i32) -> i32) -> Result<(), Exception> {
    let Val::I32(val) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let res = op(val);
    println!("\t{:?} -> {:?}", val, res);
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


fn effective_address(stack: &mut Stack, memarg: MemArg) -> Result<usize, Exception> {
    let Val::I32(i) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let ea = i as usize + memarg.offset as usize;
    println!("\tea: 0x{:0x?}", ea);
    if memarg.align != 0 {
        let is_aligned = ea & ((1 << (memarg.align - 1)) - 1) == 0;
        if !is_aligned {
            return Err(Exception::Runtime(Error::InvalidAlignment));
        }
    }
    return Ok(ea)
}


impl Machine<'_> {
    pub fn call(&mut self, func_addr: FuncAddr) -> Result<(), Exception> {
        let func = self.store.funcs[func_addr.0].clone();
        match func.as_ref() {
            FuncInst::Local { typ, module, code } => {
                let mut locals = get_locals(&mut self.stack, &typ.from, &code.locals)?;
                match self.execute(module.clone(), &code.body, &mut locals) {
                    Ok(()) => {}
                    Err(Exception::Return) => {}
                    Err(Exception::Break(_n)) => panic!("can't break through function"),
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
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        for inst in instructions {
            println!("{}: {:?}", COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst), inst);
            match inst {
                Inst::Unreachable => panic!("reached unreachable"),
                Inst::Nop => todo!(),
                Inst::Block(instructions) => {
                    match self.execute(module.clone(), instructions.as_ref(), locals) {
                        Ok(()) => {},
                        Err(Exception::Break(0)) => return Ok(()),
                        Err(Exception::Break(n)) => return Err(Exception::Break(n-1)),
                        Err(e) => return Err(e)
                    }
                },
                Inst::Loop(instructions) => {
                    loop {
                        match self.execute(module.clone(), instructions.as_ref(), locals) {
                            Ok(()) => break,
                            Err(Exception::Break(0)) => continue,
                            Err(Exception::Break(n)) => return Err(Exception::Break(n-1)),
                            Err(e) => return Err(e)
                        }
                    }
                },
                Inst::IfElse(_, _) => todo!(),
                Inst::Break(b) => return Err(Exception::Break(b.0 as usize)),
                Inst::BreakIf(b) => {
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    if c != 0 {
                        println!("\tbreaking");
                        return Err(Exception::Break(b.0 as usize));
                    }
                },
                Inst::Return => return Err(Exception::Return),
                Inst::Call(func) => {
                    let func_addr = module.borrow().func_addrs[func.0 as usize];
                    self.call(func_addr)?
                }
                Inst::CallIndirect(typidx, tableidx) => {
                    todo!();
                }
                Inst::Select => {
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let val2 = self.stack.pop()?;
                    let val1 = self.stack.pop()?;
                    if c != 0 {
                        self.stack.push(val1);
                    } else {
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
                Inst::I32GtU => binop_i32(&mut self.stack, i32gt_u)?,
                Inst::I32LtU => binop_i32(&mut self.stack, i32lt_u)?,
                Inst::I32GeU => binop_i32(&mut self.stack, i32ge_u)?,
                Inst::I32LeU => binop_i32(&mut self.stack, i32le_u)?,
                Inst::I32And => binop_i32(&mut self.stack, ops::BitAnd::bitand)?,
                Inst::I32ShrU => binop_i32(&mut self.stack, i32shr_u)?,
                Inst::I32Shl => binop_i32(&mut self.stack, ops::Shl::shl)?,
                Inst::I32Or => binop_i32(&mut self.stack, ops::BitOr::bitor)?,
                Inst::I32Xor => binop_i32(&mut self.stack, ops::BitXor::bitxor)?,
                Inst::I32Rotl => binop_i32(&mut self.stack, |a,b| a.rotate_left(b as u32))?,
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
                    let mem = &mut self.store.mems[mem_addr.0];
                    let ea = effective_address(&mut self.stack, *memarg)?;
                    const N: usize = 32;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let val = &mem.data[ea..ea+N/8];
                    let val = i32::from_le_bytes(val.try_into().unwrap());
                    self.stack.push(Val::I32(val))
                }
                Inst::I32Load8U(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr.0];
                    let ea = effective_address(&mut self.stack, *memarg)?;
                    const N: usize = 8;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let val = &mem.data[ea..ea+N/8];
                    let val = u8::from_le_bytes(val.try_into().unwrap());
                    self.stack.push(Val::I32(val as i32))
                }
                Inst::I64Load(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr.0];
                    let ea = effective_address(&mut self.stack, *memarg)?;
                    const N: usize = 64;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let val = &mem.data[ea..ea+N/8];
                    let val = i64::from_le_bytes(val.try_into().unwrap());
                    self.stack.push(Val::I64(val))
                }
                Inst::I32Store(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr.0];
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = effective_address(&mut self.stack, *memarg)?;
                    const N: usize = 32;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let bytes = c.to_le_bytes();
                    mem.data[ea..ea+N/8].copy_from_slice(&bytes);
                }
                Inst::I32Store8(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr.0];
                    let Val::I32(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = effective_address(&mut self.stack, *memarg)?;
                    const N: usize = 8;
                    if ea + N/8 > mem.len() { return Err(Exception::Runtime(Error::OobAccess { addr: ea, len: N/8 })) }
                    let bytes = (c as u8).to_le_bytes();
                    mem.data[ea..ea+N/8].copy_from_slice(&bytes);   
                }
                Inst::I64Store(memarg) => {
                    let mem_addr = module.borrow().mem_addrs[0];
                    let mem = &mut self.store.mems[mem_addr.0];
                    let Val::I64(c) = self.stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
                    let ea = effective_address(&mut self.stack, *memarg)?;
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
        println!("\tparam: {param:?}");
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