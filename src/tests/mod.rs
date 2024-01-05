use crate::wat::tokenize_script;
use crate::scripts::run_script;

include!(concat!(env!("OUT_DIR"), "/wast_tests.rs"));