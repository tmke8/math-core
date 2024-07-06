use std::str;

// #[derive(Debug, Clone, PartialEq, Eq, Copy)]
// #[repr(transparent)]
// pub struct Op(char);

// impl From<Op> for char {
//     #[inline]
//     fn from(op: Op) -> Self {
//         op.0
//     }
// }

// impl From<&Op> for char {
//     #[inline]
//     fn from(op: &Op) -> Self {
//         op.0
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Utf8Char {
    OneByte([u8; 1]),
    TwoByte([u8; 2]),
    ThreeByte([u8; 3]),
}

impl Utf8Char {
    pub fn as_str(&self) -> &str {
        let slice = match self {
            Utf8Char::OneByte(array) => &array[..],
            Utf8Char::TwoByte(array) => &array[..],
            Utf8Char::ThreeByte(array) => &array[..],
        };
        unsafe { str::from_utf8_unchecked(slice) }
    }

    pub const fn new(c: char) -> Self {
        let len = c.len_utf8();
        let mut c = c as u32;
        if len == 1 {
            Self::OneByte([c as u8])
        } else {
            let mut parts = 0; // convert to 6-bit bytes
            parts |= c & 0x3f;
            c >>= 6;
            parts <<= 8;
            parts |= c & 0x3f;
            c >>= 6;
            parts <<= 8;
            parts |= c & 0x3f;
            c >>= 6;
            parts <<= 8;
            parts |= c & 0x3f;
            parts |= 0x80_80_80_80; // set the most significant bit
            parts >>= 8 * (4 - len); // right-align bytes
                                     // Now, unused bytes are zero, (which matters for Utf8Char.eq())
                                     // and the rest are 0b10xx_xxxx

            // set header on first byte
            parts |= (0xff_00u32 >> len) & 0xff; // store length
            parts &= !(1u32 << (7 - len)); // clear the next bit after it

            let bytes = parts.to_le_bytes();
            match len {
                2 => Self::TwoByte([bytes[0], bytes[1]]),
                3 => Self::ThreeByte([bytes[0], bytes[1], bytes[2]]),
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(transparent)]
pub struct Op(Utf8Char);

impl Op {
    pub const fn new(c: char) -> Self {
        Self(Utf8Char::new(c))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

//
// Unicode Block: Basic Latin
//
pub(crate) const NULL: Op = Op::new('\u{00}');
pub(crate) const EXCLAMATION_MARK: Op = Op::new('!');
// pub(crate) const APOSTROPHE: Op = Op::new('\'');
pub(crate) const LEFT_PARENTHESIS: Op = Op::new('(');
pub(crate) const RIGHT_PARENTHESIS: Op = Op::new(')');
pub(crate) const ASTERISK: Op = Op::new('*');
pub(crate) const PLUS_SIGN: Op = Op::new('+');
pub(crate) const COMMA: Op = Op::new(',');
pub(crate) const FULL_STOP: Op = Op::new('.');
pub(crate) const SOLIDUS: Op = Op::new('/');
pub(crate) const COLON: Op = Op::new(':');
pub(crate) const SEMICOLON: Op = Op::new(';');
// pub(crate) const LESS_THAN_SIGN: Op = Op::new('<');
pub(crate) const EQUALS_SIGN: Op = Op::new('=');
// pub(crate) const GREATER_THAN_SIGN: Op = Op::new('>');
// pub(crate) const QUESTION_MARK: Op = Op::new('?');
pub(crate) const LEFT_SQUARE_BRACKET: Op = Op::new('[');
pub(crate) const REVERSE_SOLIDUS: Op = Op::new('\\');
pub(crate) const RIGHT_SQUARE_BRACKET: Op = Op::new(']');
pub(crate) const CIRCUMFLEX_ACCENT: Op = Op::new('^');
pub(crate) const LOW_LINE: Op = Op::new('_');
pub(crate) const GRAVE_ACCENT: Op = Op::new('`');
pub(crate) const LEFT_CURLY_BRACKET: Op = Op::new('{');
pub(crate) const VERTICAL_LINE: Op = Op::new('|');
pub(crate) const RIGHT_CURLY_BRACKET: Op = Op::new('}');
pub(crate) const TILDE: Op = Op::new('~');

//
// Unicode Block: Latin-1 Supplement
//
pub(crate) const DIAERESIS: Op = Op::new('¨');
pub(crate) const NOT_SIGN: Op = Op::new('¬');
pub(crate) const MACRON: Op = Op::new('¯');
pub(crate) const PLUS_MINUS_SIGN: Op = Op::new('±');
pub(crate) const ACUTE_ACCENT: Op = Op::new('´');
pub(crate) const MIDDLE_DOT: Op = Op::new('·');
pub(crate) const MULTIPLICATION_SIGN: Op = Op::new('×');
pub(crate) const DIVISION_SIGN: Op = Op::new('÷');

//
// Unicode Block: Spacing Modifier Letters
//
pub(crate) const CARON: Op = Op::new('ˇ');
pub(crate) const BREVE: Op = Op::new('˘');
pub(crate) const DOT_ABOVE: Op = Op::new('˙');

//
// Unicode Block: General Punctuation
//
pub(crate) const DOUBLE_VERTICAL_LINE: Op = Op::new('‖');
pub(crate) const HORIZONTAL_ELLIPSIS: Op = Op::new('…');
pub(crate) const PRIME: Op = Op::new('′');

//
// Unicode Block: Arrows
//
pub(crate) const LEFTWARDS_ARROW: Op = Op::new('←');
pub(crate) const UPWARDS_ARROW: Op = Op::new('↑');
pub(crate) const RIGHTWARDS_ARROW: Op = Op::new('→');
pub(crate) const DOWNWARDS_ARROW: Op = Op::new('↓');
pub(crate) const LEFT_RIGHT_ARROW: Op = Op::new('↔');
pub(crate) const UP_DOWN_ARROW: Op = Op::new('↕');
pub(crate) const NORTH_WEST_ARROW: Op = Op::new('↖');
pub(crate) const NORTH_EAST_ARROW: Op = Op::new('↗');
pub(crate) const SOUTH_EAST_ARROW: Op = Op::new('↘');
pub(crate) const SOUTH_WEST_ARROW: Op = Op::new('↙');
pub(crate) const LEFTWARDS_ARROW_WITH_STROKE: Op = Op::new('↚');
pub(crate) const RIGHTWARDS_ARROW_WITH_STROKE: Op = Op::new('↛');
// pub(crate) const LEFTWARDS_WAVE_ARROW: Op = Op::new('↜');
// pub(crate) const RIGHTWARDS_WAVE_ARROW: Op = Op::new('↝');
// pub(crate) const LEFTWARDS_TWO_HEADED_ARROW: Op = Op::new('↞');
// pub(crate) const UPWARDS_TWO_HEADED_ARROW: Op = Op::new('↟');
// pub(crate) const RIGHTWARDS_TWO_HEADED_ARROW: Op = Op::new('↠');
// pub(crate) const DOWNWARDS_TWO_HEADED_ARROW: Op = Op::new('↡');
pub(crate) const LEFTWARDS_ARROW_WITH_TAIL: Op = Op::new('↢');
pub(crate) const RIGHTWARDS_ARROW_WITH_TAIL: Op = Op::new('↣');
// pub(crate) const LEFTWARDS_ARROW_FROM_BAR: Op = Op::new('↤');
// pub(crate) const UPWARDS_ARROW_FROM_BAR: Op = Op::new('↥');
pub(crate) const RIGHTWARDS_ARROW_FROM_BAR: Op = Op::new('↦');
// pub(crate) const DOWNWARDS_ARROW_FROM_BAR: Op = Op::new('↧');
// pub(crate) const UP_DOWN_ARROW_WITH_BASE: Op = Op::new('↨');
pub(crate) const LEFTWARDS_ARROW_WITH_HOOK: Op = Op::new('↩');
pub(crate) const RIGHTWARDS_ARROW_WITH_HOOK: Op = Op::new('↪');
pub(crate) const LEFTWARDS_ARROW_WITH_LOOP: Op = Op::new('↫');
pub(crate) const RIGHTWARDS_ARROW_WITH_LOOP: Op = Op::new('↬');
pub(crate) const LEFT_RIGHT_WAVE_ARROW: Op = Op::new('↭');
pub(crate) const LEFT_RIGHT_ARROW_WITH_STROKE: Op = Op::new('↮');
pub(crate) const DOWNWARDS_ZIGZAG_ARROW: Op = Op::new('↯');
pub(crate) const UPWARDS_ARROW_WITH_TIP_LEFTWARDS: Op = Op::new('↰');
pub(crate) const UPWARDS_ARROW_WITH_TIP_RIGHTWARDS: Op = Op::new('↱');
// pub(crate) const DOWNWARDS_ARROW_WITH_TIP_LEFTWARDS: Op = Op::new('↲');
// pub(crate) const DOWNWARDS_ARROW_WITH_TIP_RIGHTWARDS: Op = Op::new('↳');
// pub(crate) const RIGHTWARDS_ARROW_WITH_CORNER_DOWNWARDS: Op = Op::new('↴');
// pub(crate) const DOWNWARDS_ARROW_WITH_CORNER_LEFTWARDS: Op = Op::new('↵');
pub(crate) const ANTICLOCKWISE_TOP_SEMICIRCLE_ARROW: Op = Op::new('↶');
pub(crate) const CLOCKWISE_TOP_SEMICIRCLE_ARROW: Op = Op::new('↷');
// pub(crate) const NORTH_WEST_ARROW_TO_LONG_BAR: Op = Op::new('↸');
// pub(crate) const LEFTWARDS_ARROW_TO_BAR_OVER_RIGHTWARDS_ARROW_TO_BAR: Op = Op::new('↹');
pub(crate) const ANTICLOCKWISE_OPEN_CIRCLE_ARROW: Op = Op::new('↺');
pub(crate) const CLOCKWISE_OPEN_CIRCLE_ARROW: Op = Op::new('↻');
pub(crate) const LEFTWARDS_HARPOON_WITH_BARB_UPWARDS: Op = Op::new('↼');
pub(crate) const LEFTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Op = Op::new('↽');
pub(crate) const UPWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Op = Op::new('↾');
pub(crate) const UPWARDS_HARPOON_WITH_BARB_LEFTWARDS: Op = Op::new('↿');
pub(crate) const RIGHTWARDS_HARPOON_WITH_BARB_UPWARDS: Op = Op::new('⇀');
pub(crate) const RIGHTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Op = Op::new('⇁');
pub(crate) const DOWNWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Op = Op::new('⇂');
pub(crate) const DOWNWARDS_HARPOON_WITH_BARB_LEFTWARDS: Op = Op::new('⇃');
pub(crate) const RIGHTWARDS_ARROW_OVER_LEFTWARDS_ARROW: Op = Op::new('⇄');
// pub(crate) const UPWARDS_ARROW_LEFTWARDS_OF_DOWNWARDS_ARROW: Op = Op::new('⇅');
pub(crate) const LEFTWARDS_ARROW_OVER_RIGHTWARDS_ARROW: Op = Op::new('⇆');
pub(crate) const LEFTWARDS_PAIRED_ARROWS: Op = Op::new('⇇');
pub(crate) const UPWARDS_PAIRED_ARROWS: Op = Op::new('⇈');
pub(crate) const RIGHTWARDS_PAIRED_ARROWS: Op = Op::new('⇉');
pub(crate) const DOWNWARDS_PAIRED_ARROWS: Op = Op::new('⇊');
pub(crate) const LEFTWARDS_HARPOON_OVER_RIGHTWARDS_HARPOON: Op = Op::new('⇋');
pub(crate) const RIGHTWARDS_HARPOON_OVER_LEFTWARDS_HARPOON: Op = Op::new('⇌');
pub(crate) const LEFTWARDS_DOUBLE_ARROW_WITH_STROKE: Op = Op::new('⇍');
pub(crate) const LEFT_RIGHT_DOUBLE_ARROW_WITH_STROKE: Op = Op::new('⇎');
pub(crate) const RIGHTWARDS_DOUBLE_ARROW_WITH_STROKE: Op = Op::new('⇏');
pub(crate) const LEFTWARDS_DOUBLE_ARROW: Op = Op::new('⇐');
pub(crate) const UPWARDS_DOUBLE_ARROW: Op = Op::new('⇑');
pub(crate) const RIGHTWARDS_DOUBLE_ARROW: Op = Op::new('⇒');
pub(crate) const DOWNWARDS_DOUBLE_ARROW: Op = Op::new('⇓');
pub(crate) const LEFT_RIGHT_DOUBLE_ARROW: Op = Op::new('⇔');
pub(crate) const UP_DOWN_DOUBLE_ARROW: Op = Op::new('⇕');
// pub(crate) const NORTH_WEST_DOUBLE_ARROW: Op = Op::new('⇖');
// pub(crate) const NORTH_EAST_DOUBLE_ARROW: Op = Op::new('⇗');
// pub(crate) const SOUTH_EAST_DOUBLE_ARROW: Op = Op::new('⇘');
// pub(crate) const SOUTH_WEST_DOUBLE_ARROW: Op = Op::new('⇙');
pub(crate) const LEFTWARDS_TRIPLE_ARROW: Op = Op::new('⇚');
pub(crate) const RIGHTWARDS_TRIPLE_ARROW: Op = Op::new('⇛');
// pub(crate) const LEFTWARDS_SQUIGGLE_ARROW: Op = Op::new('⇜');
pub(crate) const RIGHTWARDS_SQUIGGLE_ARROW: Op = Op::new('⇝');
// pub(crate) const UPWARDS_ARROW_WITH_DOUBLE_STROKE: Op = Op::new('⇞');
// pub(crate) const DOWNWARDS_ARROW_WITH_DOUBLE_STROKE: Op = Op::new('⇟');
// pub(crate) const LEFTWARDS_DASHED_ARROW: Op = Op::new('⇠');
// pub(crate) const UPWARDS_DASHED_ARROW: Op = Op::new('⇡');
// pub(crate) const RIGHTWARDS_DASHED_ARROW: Op = Op::new('⇢');
// pub(crate) const DOWNWARDS_DASHED_ARROW: Op = Op::new('⇣');
// pub(crate) const LEFTWARDS_ARROW_TO_BAR: Op = Op::new('⇤');
// pub(crate) const RIGHTWARDS_ARROW_TO_BAR: Op = Op::new('⇥');
// pub(crate) const LEFTWARDS_WHITE_ARROW: Op = Op::new('⇦');
// pub(crate) const UPWARDS_WHITE_ARROW: Op = Op::new('⇧');
// pub(crate) const RIGHTWARDS_WHITE_ARROW: Op = Op::new('⇨');
// pub(crate) const DOWNWARDS_WHITE_ARROW: Op = Op::new('⇩');
// pub(crate) const UPWARDS_WHITE_ARROW_FROM_BAR: Op = Op::new('⇪');
// pub(crate) const UPWARDS_WHITE_ARROW_ON_PEDESTAL: Op = Op::new('⇫');
// pub(crate) const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_HORIZONTAL_BAR: Op = Op::new('⇬');
// pub(crate) const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_VERTICAL_BAR: Op = Op::new('⇭');
// pub(crate) const UPWARDS_WHITE_DOUBLE_ARROW: Op = Op::new('⇮');
// pub(crate) const UPWARDS_WHITE_DOUBLE_ARROW_ON_PEDESTAL: Op = Op::new('⇯');
// pub(crate) const RIGHTWARDS_WHITE_ARROW_FROM_WALL: Op = Op::new('⇰');
// pub(crate) const NORTH_WEST_ARROW_TO_CORNER: Op = Op::new('⇱');
// pub(crate) const SOUTH_EAST_ARROW_TO_CORNER: Op = Op::new('⇲');
// pub(crate) const UP_DOWN_WHITE_ARROW: Op = Op::new('⇳');
// pub(crate) const RIGHT_ARROW_WITH_SMALL_CIRCLE: Op = Op::new('⇴');
// pub(crate) const DOWNWARDS_ARROW_LEFTWARDS_OF_UPWARDS_ARROW: Op = Op::new('⇵');
// pub(crate) const THREE_RIGHTWARDS_ARROWS: Op = Op::new('⇶');
// pub(crate) const LEFTWARDS_ARROW_WITH_VERTICAL_STROKE: Op = Op::new('⇷');
// pub(crate) const RIGHTWARDS_ARROW_WITH_VERTICAL_STROKE: Op = Op::new('⇸');
// pub(crate) const LEFT_RIGHT_ARROW_WITH_VERTICAL_STROKE: Op = Op::new('⇹');
// pub(crate) const LEFTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Op = Op::new('⇺');
// pub(crate) const RIGHTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Op = Op::new('⇻');
// pub(crate) const LEFT_RIGHT_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Op = Op::new('⇼');
// pub(crate) const LEFTWARDS_OPEN_HEADED_ARROW: Op = Op::new('⇽');
// pub(crate) const RIGHTWARDS_OPEN_HEADED_ARROW: Op = Op::new('⇾');
// pub(crate) const LEFT_RIGHT_OPEN_HEADED_ARROW: Op = Op::new('⇿');

//
// Unicode Block: Mathematical Operators
//
pub(crate) const FOR_ALL: Op = Op::new('∀');
// pub(crate) const PARTIAL_DIFFERENTIAL: Op = Op::new('∂');
pub(crate) const THERE_EXISTS: Op = Op::new('∃');
pub(crate) const THERE_DOES_NOT_EXIST: Op = Op::new('∄');
pub(crate) const NABLA: Op = Op::new('∇');
pub(crate) const ELEMENT_OF: Op = Op::new('∈');
pub(crate) const NOT_AN_ELEMENT_OF: Op = Op::new('∉');
pub(crate) const CONTAINS_AS_MEMBER: Op = Op::new('∋');
pub(crate) const N_ARY_PRODUCT: Op = Op::new('∏');
pub(crate) const N_ARY_COPRODUCT: Op = Op::new('∐');
pub(crate) const N_ARY_SUMMATION: Op = Op::new('∑');
pub(crate) const MINUS_SIGN: Op = Op::new('−');
pub(crate) const MINUS_OR_PLUS_SIGN: Op = Op::new('∓');
pub(crate) const DOT_PLUS: Op = Op::new('∔');
pub(crate) const SET_MINUS: Op = Op::new('∖');
pub(crate) const ASTERISK_OPERATOR: Op = Op::new('∗');
pub(crate) const RING_OPERATOR: Op = Op::new('∘');
pub(crate) const BULLET_OPERATOR: Op = Op::new('∙');
pub(crate) const PROPORTIONAL_TO: Op = Op::new('∝');
// pub(crate) const INFINITY: Op = Op::new('∞');
pub(crate) const DIVIDES: Op = Op::new('∣');
pub(crate) const DOES_NOT_DIVIDE: Op = Op::new('∤');
pub(crate) const PARALLEL_TO: Op = Op::new('∥');
pub(crate) const NOT_PARALLEL_TO: Op = Op::new('∦');
pub(crate) const LOGICAL_AND: Op = Op::new('∧');
pub(crate) const LOGICAL_OR: Op = Op::new('∨');
pub(crate) const INTERSECTION: Op = Op::new('∩');
pub(crate) const UNION: Op = Op::new('∪');
pub(crate) const INTEGRAL: Op = Op::new('∫');
pub(crate) const DOUBLE_INTEGRAL: Op = Op::new('∬');
pub(crate) const TRIPLE_INTEGRAL: Op = Op::new('∭');
pub(crate) const CONTOUR_INTEGRAL: Op = Op::new('∮');
pub(crate) const TILDE_OPERATOR: Op = Op::new('∼');
pub(crate) const WREATH_PRODUCT: Op = Op::new('≀');
pub(crate) const NOT_TILDE: Op = Op::new('≁');
pub(crate) const ASYMPTOTICALLY_EQUAL_TO: Op = Op::new('≃');
pub(crate) const NOT_ASYMPTOTICALLY_EQUAL_TO: Op = Op::new('≄');
pub(crate) const APPROXIMATELY_EQUAL_TO: Op = Op::new('≅');
pub(crate) const ALMOST_EQUAL_TO: Op = Op::new('≈');
pub(crate) const NOT_ALMOST_EQUAL_TO: Op = Op::new('≉');
pub(crate) const ALMOST_EQUAL_OR_EQUAL_TO: Op = Op::new('≊');
pub(crate) const EQUIVALENT_TO: Op = Op::new('≍');
pub(crate) const APPROACHES_THE_LIMIT: Op = Op::new('≐');
pub(crate) const GEOMETRICALLY_EQUAL_TO: Op = Op::new('≑');
pub(crate) const APPROXIMATELY_EQUAL_TO_OR_THE_IMAGE_OF: Op = Op::new('≒');
pub(crate) const IMAGE_OF_OR_APPROXIMATELY_EQUAL_TO: Op = Op::new('≓');
// pub(crate) const COLON_EQUALS: Op = Op::new('≔');
// pub(crate) const EQUALS_COLON: Op = Op::new('≕');
pub(crate) const RING_IN_EQUAL_TO: Op = Op::new('≖');
pub(crate) const RING_EQUAL_TO: Op = Op::new('≗');
// pub(crate) const CORRESPONDS_TO: Op = Op::new('≘');
// pub(crate) const ESTIMATES: Op = Op::new('≙');
// pub(crate) const EQUIANGULAR_TO: Op = Op::new('≚');
// pub(crate) const STAR_EQUALS: Op = Op::new('≛');
pub(crate) const DELTA_EQUAL_TO: Op = Op::new('≜');
// pub(crate) const EQUAL_TO_BY_DEFINITION: Op = Op::new('≝');
// pub(crate) const MEASURED_BY: Op = Op::new('≞');
// pub(crate) const QUESTIONED_EQUAL_TO: Op = Op::new('≟');
pub(crate) const NOT_EQUAL_TO: Op = Op::new('≠');
pub(crate) const IDENTICAL_TO: Op = Op::new('≡');
pub(crate) const NOT_IDENTICAL_TO: Op = Op::new('≢');
pub(crate) const LESS_THAN_OR_EQUAL_TO: Op = Op::new('≤');
pub(crate) const GREATER_THAN_OR_EQUAL_TO: Op = Op::new('≥');
pub(crate) const LESS_THAN_OVER_EQUAL_TO: Op = Op::new('≦');
pub(crate) const GREATER_THAN_OVER_EQUAL_TO: Op = Op::new('≧');
pub(crate) const MUCH_LESS_THAN: Op = Op::new('≪');
pub(crate) const MUCH_GREATER_THAN: Op = Op::new('≫');
// pub(crate) const BETWEEN: Op = Op::new('≬');
// pub(crate) const NOT_EQUIVALENT_TO: Op = Op::new('≭');
pub(crate) const NOT_LESS_THAN: Op = Op::new('≮');
pub(crate) const NOT_GREATER_THAN: Op = Op::new('≯');
pub(crate) const NEITHER_LESS_THAN_NOR_EQUAL_TO: Op = Op::new('≰');
pub(crate) const NEITHER_GREATER_THAN_NOR_EQUAL_TO: Op = Op::new('≱');
pub(crate) const LESS_THAN_OR_EQUIVALENT_TO: Op = Op::new('≲');
pub(crate) const GREATER_THAN_OR_EQUIVALENT_TO: Op = Op::new('≳');
// pub(crate) const NEITHER_LESS_THAN_NOR_EQUIVALENT_TO: Op = Op::new('≴');
// pub(crate) const NEITHER_GREATER_THAN_NOR_EQUIVALENT_TO: Op = Op::new('≵');
pub(crate) const LESS_THAN_OR_GREATER_THAN: Op = Op::new('≶');
// pub(crate) const GREATER_THAN_OR_LESS_THAN: Op = Op::new('≷');
// pub(crate) const NEITHER_LESS_THAN_NOR_GREATER_THAN: Op = Op::new('≸');
// pub(crate) const NEITHER_GREATER_THAN_NOR_LESS_THAN: Op = Op::new('≹');
pub(crate) const PRECEDES: Op = Op::new('≺');
pub(crate) const SUCCEEDS: Op = Op::new('≻');
// pub(crate) const PRECEDES_OR_EQUAL_TO: Op = Op::new('≼');
// pub(crate) const SUCCEEDS_OR_EQUAL_TO: Op = Op::new('≽');
// pub(crate) const PRECEDES_OR_EQUIVALENT_TO: Op = Op::new('≾');
// pub(crate) const SUCCEEDS_OR_EQUIVALENT_TO: Op = Op::new('≿');
pub(crate) const DOES_NOT_PRECEDE: Op = Op::new('⊀');
pub(crate) const DOES_NOT_SUCCEED: Op = Op::new('⊁');
pub(crate) const SUBSET_OF: Op = Op::new('⊂');
pub(crate) const SUPERSET_OF: Op = Op::new('⊃');
pub(crate) const NOT_A_SUBSET_OF: Op = Op::new('⊄');
pub(crate) const NOT_A_SUPERSET_OF: Op = Op::new('⊅');
pub(crate) const SUBSET_OF_OR_EQUAL_TO: Op = Op::new('⊆');
pub(crate) const SUPERSET_OF_OR_EQUAL_TO: Op = Op::new('⊇');
pub(crate) const NEITHER_A_SUBSET_OF_NOR_EQUAL_TO: Op = Op::new('⊈');
pub(crate) const NEITHER_A_SUPERSET_OF_NOR_EQUAL_TO: Op = Op::new('⊉');
pub(crate) const SUBSET_OF_WITH_NOT_EQUAL_TO: Op = Op::new('⊊');
pub(crate) const SUPERSET_OF_WITH_NOT_EQUAL_TO: Op = Op::new('⊋');
// pub(crate) const MULTISET: Op = Op::new('⊌');
// pub(crate) const MULTISET_MULTIPLICATION: Op = Op::new('⊍');
pub(crate) const MULTISET_UNION: Op = Op::new('⊎');
pub(crate) const SQUARE_IMAGE_OF: Op = Op::new('⊏');
pub(crate) const SQUARE_ORIGINAL_OF: Op = Op::new('⊐');
pub(crate) const SQUARE_IMAGE_OF_OR_EQUAL_TO: Op = Op::new('⊑');
pub(crate) const SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Op = Op::new('⊒');
pub(crate) const SQUARE_CAP: Op = Op::new('⊓');
pub(crate) const SQUARE_CUP: Op = Op::new('⊔');
pub(crate) const CIRCLED_PLUS: Op = Op::new('⊕');
pub(crate) const CIRCLED_MINUS: Op = Op::new('⊖');
pub(crate) const CIRCLED_TIMES: Op = Op::new('⊗');
pub(crate) const CIRCLED_DIVISION_SLASH: Op = Op::new('⊘');
pub(crate) const CIRCLED_DOT_OPERATOR: Op = Op::new('⊙');
pub(crate) const CIRCLED_RING_OPERATOR: Op = Op::new('⊚');
pub(crate) const CIRCLED_ASTERISK_OPERATOR: Op = Op::new('⊛');
// pub(crate) const CIRCLED_EQUALS: Op = Op::new('⊜');
pub(crate) const CIRCLED_DASH: Op = Op::new('⊝');
pub(crate) const SQUARED_PLUS: Op = Op::new('⊞');
pub(crate) const SQUARED_MINUS: Op = Op::new('⊟');
pub(crate) const SQUARED_TIMES: Op = Op::new('⊠');
pub(crate) const SQUARED_DOT_OPERATOR: Op = Op::new('⊡');
pub(crate) const RIGHT_TACK: Op = Op::new('⊢');
pub(crate) const LEFT_TACK: Op = Op::new('⊣');
pub(crate) const DOWN_TACK: Op = Op::new('⊤');
pub(crate) const UP_TACK: Op = Op::new('⊥');
// pub(crate) const ASSERTION: Op = Op::new('⊦');
// pub(crate) const MODELS: Op = Op::new('⊧');
pub(crate) const TRUE: Op = Op::new('⊨');
pub(crate) const FORCES: Op = Op::new('⊩');
// pub(crate) const TRIPLE_VERTICAL_BAR_RIGHT_TURNSTILE: Op = Op::new('⊪');
// pub(crate) const DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Op = Op::new('⊫');
// pub(crate) const DOES_NOT_PROVE: Op = Op::new('⊬');
// pub(crate) const NOT_TRUE: Op = Op::new('⊭');
// pub(crate) const DOES_NOT_FORCE: Op = Op::new('⊮');
// pub(crate) const NEGATED_DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Op = Op::new('⊯');
// pub(crate) const PRECEDES_UNDER_RELATION: Op = Op::new('⊰');
// pub(crate) const SUCCEEDS_UNDER_RELATION: Op = Op::new('⊱');
pub(crate) const NORMAL_SUBGROUP_OF: Op = Op::new('⊲');
pub(crate) const CONTAINS_AS_NORMAL_SUBGROUP: Op = Op::new('⊳');
pub(crate) const NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Op = Op::new('⊴');
pub(crate) const CONTAINS_AS_NORMAL_SUBGROUP_OR_EQUAL_TO: Op = Op::new('⊵');
// pub(crate) const ORIGINAL_OF: Op = Op::new('⊶');
// pub(crate) const IMAGE_OF: Op = Op::new('⊷');
pub(crate) const MULTIMAP: Op = Op::new('⊸');
// pub(crate) const HERMITIAN_CONJUGATE_MATRIX: Op = Op::new('⊹');
pub(crate) const INTERCALATE: Op = Op::new('⊺');
pub(crate) const XOR: Op = Op::new('⊻');
pub(crate) const NAND: Op = Op::new('⊼');
// pub(crate) const NOR: Op = Op::new('⊽');
// pub(crate) const RIGHT_ANGLE_WITH_ARC: Op = Op::new('⊾');
// pub(crate) const RIGHT_TRIANGLE: Op = Op::new('⊿');
pub(crate) const N_ARY_LOGICAL_AND: Op = Op::new('⋀');
pub(crate) const N_ARY_LOGICAL_OR: Op = Op::new('⋁');
pub(crate) const N_ARY_INTERSECTION: Op = Op::new('⋂');
pub(crate) const N_ARY_UNION: Op = Op::new('⋃');
// pub(crate) const DIAMOND_OPERATOR: Op = Op::new('⋄');
// pub(crate) const DOT_OPERATOR: Op = Op::new('⋅');
pub(crate) const STAR_OPERATOR: Op = Op::new('⋆');
pub(crate) const DIVISION_TIMES: Op = Op::new('⋇');
pub(crate) const BOWTIE: Op = Op::new('⋈');
pub(crate) const LEFT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Op = Op::new('⋉');
pub(crate) const RIGHT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Op = Op::new('⋊');
pub(crate) const LEFT_SEMIDIRECT_PRODUCT: Op = Op::new('⋋');
pub(crate) const RIGHT_SEMIDIRECT_PRODUCT: Op = Op::new('⋌');
// pub(crate) const REVERSED_TILDE_EQUALS: Op = Op::new('⋍');
pub(crate) const CURLY_LOGICAL_OR: Op = Op::new('⋎');
pub(crate) const CURLY_LOGICAL_AND: Op = Op::new('⋏');
// pub(crate) const DOUBLE_SUBSET: Op = Op::new('⋐');
// pub(crate) const DOUBLE_SUPERSET: Op = Op::new('⋑');
pub(crate) const DOUBLE_INTERSECTION: Op = Op::new('⋒');
pub(crate) const DOUBLE_UNION: Op = Op::new('⋓');
// pub(crate) const PITCHFORK: Op = Op::new('⋔');
// pub(crate) const EQUAL_AND_PARALLEL_TO: Op = Op::new('⋕');
pub(crate) const LESS_THAN_WITH_DOT: Op = Op::new('⋖');
// pub(crate) const GREATER_THAN_WITH_DOT: Op = Op::new('⋗');
pub(crate) const VERY_MUCH_LESS_THAN: Op = Op::new('⋘');
// pub(crate) const VERY_MUCH_GREATER_THAN: Op = Op::new('⋙');
pub(crate) const LESS_THAN_EQUAL_TO_OR_GREATER_THAN: Op = Op::new('⋚');
// pub(crate) const GREATER_THAN_EQUAL_TO_OR_LESS_THAN: Op = Op::new('⋛');
// pub(crate) const EQUAL_TO_OR_LESS_THAN: Op = Op::new('⋜');
// pub(crate) const EQUAL_TO_OR_GREATER_THAN: Op = Op::new('⋝');
// pub(crate) const EQUAL_TO_OR_PRECEDES: Op = Op::new('⋞');
// pub(crate) const EQUAL_TO_OR_SUCCEEDS: Op = Op::new('⋟');
// pub(crate) const DOES_NOT_PRECEDE_OR_EQUAL: Op = Op::new('⋠');
// pub(crate) const DOES_NOT_SUCCEED_OR_EQUAL: Op = Op::new('⋡');
// pub(crate) const NOT_SQUARE_IMAGE_OF_OR_EQUAL_TO: Op = Op::new('⋢');
// pub(crate) const NOT_SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Op = Op::new('⋣');
// pub(crate) const SQUARE_IMAGE_OF_OR_NOT_EQUAL_TO: Op = Op::new('⋤');
// pub(crate) const SQUARE_ORIGINAL_OF_OR_NOT_EQUAL_TO: Op = Op::new('⋥');
// pub(crate) const LESS_THAN_BUT_NOT_EQUIVALENT_TO: Op = Op::new('⋦');
// pub(crate) const GREATER_THAN_BUT_NOT_EQUIVALENT_TO: Op = Op::new('⋧');
// pub(crate) const PRECEDES_BUT_NOT_EQUIVALENT_TO: Op = Op::new('⋨');
// pub(crate) const SUCCEEDS_BUT_NOT_EQUIVALENT_TO: Op = Op::new('⋩');
// pub(crate) const NOT_NORMAL_SUBGROUP_OF: Op = Op::new('⋪');
// pub(crate) const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP: Op = Op::new('⋫');
// pub(crate) const NOT_NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Op = Op::new('⋬');
// pub(crate) const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP_OR_EQUAL: Op = Op::new('⋭');
pub(crate) const VERTICAL_ELLIPSIS: Op = Op::new('⋮');
pub(crate) const MIDLINE_HORIZONTAL_ELLIPSIS: Op = Op::new('⋯');
// pub(crate) const UP_RIGHT_DIAGONAL_ELLIPSIS: Op = Op::new('⋰');
pub(crate) const DOWN_RIGHT_DIAGONAL_ELLIPSIS: Op = Op::new('⋱');
// pub(crate) const ELEMENT_OF_WITH_LONG_HORIZONTAL_STROKE: Op = Op::new('⋲');
// pub(crate) const ELEMENT_OF_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op::new('⋳');
// pub(crate) const SMALL_ELEMENT_OF_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op::new('⋴');
// pub(crate) const ELEMENT_OF_WITH_DOT_ABOVE: Op = Op::new('⋵');
// pub(crate) const ELEMENT_OF_WITH_OVERBAR: Op = Op::new('⋶');
// pub(crate) const SMALL_ELEMENT_OF_WITH_OVERBAR: Op = Op::new('⋷');
// pub(crate) const ELEMENT_OF_WITH_UNDERBAR: Op = Op::new('⋸');
// pub(crate) const ELEMENT_OF_WITH_TWO_HORIZONTAL_STROKES: Op = Op::new('⋹');
// pub(crate) const CONTAINS_WITH_LONG_HORIZONTAL_STROKE: Op = Op::new('⋺');
// pub(crate) const CONTAINS_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op::new('⋻');
// pub(crate) const SMALL_CONTAINS_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op::new('⋼');
// pub(crate) const CONTAINS_WITH_OVERBAR: Op = Op::new('⋽');
// pub(crate) const SMALL_CONTAINS_WITH_OVERBAR: Op = Op::new('⋾');
// pub(crate) const Z_NOTATION_BAG_MEMBERSHIP: Op = Op::new('⋿');

//
// Unicode Block: Miscellaneous Technical
//
pub(crate) const LEFT_CEILING: Op = Op::new('⌈');
pub(crate) const RIGHT_CEILING: Op = Op::new('⌉');
pub(crate) const LEFT_FLOOR: Op = Op::new('⌊');
pub(crate) const RIGHT_FLOOR: Op = Op::new('⌋');
pub(crate) const FROWN: Op = Op::new('⌢');
pub(crate) const SMILE: Op = Op::new('⌣');
pub(crate) const TOP_SQUARE_BRACKET: Op = Op::new('⎴');
pub(crate) const BOTTOM_SQUARE_BRACKET: Op = Op::new('⎵');
pub(crate) const TOP_PARENTHESIS: Op = Op::new('⏜');
pub(crate) const BOTTOM_PARENTHESIS: Op = Op::new('⏝');
pub(crate) const TOP_CURLY_BRACKET: Op = Op::new('⏞');
pub(crate) const BOTTOM_CURLY_BRACKET: Op = Op::new('⏟');

//
// Unicode Block: Geometric Shapes
//
pub(crate) const WHITE_UP_POINTING_TRIANGLE: Op = Op::new('△');
pub(crate) const WHITE_RIGHT_POINTING_TRIANGLE: Op = Op::new('▷');
pub(crate) const WHITE_DOWN_POINTING_TRIANGLE: Op = Op::new('▽');
pub(crate) const WHITE_LEFT_POINTING_TRIANGLE: Op = Op::new('◁');

pub(crate) const LARGE_CIRCLE: Op = Op::new('◯');

//
// Unicode Block: Miscellaneous Mathematical Symbols-A
//
pub(crate) const MATHEMATICAL_LEFT_WHITE_SQUARE_BRACKET: Op = Op::new('⟦');
pub(crate) const MATHEMATICAL_RIGHT_WHITE_SQUARE_BRACKET: Op = Op::new('⟧');
pub(crate) const MATHEMATICAL_LEFT_ANGLE_BRACKET: Op = Op::new('⟨');
pub(crate) const MATHEMATICAL_RIGHT_ANGLE_BRACKET: Op = Op::new('⟩');
pub(crate) const MATHEMATICAL_LEFT_FLATTENED_PARENTHESIS: Op = Op::new('⟮');
pub(crate) const MATHEMATICAL_RIGHT_FLATTENED_PARENTHESIS: Op = Op::new('⟯');

//
// Unicode Block: Supplemental Arrows-A
//
pub(crate) const LONG_LEFTWARDS_ARROW: Op = Op::new('⟵');
pub(crate) const LONG_RIGHTWARDS_ARROW: Op = Op::new('⟶');
pub(crate) const LONG_LEFT_RIGHT_ARROW: Op = Op::new('⟷');
pub(crate) const LONG_LEFTWARDS_DOUBLE_ARROW: Op = Op::new('⟸');
pub(crate) const LONG_RIGHTWARDS_DOUBLE_ARROW: Op = Op::new('⟹');
pub(crate) const LONG_LEFT_RIGHT_DOUBLE_ARROW: Op = Op::new('⟺');
// pub(crate) const LONG_LEFTWARDS_ARROW_FROM_BAR: Op = Op::new('⟻');
pub(crate) const LONG_RIGHTWARDS_ARROW_FROM_BAR: Op = Op::new('⟼');

//
// Unicode Block: Supplemental Arrows-B
//
pub(crate) const LEFTWARDS_ARROW_TAIL: Op = Op::new('⤙');
pub(crate) const RIGHTWARDS_ARROW_TAIL: Op = Op::new('⤚');

//
// Unicode Block: Miscellaneous Mathematical Symbols-B
//
pub(crate) const SQUARED_RISING_DIAGONAL_SLASH: Op = Op::new('⧄');
pub(crate) const SQUARED_FALLING_DIAGONAL_SLASH: Op = Op::new('⧅');
pub(crate) const SQUARED_SQUARE: Op = Op::new('⧈');

//
// Unicode Block: Supplemental Mathematical Operators
//
pub(crate) const N_ARY_CIRCLED_DOT_OPERATOR: Op = Op::new('⨀');
pub(crate) const N_ARY_CIRCLED_PLUS_OPERATOR: Op = Op::new('⨁');
pub(crate) const N_ARY_CIRCLED_TIMES_OPERATOR: Op = Op::new('⨂');
// pub(crate) const N_ARY_UNION_OPERATOR_WITH_DOT: Op = Op::new('⨃');
pub(crate) const N_ARY_UNION_OPERATOR_WITH_PLUS: Op = Op::new('⨄');
// pub(crate) const N_ARY_SQUARE_INTERSECTION_OPERATOR: Op = Op::new('⨅');
pub(crate) const N_ARY_SQUARE_UNION_OPERATOR: Op = Op::new('⨆');
pub(crate) const AMALGAMATION_OR_COPRODUCT: Op = Op::new('⨿');
pub(crate) const LESS_THAN_OR_SLANTED_EQUAL_TO: Op = Op::new('⩽');
pub(crate) const GREATER_THAN_OR_SLANTED_EQUAL_TO: Op = Op::new('⩾');
pub(crate) const LESS_THAN_OR_APPROXIMATE: Op = Op::new('⪅');
pub(crate) const GREATER_THAN_OR_APPROXIMATE: Op = Op::new('⪆');
pub(crate) const LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_GREATER_THAN: Op = Op::new('⪋');
pub(crate) const SLANTED_EQUAL_TO_OR_LESS_THAN: Op = Op::new('⪕');
pub(crate) const SLANTED_EQUAL_TO_OR_GREATER_THAN: Op = Op::new('⪖');
pub(crate) const PRECEDES_ABOVE_SINGLE_LINE_EQUALS_SIGN: Op = Op::new('⪯');
pub(crate) const SUCCEEDS_ABOVE_SINGLE_LINE_EQUALS_SIGN: Op = Op::new('⪰');

//
// Unicode Block: Small Form Variants
//
pub(crate) const SMALL_REVERSE_SOLIDUS: Op = Op::new('﹨');

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encode_utf8() {
        let greater_than_or_approximate = '⪆';
        let mut buf = [0u8; 3];
        let _ = greater_than_or_approximate.encode_utf8(&mut buf);

        let c = Utf8Char::new(greater_than_or_approximate);
        assert_eq!(c.as_str(), greater_than_or_approximate.to_string().as_str());
        if let Utf8Char::ThreeByte(array) = c {
            assert_eq!(buf, array)
        }
    }
}
