pub(crate) fn simple_float_parse(digits: &str) -> Result<f32, ()> {
    let (digits, sign) = if let Some(digits) = digits.strip_prefix('-') {
        (digits, -1.0f64)
    } else {
        (digits, 1.0f64)
    };
    let (integer, fraction) = if let Some(parts) = digits.split_once('.') {
        parts
    } else {
        (digits, "")
    };
    let frac_len = fraction.len() as i32;
    // the most digits we can handle is 39
    let mut buffer = [0u8; 39];
    buffer[0..integer.len()].copy_from_slice(integer.as_bytes());
    buffer[integer.len()..(integer.len() + fraction.len())].copy_from_slice(fraction.as_bytes());
    let mut value =
        unsafe { std::str::from_utf8_unchecked(&buffer[0..(integer.len() + fraction.len())]) }
            .parse::<u128>()
            .map_err(|_| ())? as f64;
    value /= 10f64.powi(frac_len);
    value *= sign;
    Ok(value as f32)
}
