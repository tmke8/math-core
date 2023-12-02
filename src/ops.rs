#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Op(pub char);

impl Op {
    #[inline]
    pub fn char(&self) -> char {
        self.0
    }
}

pub const NULL: Op = Op('\u{0}');
pub const EQUAL: Op = Op('=');
pub const TIMES: Op = Op('×');
pub const LEFT_CURLY_BRACKET: Op = Op('{');
pub const RIGHT_CURLY_BRACKET: Op = Op('}');
