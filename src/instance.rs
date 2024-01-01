use std::{collections::BTreeMap, cell::RefCell, rc::Rc};

use crate::{repr::{Func, FuncType, Module, Datamode, TableType, TableIdx, MemType}, rt::{Val, Machine, Stack, Locals, self}};

pub enum FuncInst {
    Local { typ: FuncType, module: Rc<RefCell<ModuleInst>>, code: Func },
    External { typ: FuncType, func: Box<dyn WasmFfi> },
}

pub struct Store {
    pub funcs: Vec<Rc<FuncInst>>,
    pub mems: Vec<MemInstInner>,
    pub tables: Vec<TableInstInner>,
}


impl Store {
    fn allocfunc(&mut self, func: Func, moduleinst: Rc<RefCell<ModuleInst>>) -> FuncAddr {
        let addr = self.funcs.len();
        let functype = moduleinst.borrow().types[func.typ.0 as usize].clone();
        let funcinst = FuncInst::Local { code: func, typ: functype, module: moduleinst.clone() };
        self.funcs.push(Rc::new(funcinst));
        return FuncAddr(addr);
    }

    fn allochostfunc(&mut self, functype: FuncType, hostfunc: Box<dyn WasmFfi>) -> FuncAddr {
        let addr = self.funcs.len();
        let funcinst = FuncInst::External { typ: functype, func: hostfunc };
        self.funcs.push(Rc::new(funcinst));
        return FuncAddr(addr)
    }

    fn allocmem(&mut self, memtype: MemType) -> MemAddr {
        let addr = self.mems.len();
        let mem = MemInstInner::new(memtype.limits.min as usize * WASM_PAGE_SIZE);
        self.mems.push(mem);
        return MemAddr(addr)
    }

    fn alloctable(&mut self, tabletype: TableType, init: rt::Ref) -> TableAddr {
        let addr = self.tables.len();
        let n = tabletype.limits.min;
        let tableinst = TableInstInner { typ: tabletype, elem: vec![init; n as usize] };
        self.tables.push(tableinst);
        return TableAddr(addr);
    }
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

pub struct TableInstInner {
    typ: TableType,
    elem: Vec<rt::Ref>
}

pub struct ModuleInst {
    types: Vec<FuncType>,
    pub func_addrs: Vec<FuncAddr>,
    pub mem_addrs: Vec<MemAddr>,
    pub table_addrs: Vec<TableAddr>,
}

#[derive(Copy, Clone, Debug)]
pub struct FuncAddr(pub (crate) usize);

#[derive(Copy, Clone, Debug)]
pub struct TableAddr(pub (crate) usize);

#[derive(Copy, Clone, Debug)]
pub struct MemAddr(pub (crate) usize);

impl ModuleInst {
    pub (crate) fn table_addr(&self, idx: TableIdx) -> Option<TableAddr> {
        self.table_addrs.get(idx.0 as usize).copied()
    }
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
        table_addrs: vec![],
    }));
    for typ in &module.types {
        inst.borrow_mut().types.push(typ.clone());
    }
    for import in &module.imports {
        println!("{:?}::{:?}", import.module, import.nm);
        match import.desc {
            crate::repr::ImportDesc::Func(t) => {
                let functype = module.types[t.0 as usize].clone();
                let hostfunc = externals.get_func(Name::new(&import.module, &import.nm)).unwrap();
                let funcaddr = store.allochostfunc(functype, hostfunc);
                inst.borrow_mut().func_addrs.push(funcaddr);

            },
            crate::repr::ImportDesc::Table {  } => todo!(),
            crate::repr::ImportDesc::Mem {  } => todo!(),
            crate::repr::ImportDesc::Global {  } => todo!(),
        }
    }

    for func in &module.funcs {
        let funcaddr = store.allocfunc(func.clone(), inst.clone());
        inst.borrow_mut().func_addrs.push(funcaddr);
    }

    for table in &module.tables {
        let typ = table.reftype;
        let tableaddr = store.alloctable(table.clone(), rt::Ref::Null(typ));
        inst.borrow_mut().table_addrs.push(tableaddr);
    }
    
    for mem in &module.mems {
        let memaddr = store.allocmem(mem.clone());
        inst.borrow_mut().mem_addrs.push(memaddr);
    }

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
            let mem = &mut m.store.mems[inst.borrow().mem_addrs[0].0];
            mem.data[offset..offset+len].copy_from_slice(&data.init);
            println!("initialized data");
        }
    }
    return inst;
}
