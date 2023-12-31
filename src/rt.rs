use std::{ops::{Index, self}, rc::Rc, cell::RefCell};

use crate::{repr::{LocalIdx, ResultType, Import, Func, Inst, Module, self}, instance::{Store, ModuleInst, FuncInst}};


pub struct Locals {
    pub locals: Vec<Val>,
}

impl Index<LocalIdx> for Locals {
    type Output = Val;

    fn index(&self, index: LocalIdx) -> &Self::Output {
        self.locals.index(index.0 as usize)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Val {
    I32(i32),
    F32(f32),
}

#[derive(Default)]
pub struct Stack {
    items: Vec<Val>,
}

impl Stack {
    fn push(&mut self, item: Val) {
        self.items.push(item);
    }

    fn pop(&mut self) -> Result<Val, Error> {
        self.items.pop().ok_or(Error::StackEmpty)
    }
}

#[derive(Default)]
pub struct Storage {
    // TODO: change to byte array
    pub slots: Vec<Val>,
}

impl Storage {
    pub fn new(size: usize) -> Self {
        Storage {
            slots: vec![Val::I32(0); size],
        }
    }

    fn load(&mut self, addr: usize) -> Result<Val, Error> {
        return self.slots.get(addr).ok_or(Error::SegFault).cloned();
    }

    fn store(&mut self, addr: usize, val: Val) -> Result<(), Error> {
        *self.slots.get_mut(addr).ok_or(Error::SegFault)? = val;
        Ok(())
    }
}


#[derive(Debug)]
pub enum Error {
    StackEmpty,
    SegFault,
    FunctionNotFound,
    LocalNotFound,
    WrongValType,
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
    let Val::I32(left) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let Val::I32(right) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let res = op(left, right);
    stack.push(Val::I32(res));
    Ok(())
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
                Inst::Block(_) => todo!(),
                Inst::Loop(_) => todo!(),
                Inst::IfElse(_, _) => todo!(),
                Inst::Break(_) => todo!(),
                Inst::BreakIf(_) => todo!(),
                Inst::Return => todo!(),
                Inst::Call(func) => {
                    let func_addr = module.borrow().func_addrs[func.0 as usize];
                    self.call(func_addr)?
                }
                Inst::LocalGet(idx) => {
                    let local = locals[*idx];
                    self.stack.push(local);
                }
                Inst::I32Add => binop_i32(&mut self.stack, ops::Add::add)?,
                Inst::F32Add => todo!(),
                Inst::I32Const(v) => self.stack.push(Val::I32(*v)),
                Inst::Drop => {
                    self.stack.pop()?;
                }
                _=> todo!(),
            }
        }
        Ok(())
    }
}


fn default_value(t: repr::ValType) -> Val {
    match t {
        repr::ValType::I32 => Val::I32(0),
        repr::ValType::I64 => todo!(),
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