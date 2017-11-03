# redif-rs
Redis protocol server Framework in Rust

Redif is a framework, it talks the data transport in redis protocol,
and call user provided Handler to handle the request.
User should implement Handler trait, and invoke Redif with redif::run( port, handler).

For example 

```rust

extern crate redif;

use std::sync::{Arc,Mutex};
use redif::{Value, Handler};

// implement Handler trait
struct Store {
    kv : HashMap<String, String>,
}

impl Handler for Store {

    fn handle(&mut self, data: &Value) -> Option<Value> {
		/// ...
	}

}


fn main() {

    let port = 4344u16;
    let store = Store::new();
    let handler = Arc::new(Mutex::new(store));

    // call redif::run() with port and handler
    if let Err(ref e) = redif::run( port, handler.clone() ) {
        error!("ERROR {}", e);
        std::process::exit(1);
    }
}

```

examples/simple.rs is a simple demo.


