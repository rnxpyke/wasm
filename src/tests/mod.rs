use crate::scripts::run_script;
use crate::text::tokenize_script;

include!(concat!(env!("OUT_DIR"), "/wast_tests.rs"));
