use std::{collections::BTreeMap, cell::RefCell, rc::Rc};

use crate::{repr::{Func, FuncType, Inst, TypeIdx, Module}, rt::Val};

pub enum FuncInst {
    Local { typ: FuncType, module: Rc<RefCell<ModuleInst>>, code: Func },
    External { typ: TypeIdx, func: Box<dyn WasmFfi> },
}

pub struct Store {
    pub funcs: Vec<Rc<FuncInst>>,
}

pub struct ModuleInst {
    types: Vec<FuncType>,
    pub func_addrs: Vec<usize>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Name {
    toplevel: String,
    secondLevel: String,
}

impl Name {
    pub fn new(top: &str, second: &str) -> Self {
        Self {
            toplevel: top.into(),
            secondLevel: second.into(),
        }
    }
}

pub trait WasmFfi {
    fn call(&self, store: &mut Store, args: &[Val]) -> Vec<Val>;
}

pub struct FFiFunc<F>(pub F);
impl<F> WasmFfi for FFiFunc<F> 
where F: Fn(&mut Store, &[Val]) -> Vec<Val>
{
    fn call(&self, store: &mut Store, args: &[Val]) -> Vec<Val> {
        self.0(store, args)
    }
}


pub enum ExternVal {
    ExternalFunc(Box<dyn WasmFfi>)
}

pub struct Externals {
    pub values: BTreeMap<Name, ExternVal>,
}

impl Externals {
    fn get_func(&mut self, name: Name) -> Option<Box<dyn WasmFfi>> {
        let v = self.values.remove(&name)?;
        match v {
            ExternVal::ExternalFunc(func) => Some(func),
            _ => None,
        }
    }
}

pub fn instantiate(module: &Module, store: &mut Store, mut externals: Externals) -> Rc<RefCell<ModuleInst>> {
    let inst = Rc::new(RefCell::new(ModuleInst {
        types: vec![],
        func_addrs: vec![],
    }));
    for import in &module.imports {
        println!("{:?}::{:?}", import.module, import.nm);
        match import.desc {
            crate::repr::ImportDesc::Func(t) => {

                let idx = store.funcs.len();
                let func = externals.get_func(Name::new(&import.module, &import.nm)).unwrap();
                store.funcs.push(Rc::new(FuncInst::External { typ: t, func }));
                inst.borrow_mut().func_addrs.push(idx);

            },
            crate::repr::ImportDesc::Table {  } => todo!(),
            crate::repr::ImportDesc::Mem {  } => todo!(),
            crate::repr::ImportDesc::Global {  } => todo!(),
        }
    }

    for func in &module.funcs {
        let typ = module[func.typ].clone();
        let idx = store.funcs.len();
        store.funcs.push(Rc::new(FuncInst::Local { typ, module: inst.clone(), code: func.clone() }));
        inst.borrow_mut().func_addrs.push(idx);
    }
    
    return inst;
}
