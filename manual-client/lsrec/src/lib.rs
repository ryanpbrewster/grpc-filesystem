extern "C" {
    fn hello() -> i32;
    // fn ls() -> String; // comma-separated list of files + directories
}

#[no_mangle]
pub extern fn entrypoint() -> i32 {
    unsafe { hello() * hello() }
}
