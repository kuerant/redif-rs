
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
