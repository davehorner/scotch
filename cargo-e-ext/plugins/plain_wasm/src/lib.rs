    // A generic wasm export plugin: two simple exported functions
    #[no_mangle]
    pub extern "C" fn alpha() -> i32 {
        1
   }
    
    #[no_mangle]
    pub extern "C" fn beta() -> i32 {
        2
    }
