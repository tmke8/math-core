use std::mem::MaybeUninit;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::itoa::fmt_u8_as_hex;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl RGB {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        RGB { r, g, b }
    }

    pub fn append_as_hex(&self, output: &mut String) {
        output.push('#');
        let mut buf = [MaybeUninit::<u8>::uninit(); 2];
        output.push_str(fmt_u8_as_hex(self.r, &mut buf));
        output.push_str(fmt_u8_as_hex(self.g, &mut buf));
        output.push_str(fmt_u8_as_hex(self.b, &mut buf));
    }
}
