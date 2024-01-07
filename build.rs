use std::{fs, path::PathBuf};

#[derive(Debug)]
struct WastFile {
    name: String,
    path: PathBuf,
}

fn wast_files() -> Vec<WastFile> {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut wast_dir = PathBuf::from(root);
    wast_dir.push("spec/test/core");
    let entries = fs::read_dir(wast_dir).unwrap();

    let mut wast_files = vec![];
    for entry in entries {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_file() { continue; }
        let name = entry.file_name().into_string().unwrap();
        if let Some(name) = name.strip_suffix(".wast") {
            eprintln!("{:?}", entry.path());
            wast_files.push(WastFile { name: name.into(), path: entry.path() });
        }
    }
    return wast_files;
}

fn write_wast_tokenization_test(writer: &mut dyn std::io::Write, wast: &WastFile) {
    write!(writer, "
#[test]
fn tokenize_wast_{test_name}() {{
    let path = std::path::PathBuf::from(\"{filename}\");
    let content = std::fs::read_to_string(&path).unwrap();
    let _res = tokenize_script(&content).unwrap();
}}
"   , test_name = wast.name.replace("-", "_")
    , filename = wast.path.clone().into_os_string().into_string().unwrap()
    ).unwrap();
}

#[allow(dead_code)]
fn write_wast_script_test(writer: &mut dyn std::io::Write, wast: &WastFile) {
    write!(writer, "
    #[test]
    fn wast_script_{test_name}() {{
        let path = std::path::PathBuf::from(\"{filename}\");
        let content = std::fs::read_to_string(&path).unwrap();
        let _res = run_script(&content).unwrap();
    }}
    "   , test_name = wast.name.replace("-", "_")
        , filename = wast.path.clone().into_os_string().into_string().unwrap()
        ).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let destination = std::path::Path::new(&out_dir).join("wast_tests.rs");
    let mut f = std::fs::File::create(&destination).unwrap();
    let wast_files = wast_files();
    for wast in &wast_files {
        write_wast_tokenization_test(&mut f, &wast);
    }
    /*
    for wast in &wast_files {
        write_wast_script_test(&mut f, &wast);
    }
    */
}