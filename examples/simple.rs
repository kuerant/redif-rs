
extern crate clap;
extern crate redif;

use std::sync::{Arc,Mutex};
use redif::{Value, Handler};

fn main() {
    let args = clap::App::new("Redis Server Framework")
        .version("0.1.0")
        .author("kuerant@126.com")
        .about("redis protocol server framework")
        .arg(clap::Arg::with_name("port")
             .short("p")
             .long("port")
             .takes_value(true)
             .value_name("PORT")
             .help("TCP port to listen (default 4400)"))
        .arg(clap::Arg::with_name("verbose")
             .short("v")
             .long("verbose")
             .takes_value(false)
             .help("in verbosity mode"))
        .get_matches();

    let port  = args.value_of("port").unwrap_or("4400").parse::<u16>().unwrap_or(4400);
    //let debug = false;
    //let verbose = args.is_present("verbose");

    init_logging();

    let store = Store::new();
    let handler = Arc::new(Mutex::new(store));

    if let Err(ref e) = redif::run( port, handler.clone() ) {
        error!("ERROR {}", e);
        std::process::exit(1);
    }
}

///////////////////////////////////////////////////////////////////////

use std::collections::HashMap;

struct Store {
    kv : HashMap<String, String>,
}

impl Store {
    const INVALID_REQUEST : &'static str = "Invalid Request";

    pub fn new() -> Self {
        Store {
            kv : HashMap::new(),
        }
    }

    fn command_set(&mut self, args: &[Value]) -> Option<Value> {
        if args.len() < 2 {
            return Some(Value::Error("too few arguments".to_owned()))
        }

        let key = match &args[0] {
            &Value::Data(ref data) => {
                match ::std::str::from_utf8(data) {
                    Ok(s) => s.to_owned(),
                    Err(e) => {
                        error!("bad request SET {:?} -- {}", args, e);
                        return Some(Value::Error(format!("bad request : {:?}", args)));
                    }
                }
            }
            _ => {
                return Some(Value::Error(format!("bad request SET {:?}", args)));
            }
        };
        let val = match &args[1] {
            &Value::Data(ref data) => {
                match ::std::str::from_utf8(data) {
                    Ok(s) => s.to_owned(),
                    Err(e) => {
                        error!("bad request SET {:?} -- {}", args, e);
                        return Some(Value::Error(format!("bad request : {:?}", args)));
                    }
                }
            }
            _ => {
                return Some(Value::Error(format!("bad request SET {:?}", args)));
            }
        };

        self.kv.insert(key, val);

        Some(Value::Status("OK".to_owned()))
    }

    fn command_get(&mut self, args: &[Value]) -> Option<Value> {
        if args.len() < 1 {
            return Some(Value::Error("too few arguments".to_owned()))
        }

        let key = match &args[0] {
            &Value::Data(ref data) => {
                match ::std::str::from_utf8(data) {
                    Ok(s) => s.to_owned(),
                    Err(e) => {
                        error!("bad request GET {:?} -- {}", args, e);
                        return Some(Value::Error(format!("bad request : {:?}", args)));
                    }
                }
            }
            _ => {
                return Some(Value::Error(format!("bad request GET {:?}", args)));
            }
        };

        match self.kv.get(&key) {
            Some(s) => Some(Value::Data(s.as_bytes().to_vec())),
            None => Some(Value::Nil),
        }
    }

    fn command_ping(&self, args: &[Value]) -> Option<Value> {
        if args.len() > 0 {
            let v = args.iter().map(|x| x.as_slice()).collect::<Vec<&[u8]>>().join(&0x20u8);
            Some(Value::Data(v))
        } else {
            Some(Value::Data(b"PONG".to_vec()))
        }
    }
}

impl Handler for Store {

    fn handle(&mut self, data: &Value) -> Option<Value> {
        if let Value::Bulk(ref req) = *data {
            if req.len() < 1 {
                return Some(Value::Nil);
            }

            //info!("REQUEST {:?}", req);
            let cmd = match req[0].to_string() {
                Ok(o) => match o {
                    Some(s) => s.to_uppercase(),
                    None => return Some(Value::Nil),
                }
                Err(e) => {
                    error!("invalid command {:?} -- {}", req[0], e);
                    return Some(Value::Error(Self::INVALID_REQUEST.to_owned()))
                }
            };
            //info!("COMMAND {:?}", &cmd);

            match cmd.as_str() {
                "SET" => self.command_set( &req[1..] ),
                "GET" => self.command_get( &req[1..] ),
                "PING" => self.command_ping( &req[1..] ),
                cmd => Some(Value::Error(format!("invalid command : {}", cmd))),
            }
        } else {
            Some(Value::Error(Self::INVALID_REQUEST.to_owned()))
        }
    }
}



///////////////////////////////////////////////////////////////////////
//

#[macro_use]
extern crate log;
extern crate time;
extern crate env_logger;

fn init_logging() {
    use log::{LogRecord, LogLevelFilter};
    use env_logger::LogBuilder;
    use std::env;

    static LOG_LEVEL_NAMES: [&'static str; 6] = ["O", "E", "W", "I", "D", "T"];

    fn basename(filename: &str) -> &str {
        filename.split("/").last().unwrap()
    }

    fn now() -> String {
        time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap()
    }

    let format = |record: &LogRecord| {
        format!("{} {} {}:{} {}", now(), LOG_LEVEL_NAMES[record.level() as usize], basename(record.location().file()), record.location().line(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();
    //// USAGE : RUST_LOG=debug target/debug/hello
}


