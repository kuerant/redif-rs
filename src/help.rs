
#[allow(dead_code)]
pub fn hexdump(bytes: &[u8]) -> Vec<String> {
    const CHUNK_LENGTH: usize = 16;

    bytes.chunks(CHUNK_LENGTH).enumerate().map(|(n, s)| {
        let s1 : String = s.iter().map(|x| format!("{:02X}", x)).collect::<Vec<String>>().join(" ");
        let s2 : String = s.iter().map(|x| {
            if 0x20 <= *x && *x < 0x7f {
                *x as char
            } else {
                '.'
            }
        }).collect();
        format!("{:08x}  {:47}  {}", n*16, s1, s2)
    }).collect()
}

