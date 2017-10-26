
use std::io::Result;
use std::io;
use std::fmt;
use std::str;


#[derive(PartialEq, Eq, Clone)]
pub enum Value {
    /// A nil response from the server.
    Nil,
    NullArray,
    /// An integer response.  Note that there are a few situations
    /// in which redis actually returns a string for an integer which
    /// is why this library generally treats integers and strings
    /// the same for all numeric responses.
    Int(i64),
    /// An arbitary binary data.
    Data(Vec<u8>),
    /// A bulk response of more data.  This is generally used by redis
    /// to express nested structures.
    Bulk(Vec<Value>),
    /// A status response.
    Status(String),
    /// A status response which represents the string "OK".
    //Okay,
    /// An error response.
    Error(String),
}


impl Value {

    const RESP_MAX_SIZE: i64 = 512 * 1024 * 1024;

    pub fn decode(bytes: &[u8], start_index: usize) -> Result<(Value,usize)> {
        let len = bytes.len();
        let mut k = start_index;
        while (k < len) && (bytes[k] != b'\n') { 
            k += 1; 
        }
        if k >= len {
            return Ok((Value::Nil, 0));
        }
        if bytes[k - 1] != b'\r' {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid CRLF: {:?}", &bytes[start_index .. k+1])));
        }

        let p = start_index + 1;
        let q = k - 1;
        match bytes[start_index] {
            // Value::Status
            b'+' => {
                match String::from_utf8(bytes[p .. q].to_vec()) {
                    Ok(s) => return Ok((Value::Status(s), k + 1)),
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid Data: {:?} -- {}", &bytes[start_index .. k+1], e))),
                }
            }
            // Value::Error
            b'-' => {
                match String::from_utf8(bytes[p .. q].to_vec()) {
                    Ok(s) => return Ok((Value::Error(s), k + 1)),
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid Data: {:?} -- {}", &bytes[start_index .. k+1], e))),
                }
            }
            // Value::Int
            b':' => {
                match String::from_utf8(bytes[p .. q].to_vec()) {
                    Ok(s) => match s.parse::<i64>() {
                        Ok(x) => return Ok((Value::Int(x), k + 1)),
                        Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid Data: {:?} -- {}", &bytes[start_index .. k+1], e))),
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("invalid Data: {:?} -- {}", &bytes[start_index .. k+1], e))),
                }
            }
            // Value::Data
            b'$' => {
                let x = parse_length( &bytes[p .. q ] )?;
                if x == -1 {
                    return Ok((Value::Nil, k + 1));
                }
                if (x < -1) || (x > Self::RESP_MAX_SIZE) {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid Data length: {:?}", &bytes[start_index .. k+1])));
                }
                let n = x as usize;
                if (len - k) >= (n + 2) {
                    return Ok((Value::Data(bytes[k+1 .. k+n+1].to_vec()), k + n + 2 + 1));
                }
            }
            // Value::Bulk
            b'*' => {
                let x = parse_length( &bytes[p .. q ] )?;
                if x == -1 {
                    return Ok((Value::NullArray, k + 1));
                }
                if (x < -1) || (x > Self::RESP_MAX_SIZE) {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid Array length: {:?}", &bytes[start_index .. k+1])));
                }
                let n = x as usize;
                let mut array: Vec<Value> = Vec::with_capacity(n);
                let mut offset = k + 1;
                for _ in 0 .. n {
                    let (val, _offset) = Self::decode(bytes, offset)?;
                    if _offset == 0 {
                        return Ok((Value::Nil, 0));
                    }

                    offset = _offset;
                    array.push( val );
                }
                return Ok((Value::Bulk(array), offset));
            }
            // invalid prefix
            prefix => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid RESP type: {:?}", prefix)));
            }
        }

        Ok((Value::Nil, 0))
    }   //// decode()


    const CRLF_BYTES: &'static [u8] = b"\r\n";
    const NULL_BYTES: &'static [u8] = b"$-1\r\n";
    const NULL_ARRAY_BYTES: &'static [u8] = b"*-1\r\n";

    #[inline]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        match *self {
            Value::Nil => {
                buf.extend_from_slice(Self::NULL_BYTES);
            }
            Value::NullArray => {
                buf.extend_from_slice(Self::NULL_ARRAY_BYTES);
            }
            Value::Status(ref val) => {
                buf.push(b'+');
                buf.extend_from_slice(val.as_bytes());
                buf.extend_from_slice(Self::CRLF_BYTES);
            }
            Value::Error(ref val) => {
                buf.push(b'-');
                buf.extend_from_slice(val.as_bytes());
                buf.extend_from_slice(Self::CRLF_BYTES);
            }
            Value::Int(ref val) => {
                buf.push(b':');
                buf.extend_from_slice(val.to_string().as_bytes());
                buf.extend_from_slice(Self::CRLF_BYTES);
            }
            Value::Data(ref val) => {
                buf.push(b'$');
                buf.extend_from_slice(val.len().to_string().as_bytes());
                buf.extend_from_slice(Self::CRLF_BYTES);
                buf.extend_from_slice(val);
                buf.extend_from_slice(Self::CRLF_BYTES);
            }
            Value::Bulk(ref val) => {
                buf.push(b'*');
                buf.extend_from_slice(val.len().to_string().as_bytes());
                buf.extend_from_slice(Self::CRLF_BYTES);
                for item in val {
                    buf.append(&mut item.encode());
                }
            }
        }

        buf
    }   //// encode()

    #[inline]
    pub fn to_string(&self) -> ::std::result::Result<Option<String>, ::std::str::Utf8Error> {
        let s = match *self {
            Value::Status(ref val) => {
                Some(val.clone())
            }
            Value::Error(ref val) => {
                Some(val.clone())
            }
            Value::Int(ref val) => {
                Some(val.to_string())
            }
            Value::Data(ref val) => {
                match str::from_utf8(val) {
                    Ok(s) => Some(s.to_owned()),
                    Err(e) => return Err(e),
                }
            }
            _ => None,
        };

        Ok(s)
    }   //// to_string()

    #[inline]
    const NULL_SLICE: &'static [u8] = b"";
    pub fn as_slice(&self) -> &[u8] {
        match *self {
            Value::Data(ref val) => val.as_slice(),
            _ => Self::NULL_SLICE,
        }
    }
}

/// Encodes a slice of string to RESP binary buffer.
/// It is use to create a request command on redis client.
/// # Examples
/// ```
/// # use self::redif::encode_slice;
/// let array = ["SET", "a", "1"];
/// assert_eq!(encode_slice(&array),
///            "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n".to_string().into_bytes());
/// ```
pub fn encode_slice(slice: &[&str]) -> Vec<u8> {
    let array: Vec<Value> = slice.iter().map(|string| Value::Data(string.as_bytes().to_vec())).collect();
    Value::Bulk(array).encode()
}

fn parse_length(bytes: &[u8]) -> Result<i64> {
    String::from_utf8(bytes.to_vec()).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?.parse::<i64>().map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

impl fmt::Debug for Value {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Nil => write!(fmt, "nil"),
            Value::NullArray => write!(fmt, "null-array"),
            Value::Int(val) => write!(fmt, "int({:?})", val),
            Value::Data(ref val) => {
                match str::from_utf8(val) {
                    Ok(x) => write!(fmt, "string-data('{:?}')", x),
                    Err(_) => write!(fmt, "binary-data({:?})", val),
                }
            }
            Value::Bulk(ref values) => {
                try!(write!(fmt, "bulk("));
                let mut is_first = true;
                for val in values.iter() {
                    if !is_first {
                        try!(write!(fmt, ", "));
                    }
                    try!(write!(fmt, "{:?}", val));
                    is_first = false;
                }
                write!(fmt, ")")
            }
            //Value::Okay => write!(fmt, "ok"),
            Value::Status(ref s) => write!(fmt, "status({:?})", s),
            Value::Error(ref s) => write!(fmt, "error({:?})", s),
        }
    }
}


//unsafe impl Sync for Value {}
//unsafe impl Send for Value {}


#[cfg(test)]
mod tests {
    use super::Value;

    struct Case {
        data: Vec<u8>,
        want: Value,
    }

    #[test]
    fn struct_decoder() {
        let cases: &[Case] = &[
            Case {
                data: "+\r\n".to_string().into_bytes(),
                want: Value::Status("".to_string()),
            },
            Case {
                data: "+OK\r\n".to_string().into_bytes(),
                want: Value::Status("OK".to_string()),
            },
            Case {
                data: "+中文\r\n".to_string().into_bytes(),
                want: Value::Status("中文".to_string()),
            },
            Case {
                data: "-Error Message\r\n".to_string().into_bytes(),
                want: Value::Error("Error Message".to_string()),
            },
            Case {
                data: ":0\r\n".to_string().into_bytes(),
                want: Value::Int(0),
            },
            Case {
                data: ":-1\r\n".to_string().into_bytes(),
                want: Value::Int(-1),
            },
            Case {
                data: ":4400\r\n".to_string().into_bytes(),
                want: Value::Int(4400),
            },
            Case {
                data: "$-1\r\n".to_string().into_bytes(),
                want: Value::Nil,
            },
            Case {
                data: "$0\r\n\r\n".to_string().into_bytes(),
                want: Value::Data(Vec::new()),
            },
            Case {
                data: "$6\r\nfoobar\r\n".to_string().into_bytes(),
                want: Value::Data(b"foobar".to_vec()),
            },
            Case {
                data: "$17\r\n你好！\n 换行\r\n".to_string().into_bytes(),
                want: Value::Data("你好！\n 换行".as_bytes().to_vec()),
            },
            Case {
                data: "*-1\r\n".to_string().into_bytes(),
                want: Value::NullArray,
            },
            Case {
                data: "*0\r\n".to_string().into_bytes(),
                want: Value::Bulk(Vec::new()),
            },
            Case {
                data: "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".to_string().into_bytes(),
                want: Value::Bulk(vec![Value::Data(b"foo".to_vec()), Value::Data(b"bar".to_vec())]),
            },
            Case {
                data: "*3\r\n:1\r\n:2\r\n:3\r\n".to_string().into_bytes(),
                want: Value::Bulk(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            },
            Case {
                data: "*4\r\n:1\r\n:2\r\n:3\r\n$6\r\nfoobar\r\n".to_string().into_bytes(),
                want: Value::Bulk(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Data(b"foobar".to_vec())]),
            },
        ];

        // single decode
        for case in cases {
            let (val, _) = Value::decode(&case.data[..], 0).unwrap();
            assert_eq!(val, case.want);
        }

        // multiple decode
        let mut all: Vec<u8> = Vec::new();
        for case in cases {
            all.extend_from_slice(case.data.as_slice());
        }
        let mut offset = 0;
        for case in cases {
            let (val, _offset) = Value::decode(&all, offset).unwrap();
            assert_eq!(val, case.want);
            offset = _offset;
        }
    }
}


