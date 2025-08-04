/// Decode the first UTF-8 `char` out of the buffer.
///
/// Returns `Ok(char, len)` if the char can be decoded succesfully,
/// `Err(false)` if there is not enough bytes to fully decode
/// a char, and `Err(true)` if the bytes are invalid
#[allow(unused)]
pub(crate) fn decode_char(buf: &[u8]) -> Result<(char, usize), bool> {
    let Some(first) = buf.first() else {
        return Err(false);
    };
    let first = *first;
    let expected_len = if first & 0x80 == 0x00 {
        1
    } else if first & 0xE0 == 0xC0 {
        2
    } else if first & 0xF0 == 0xE0 {
        3
    } else if first & 0xF8 == 0xF0 {
        4
    } else {
        // invalid first byte
        return Err(true);
    };
    if buf.len() < expected_len {
        return Err(false);
    }
    let Ok(s) = std::str::from_utf8(&buf[0..expected_len]) else {
        // invalid bytes
        return Err(true);
    };
    match s.chars().next() {
        Some(c) => {
            debug_assert_eq!(c.len_utf8(), expected_len, "utf8::decode_char wrong");
            Ok((c, expected_len))
        }
        None => Err(false),
    }
}
