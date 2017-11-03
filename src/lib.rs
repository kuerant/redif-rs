
//! redif -- a Redis protocol server Framework
//!
//!
//! Redif is a framework, it talks the data transport in redis protocol,
//! and call user provided Handler to handle the request.
//! User should implement Handler trait, and invoke Redif with redif::run( port, handler).
//! 
//! For example 
//! 
//! ```
//! 
//! extern crate redif;
//! 
//! use std::sync::{Arc,Mutex};
//! use redif::{Value, Handler};
//! 
//! // implement Handler trait
//! struct Store {
//!     kv : HashMap<String, String>,
//! }
//! 
//! impl Handler for Store {
//! 
//!     fn handle(&mut self, data: &Value) -> Option<Value> {
//! 		/// ...
//! 	}
//! 
//! }
//! 
//! 
//! fn main() {
//! 
//!     let port = 4344u16;
//!     let store = Store::new();
//!     let handler = Arc::new(Mutex::new(store));
//! 
//!     // call redif::run() with port and handler
//!     if let Err(ref e) = redif::run( port, handler.clone() ) {
//!         error!("ERROR {}", e);
//!         std::process::exit(1);
//!     }
//! }
//! 
//! ```
//!
//!

#[macro_use]
extern crate log;
extern crate amy;

mod redif;
mod help;
mod value;
mod frame_reader;
mod frame_writer;

pub use value::Value;
pub use value::encode_slice;
pub use redif::run;

/// Handler  handle client's request and produce response
///
/// if there is Some(response), then redif will send response to client;
/// if there is None response, then redif would send nothing to client,
/// and client maybe starve!
///
pub trait Handler {
    fn handle(&mut self, req: &Value) -> Option<Value>;
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
