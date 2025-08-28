#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unnecessary_transmutes)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
