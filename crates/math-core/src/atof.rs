/// Parse a string into a float, but only allow a limited set of inputs.
///
/// The problem is that `digits.parse::<f32>()` adds 18kB to the WASM binary size(!!), so we use
/// this limited parsing function here instead.
///
/// So, how does it work?
///
/// First, we check for a sign. If there is one, we remove it and remember it. Then we check for
/// the decimal point. We remove it but remember its position. We then parse the number without the
/// decimal point as a `u64`. We then shift the decimal point to the left by the number of digits
/// after the decimal point and convert the final result to `f32`.
///
/// These are the largest and the smallest numbers this function can handle:
///
/// - `limited_float_parse("18446744073709551615")`
/// - `limited_float_parse("0.0000000000000000001")`
pub fn limited_float_parse(digits: &str) -> Option<f32> {
    let (digits, sign) = if let Some(digits) = digits.strip_prefix('-') {
        (digits, -1.0f64)
    } else {
        (digits, 1.0f64)
    };
    let digits_bytes = digits.as_bytes();
    let integer_len = digits_bytes
        .iter()
        .position(|&b| b == b'.')
        .unwrap_or(digits.len());
    // Split the input at the decimal point.
    // The result will be two valid UTF-8 slices, because splitting directly before an ASCII dot
    // is guaranteed to be valid UTF-8.
    // If there is no dot, the second slice will be empty.
    let (integer, fraction_with_dot) = digits_bytes.split_at_checked(integer_len)?;
    // We split off the dot from the second part.
    // This is again valid UTF-8, because we know the second part begins with a dot.
    let fraction = fraction_with_dot.get(1..).unwrap_or(&[]);

    // We now need to join the two parts again, without the dot, into a single buffer.

    // The most (decimal) digits u64 can handle is 20, so we need a buffer of 20 bytes.
    let mut buffer = [0u8; 20];
    // Now we split off parts of the buffer and copy the digits into there.
    let mut remaining_buffer = buffer.as_mut_slice();
    let integer_buffer = remaining_buffer.split_off_mut(..integer.len())?;
    integer_buffer.copy_from_slice(integer);
    let fraction_buffer = remaining_buffer.split_off_mut(..fraction.len())?;
    fraction_buffer.copy_from_slice(fraction);

    // We get a slice of the part of the buffer that we have filled with digits.
    let num_without_dot = &buffer[..(integer.len() + fraction.len())];
    // SAFETY: All the splitting and joining operations we did should have produced valid UTF-8.
    debug_assert!(std::str::from_utf8(num_without_dot).is_ok());
    let num_without_dot = unsafe { std::str::from_utf8_unchecked(num_without_dot) };

    // Finally, we parse the number as a u64.
    // This parsing will do other checks that we didn't do, like checking that the input contains
    // only digits.
    // We cast to f64 instead of f32 to get the rounding right.
    let mut value = num_without_dot.parse::<u64>().ok()? as f64;
    // Shift the decimal point to the left by the number of digits in the fraction.
    value /= 10f64.powi(fraction.len() as i32);
    // Apply the sign.
    value *= sign;
    // Round the value to the nearest f32.
    // The values produced from shifting u64 are guaranteed to be representable as f32.
    Some(value as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_floats() {
        // multiple dots
        assert!(limited_float_parse("1..0").is_none());
        // multiple signs
        assert!(limited_float_parse("--1").is_none());
        // text
        assert!(limited_float_parse("hello").is_none());
        // empty string
        assert!(limited_float_parse("").is_none());
        // with spaces
        assert!(limited_float_parse(" 1.0").is_none());
        assert!(limited_float_parse("1.0 ").is_none());
        assert!(limited_float_parse(" 1.0 ").is_none());

        // 1 above the maximum value of u64
        assert!(limited_float_parse("18446744073709551616").is_none());
        // too long
        assert!(limited_float_parse("100000000000000.000000").is_none());
        // non-digit characters
        assert!(limited_float_parse("10👍🏽.0").is_none());
    }

    #[test]
    fn test_simple_float_parse() {
        assert_eq!(limited_float_parse("1.0").unwrap(), 1.0);
        assert_eq!(limited_float_parse("0001.0000").unwrap(), 1.0);
        assert_eq!(
            limited_float_parse("18446744073709551615").unwrap(),
            1.8446744e19
        );
        assert_eq!(
            limited_float_parse("-18446744073.709551615").unwrap(),
            -18446744000.0
        );
        assert_eq!(
            limited_float_parse("0.1844674407370955161").unwrap(),
            0.18446743
        );
        assert_eq!(limited_float_parse("0.0000000000000000001").unwrap(), 1e-19);

        // Verify the rounding behavior.
        assert_eq!(limited_float_parse("16777216.0").unwrap(), 16777216.0);
        assert_eq!(limited_float_parse("16777217.0").unwrap(), 16777216.0);
        assert_eq!(limited_float_parse("16777218.0").unwrap(), 16777218.0);
        assert_eq!(limited_float_parse("16777219.0").unwrap(), 16777220.0);
    }
}
