/// Parse a string into a float, but only allow a limited set of inputs.
///
/// The problem is that `digits.parse::<f32>()` adds 18kB to the WASM binary size(!!), so we use
/// this limited parsing function here instead.
///
/// So, how does it work?
///
/// First, we check for a sign. If there is one, we strip it and remember it. Then we walk over the
/// bytes in a single pass, accumulating all digits (ignoring the decimal point) into a `u64`
/// mantissa and building an `f64` divisor of `10^n`, where `n` is the number of digits after the
/// decimal point. The result is `mantissa / divisor`, computed in `f64` and then narrowed to
/// `f32`, so the only significant rounding happens in that final narrowing step.
///
/// Inputs whose digits (ignoring the decimal point) don't fit into a `u64` return `None`, as does
/// anything that isn't an optional `-` followed by digits with at most one decimal point.
///
/// These are the largest and the smallest numbers this function can handle:
///
/// - `limited_float_parse("18446744073709551615")`
/// - `limited_float_parse("0.0000000000000000001")`
pub fn limited_float_parse(digits: &str) -> Option<f32> {
    let mut bytes = digits.as_bytes();

    // strip_prefix is panic-free (no slicing that could emit a bounds check).
    let neg = match bytes.strip_prefix(b"-") {
        Some(rest) => {
            bytes = rest;
            true
        }
        None => false,
    };

    let mut mantissa: u64 = 0;
    let mut divisor: f64 = 1.0;
    let mut seen_dot = false;
    let mut seen_digit = false;

    for &b in bytes {
        if b == b'.' {
            if seen_dot {
                return None; // second dot
            }
            seen_dot = true;
        } else {
            let d = b.wrapping_sub(b'0');
            if d > 9 {
                return None; // not a digit
            }
            // Checked ops: no overflow panic path in the binary, and inputs
            // with more significant digits than u64 can hold are rejected
            // rather than silently parsed to a wrong value.
            mantissa = mantissa.checked_mul(10)?.checked_add(d as u64)?;
            if seen_dot {
                divisor *= 10.0;
            }
            seen_digit = true;
        }
    }

    if !seen_digit {
        return None; // "", "-", ".", "-."
    }

    // Do the division in f64: the divisor (a power of 10 up to 1e19) and the
    // quotient are exact or near-exact in f64, so the only significant
    // rounding happens in the final narrowing to f32. f64 arithmetic is
    // native in WASM and adds no binary size.
    let v = (mantissa as f64 / divisor) as f32;
    Some(if neg { -v } else { v })
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
            // 0.18446743
            "0.1844674407370955161".parse::<f32>().unwrap()
        );
        assert_eq!(limited_float_parse("0.0000000000000000001").unwrap(), 1e-19);

        // Verify the rounding behavior.
        assert_eq!(limited_float_parse("16777216.0").unwrap(), 16777216.0);
        assert_eq!(limited_float_parse("16777217.0").unwrap(), 16777216.0);
        assert_eq!(limited_float_parse("16777218.0").unwrap(), 16777218.0);
        assert_eq!(limited_float_parse("16777219.0").unwrap(), 16777220.0);
    }
}
