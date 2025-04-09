pub mod interface;

pub const RUNTIME_WRAPPER_BINARY: &'static [u8] =
    include_bytes!("../../target/release/runtime-wrapper");
