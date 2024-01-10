use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

use wasm::instance;
use wasm::instance::{instantiate, ExternVal, Externals, FFiFunc, Name, Store};
use wasm::rt::{Machine, Stack, Val};
use wasm::scripts::run_script;

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

fn rocket_externals() -> Externals {
    let mut vals = BTreeMap::new();
    let atan = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let clear_screen = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let cos = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let sin = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let draw_bullet = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let draw_enemy = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let draw_particle = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let draw_player = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    let draw_score = Box::new(FFiFunc(|_a: &mut Store, _vals: &[Val]| vec![]));
    vals.insert(Name::new("env", "Math_atan"), ExternVal::ExternalFunc(atan));
    vals.insert(
        Name::new("env", "clear_screen"),
        ExternVal::ExternalFunc(clear_screen),
    );
    vals.insert(Name::new("env", "cos"), ExternVal::ExternalFunc(cos));
    vals.insert(Name::new("env", "sin"), ExternVal::ExternalFunc(sin));
    vals.insert(
        Name::new("env", "draw_bullet"),
        ExternVal::ExternalFunc(draw_bullet),
    );
    vals.insert(
        Name::new("env", "draw_enemy"),
        ExternVal::ExternalFunc(draw_enemy),
    );
    vals.insert(
        Name::new("env", "draw_particle"),
        ExternVal::ExternalFunc(draw_particle),
    );
    vals.insert(
        Name::new("env", "draw_player"),
        ExternVal::ExternalFunc(draw_player),
    );
    vals.insert(
        Name::new("env", "draw_score"),
        ExternVal::ExternalFunc(draw_score),
    );
    Externals { values: vals }
}

fn rocket_example(path: &Path) {
    let add_mod = wasm::parser::parse_file(path).unwrap();
    let mut store = instance::Store {
        funcs: vec![],
        mems: vec![],
        tables: vec![],
    };

    let externals = rocket_externals();

    let instance = instantiate(&add_mod, &mut store, externals);
    let mut m = Machine {
        stack: Stack::default(),
        store: &mut store,
    };

    if let Some(start) = add_mod.start {
        let start_func_addr = instance.borrow().func_addrs[start.0 as usize];
        m.call(start_func_addr).unwrap();
    }
}

fn run_wast(path: &Path) {
    let script_str = std::fs::read_to_string(&path).unwrap();
    run_script(&script_str).unwrap();
}

fn main() {
    let args = Args::from_env();
    let ext = args
        .wasm
        .extension()
        .expect("file should have an extension")
        .to_str()
        .unwrap();

    if ext == "wast" {
        run_wast(&args.wasm);
        return;
    }

    if args.wasm.ends_with("rocket.wasm") {
        rocket_example(&args.wasm);
        return;
    }
}
