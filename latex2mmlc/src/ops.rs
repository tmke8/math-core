#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op(pub &'static str);

impl Op {
    #[inline]
    pub fn str(&self) -> &str {
        self.0
    }
}

impl AsRef<str> for Op {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0
    }
}

// ASCII
pub(crate) const NULL: Op = Op("\u{0}");
pub(crate) const EXCLAMATION_MARK: Op = Op("!");
pub(crate) const APOS: Op = Op("\'");
pub(crate) const LEFT_PARENTHESIS: Op = Op("(");
pub(crate) const RIGHT_PARENTHESIS: Op = Op(")");
pub(crate) const ASTERISK: Op = Op("*");
pub(crate) const PLUS: Op = Op("+");
pub(crate) const COMMA: Op = Op(",");
pub(crate) const DOT: Op = Op(".");
pub(crate) const SOLIDUS: Op = Op("/");
pub(crate) const COLON: Op = Op(":");
pub(crate) const SEMICOLON: Op = Op(";");
pub(crate) const LT: Op = Op("<");
pub(crate) const EQUAL: Op = Op("=");
pub(crate) const GT: Op = Op(">");
pub(crate) const LEFT_SQUARE_BRACKET: Op = Op("[");
pub(crate) const RIGHT_SQUARE_BRACKET: Op = Op("]");
pub(crate) const LEFT_CURLY_BRACKET: Op = Op("{");
pub(crate) const VERTICAL_LINE: Op = Op("|");
pub(crate) const RIGHT_CURLY_BRACKET: Op = Op("}");

// Latin-1 Supplement Block
pub(crate) const TIMES: Op = Op("×");

// General Punctuation Block
pub(crate) const PRIME: Op = Op("′");

// Mathematical Operators Block
// https://cloford.com/resources/charcodes/utf-8_mathematical.htm
pub(crate) const FORALL: Op = Op("∀");
// pub(crate) const PART: Op = Op("∂");
pub(crate) const EXISTS: Op = Op("∃");
// pub(crate) const NABLA: Op = Op("∇");
pub(crate) const ISIN: Op = Op("∈");
pub(crate) const NOTIN: Op = Op("∉");
pub(crate) const NI: Op = Op("∋");
pub(crate) const PROD: Op = Op("∏");
pub(crate) const SUM: Op = Op("∑");
pub(crate) const MINUS: Op = Op("−");
pub(crate) const EQUIV: Op = Op("≡");
