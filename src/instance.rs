use std::{collections::BTreeMap, cell::RefCell, rc::Rc};

use crate::{repr::{Func, FuncType, TypeIdx, Module, Datamode}, rt::{Val, Machine, Stack, Locals}};

pub enum FuncInst {
    Local { typ: FuncType, module: Rc<RefCell<ModuleInst>>, code: Func },
    External { typ: TypeIdx, func: Box<dyn WasmFfi> },
}

pub struct Store {
    pub funcs: Vec<Rc<FuncInst>>,
    pub mems: Vec<MemInstInner>
}


pub const WASM_PAGE_SIZE: usize = 65536;

pub struct MemInstInner {
    pub data: Vec<u8>
}

impl MemInstInner {
    fn new(bytes: usize) -> Self {
        Self { data: vec![0u8; bytes] }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct ModuleInst {
    types: Vec<FuncType>,
    pub func_addrs: Vec<usize>,
    pub mem_addrs: Vec<usize>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Name {
    toplevel: String,
    secondlevel: String,
}

impl Name {
    pub fn new(top: &str, second: &str) -> Self {
        Self {
            toplevel: top.into(),
            secondlevel: second.into(),
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
        }
    }
}

pub fn instantiate(module: &Module, store: &mut Store, mut externals: Externals) -> Rc<RefCell<ModuleInst>> {
    let inst = Rc::new(RefCell::new(ModuleInst {
        types: vec![],
        func_addrs: vec![],
        mem_addrs: vec![],
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
    
    let mem_addr = store.mems.len();
    store.mems.push(MemInstInner::new(WASM_PAGE_SIZE * 100));
    inst.borrow_mut().mem_addrs.push(mem_addr);

    for data in &module.datas {
        if let Datamode::Active { memory, offset } = &data.mode {
            assert!(memory.0 == 0);
            // TODO: this whole thing is entirely not to spec: improve
            let mut m = Machine { stack: Stack::new(), store };
            m.execute(inst.clone(), &offset, &mut Locals::empty() ).unwrap();
            println!("{:?}", m.stack);
            let Val::I32(offset) = m.stack.pop().unwrap() else { panic!() };
            let offset = offset as usize;
            let len = data.init.len();
            let mem = &mut m.store.mems[inst.borrow().mem_addrs[0] as usize];
            mem.data[offset..offset+len].copy_from_slice(&data.init);
            println!("initialized data");
        }
    }
    return inst;
}
