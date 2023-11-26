use std::{
    ops::{self, Index},
    rc::Rc,
};

use wasm::{
    bytecode::{parse_instructions, Inst, LocalIdx},
    parser::{Func, Import, Module, ResultType},
};

#[derive(Copy, Clone, Debug)]
pub enum Val {
    I32(i32),
    F32(f32),
}

#[derive(Default)]
pub struct Stack {
    items: Vec<Val>,
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
    fn new(size: usize) -> Self {
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

pub trait ExternalFunc {
    fn call(&self, stack: &mut Stack, storage: &mut Storage) -> Result<(), Exception>;
}

#[derive(Default)]
pub struct Machine {
    pub stack: Stack,
    pub memory: Storage,
    pub external_funcs: Vec<(String, String, Rc<dyn ExternalFunc>)>,
}

pub struct Locals {
    locals: Vec<Val>,
}

impl Index<LocalIdx> for Locals {
    type Output = Val;

    fn index(&self, index: LocalIdx) -> &Self::Output {
        self.locals.index(index.0 as usize)
    }
}

fn binop_i32(stack: &mut Stack, op: impl FnOnce(i32, i32) -> i32) -> Result<(), Exception> {
    let Val::I32(left) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let Val::I32(right) = stack.pop()? else { return Err(Exception::Runtime(Error::WrongValType))};
    let res = op(left, right);
    stack.push(Val::I32(res));
    Ok(())
}

impl Machine {
    fn execute(
        &mut self,
        module: &Module,
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
                Inst::If(_) => todo!(),
                Inst::IfElse(_, _) => todo!(),
                Inst::Break(_) => todo!(),
                Inst::BreakIf(_) => todo!(),
                Inst::Return => todo!(),
                Inst::Call(func) => {
                    let func = &module[*func];
                    match func {
                        Func::Local { typ, locals, body } => {
                            let code = parse_instructions(body).unwrap();
                            let typ = &module[*typ];
                            let mut locals = get_locals(&mut self.stack, &typ.from)?;
                            match self.execute(module, &code, &mut locals) {
                                Ok(()) => {}
                                Err(Exception::Return) => {}
                                Err(e) => return Err(e),
                            }
                            // TODO: check stack return effect
                        }
                        Func::External { typ, import } => {
                            let import = &module.imports[*import];
                            let func = self.find_import_func(&import)?;
                            let Machine { stack, memory, .. } = self;
                            func.call(stack, memory)?;
                        }
                    }
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
            }
        }
        Ok(())
    }

    fn find_import_func(&self, import: &Import) -> Result<Rc<dyn ExternalFunc>, Error> {
        for (module, name, func) in self.external_funcs.iter() {
            if module == &import.module && name == &import.nm {
                return Ok(func.clone());
            }
        }
        Err(Error::FunctionNotFound)
    }
}

fn get_locals(stack: &mut Stack, from: &ResultType) -> Result<Locals, Exception> {
    let mut vars = vec![];
    for param in from.types.iter() {
        println!("param: {param:?}");
        let arg = stack.pop()?;
        // TODO: assert type
        vars.push(arg);
    }
    vars.reverse();
    Ok(Locals { locals: vars })
}

struct Greet;
impl ExternalFunc for Greet {
    fn call(&self, stack: &mut Stack, storage: &mut Storage) -> Result<(), Exception> {
        println!("greet");
        Ok(())
    }
}

fn main() {
    println!("Hello, world!");
    let add_mod = wasm::parser::parse_file("examples/greet.wasm").unwrap();

    let mut m = Machine {
        stack: Stack::default(),
        memory: Storage::new(65536),
        external_funcs: vec![("env".into(), "greet".into(), Rc::new(Greet))],
    };

    if let Some(start) = add_mod.start {
        let func = &add_mod[start];
        let code = parse_instructions(func.body().unwrap()).unwrap();
        let mut locals = Locals { locals: vec![] };
        m.execute(&add_mod, &code, &mut locals).unwrap();
    }
}
