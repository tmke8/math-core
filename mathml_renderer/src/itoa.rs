pub fn append_u8_as_hex(output: &mut String, b: u8) {
    let buf = [digit_to_hex_ascii(b >> 4), digit_to_hex_ascii(b & 0x0F)];
    // SAFETY: `buf` is always valid ASCII.
    output.push_str(unsafe { std::str::from_utf8_unchecked(&buf) });
}

#[inline]
fn digit_to_hex_ascii(digit: u8) -> u8 {
    match digit {
        0..=9 => digit + b'0',
        10..=15 => digit - 10 + b'A',
        _ => unreachable!("Invalid hex digit"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_u8_hex() {
        let mut buf = String::new();
        append_u8_as_hex(&mut buf, 0);
        assert_eq!(&buf, "00");
        buf.clear();
    }
}
