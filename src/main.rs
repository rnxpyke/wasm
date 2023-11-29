use std::{
    rc::Rc, path::PathBuf,
};

use wasm::rt::{ExternalFunc, Stack, Storage, Exception, Machine, self};


struct Greet;
impl ExternalFunc for Greet {
    fn call(&self, _stack: &mut Stack, _storage: &mut Storage) -> Result<(), Exception> {
        println!("greet");
        Ok(())
    }
}


pub struct Args {
    wasm: PathBuf,
}

impl Args {
    fn from_env() -> Self {
        let env = std::env::args();
        let mut wasm = None;
        let mut count = 0;
        for arg in env.skip(1) {
            if count == 0 {
                wasm = Some(PathBuf::from(arg));
            }
            count += 1;
        }
        let Some(wasm) = wasm else { panic!("no file") };
        Self { wasm }
    }
}

fn main() {
    println!("Hello, world!");
    let args = Args::from_env();
    let add_mod = wasm::parser::parse_file(&args.wasm).unwrap();

    let mut m = Machine {
        stack: Stack::default(),
        memory: Storage::new(65536),
        external_funcs: vec![("env".into(), "greet".into(), Rc::new(Greet))],
    };

    println!("running");
    if let Some(start) = add_mod.start {
        let func = &add_mod[start];
        println!("startfunc: {:?}", func);
        let mut locals = rt::Locals { locals: vec![] };
        m.execute(&add_mod, &func.body().unwrap(), &mut locals).unwrap();
    }
}
