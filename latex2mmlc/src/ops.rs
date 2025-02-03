#[cfg(test)]
use serde::Serialize;

use crate::attribute::Stretchy;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(test, derive(Serialize))]
#[repr(transparent)]
pub struct Op(char);

impl From<Op> for char {
    #[inline]
    fn from(op: Op) -> Self {
        op.0
    }
}

impl From<&Op> for char {
    #[inline]
    fn from(op: &Op) -> Self {
        op.0
    }
}

/// List of characters and stretchy properties.
///
/// Each entry is a tuple consisting of:
/// 1. The character.
/// 2. Whether the character has ordinary spacing.
///    If this is true, the parenthesis behaves like a normal identifier
///    (which is different from an operator with reduced spacing!)
/// 3. The stretchy property.
static PAREN_OPS: [(char, bool, Stretchy); 27] = [
    ('\u{0}', false, Stretchy::Always),
    ('(', false, Stretchy::Always),
    (')', false, Stretchy::Always),
    ('/', true, Stretchy::Never),
    ('[', false, Stretchy::Always),
    ('\\', true, Stretchy::Never),
    (']', false, Stretchy::Always),
    ('{', false, Stretchy::Always),
    ('|', true, Stretchy::PrePostfix),
    ('}', false, Stretchy::Always),
    ('‖', true, Stretchy::PrePostfix),
    ('↑', false, Stretchy::Inconsistent),
    ('↓', false, Stretchy::Inconsistent),
    ('↕', false, Stretchy::Inconsistent),
    ('⇑', false, Stretchy::Inconsistent),
    ('⇓', false, Stretchy::Inconsistent),
    ('⇕', false, Stretchy::Inconsistent),
    ('⌈', false, Stretchy::Always),
    ('⌉', false, Stretchy::Always),
    ('⌊', false, Stretchy::Always),
    ('⌋', false, Stretchy::Always),
    ('⟦', false, Stretchy::Always),
    ('⟧', false, Stretchy::Always),
    ('⟨', false, Stretchy::Always),
    ('⟩', false, Stretchy::Always),
    ('⦇', false, Stretchy::Always),
    ('⦈', false, Stretchy::Always),
];

pub type ParenOp = &'static (char, bool, Stretchy);

//
// Unicode Block: Basic Latin
//
pub(crate) const NULL: ParenOp = &PAREN_OPS[0];
pub(crate) const EXCLAMATION_MARK: Op = Op('!');
// pub(crate) const APOSTROPHE: Op = Op('\'');
pub(crate) const LEFT_PARENTHESIS: ParenOp = &PAREN_OPS[1];
pub(crate) const RIGHT_PARENTHESIS: ParenOp = &PAREN_OPS[2];
pub(crate) const ASTERISK: Op = Op('*');
pub(crate) const PLUS_SIGN: Op = Op('+');
pub(crate) const COMMA: Op = Op(',');
pub(crate) const FULL_STOP: char = '.'; // not treated as operator
pub(crate) const SOLIDUS: ParenOp = &PAREN_OPS[3];
pub(crate) const COLON: Op = Op(':');
pub(crate) const SEMICOLON: Op = Op(';');
// pub(crate) const LESS_THAN_SIGN: Op = Op('<');
pub(crate) const EQUALS_SIGN: Op = Op('=');
// pub(crate) const GREATER_THAN_SIGN: Op = Op('>');
// pub(crate) const QUESTION_MARK: Op = Op('?');
pub(crate) const LEFT_SQUARE_BRACKET: ParenOp = &PAREN_OPS[4];
pub(crate) const REVERSE_SOLIDUS: ParenOp = &PAREN_OPS[5];
pub(crate) const RIGHT_SQUARE_BRACKET: ParenOp = &PAREN_OPS[6];
pub(crate) const CIRCUMFLEX_ACCENT: Op = Op('^');
pub(crate) const LOW_LINE: Op = Op('_');
pub(crate) const GRAVE_ACCENT: Op = Op('`');
pub(crate) const LEFT_CURLY_BRACKET: ParenOp = &PAREN_OPS[7];
pub(crate) const VERTICAL_LINE: ParenOp = &PAREN_OPS[8];
pub(crate) const RIGHT_CURLY_BRACKET: ParenOp = &PAREN_OPS[9];
pub(crate) const TILDE: Op = Op('~');

//
// Unicode Block: Latin-1 Supplement
//
pub(crate) const DIAERESIS: Op = Op('¨');
pub(crate) const NOT_SIGN: Op = Op('¬');
pub(crate) const MACRON: Op = Op('¯');
pub(crate) const PLUS_MINUS_SIGN: Op = Op('±');
pub(crate) const ACUTE_ACCENT: Op = Op('´');
pub(crate) const MIDDLE_DOT: Op = Op('·');
pub(crate) const MULTIPLICATION_SIGN: Op = Op('×');
pub(crate) const DIVISION_SIGN: Op = Op('÷');

//
// Unicode Block: Spacing Modifier Letters
//
pub(crate) const CARON: Op = Op('ˇ');
pub(crate) const BREVE: Op = Op('˘');
pub(crate) const DOT_ABOVE: Op = Op('˙');

//
// Unicode Block: General Punctuation
//
pub(crate) const DOUBLE_VERTICAL_LINE: ParenOp = &PAREN_OPS[10];
pub(crate) const HORIZONTAL_ELLIPSIS: Op = Op('…');
pub(crate) const PRIME: Op = Op('′');
pub(crate) const DOUBLE_PRIME: Op = Op('″');
pub(crate) const TRIPLE_PRIME: Op = Op('‴');
pub(crate) const REVERSED_PRIME: Op = Op('‵');
// pub(crate) const REVERSED_DOUBLE_PRIME: Op = Op('‶');
// pub(crate) const REVERSED_TRIPLE_PRIME: Op = Op('‷');
// pub(crate) const CARET: Op = Op('‸');
// pub(crate) const SINGLE_LEFT_POINTING_ANGLE_QUOTATION_MARK: Op = Op('‹');
// pub(crate) const SINGLE_RIGHT_POINTING_ANGLE_QUOTATION_MARK: Op = Op('›');
// pub(crate) const REFERENCE_MARK: Op = Op('※');
// pub(crate) const DOUBLE_EXCLAMATION_MARK: Op = Op('‼');
// pub(crate) const INTERROBANG: Op = Op('‽');
pub(crate) const OVERLINE: Op = Op('‾');

pub(crate) const QUADRUPLE_PRIME: Op = Op('⁗');

//
// Unicode Block: Arrows
//
pub(crate) const LEFTWARDS_ARROW: Op = Op('←');
pub(crate) const UPWARDS_ARROW: ParenOp = &PAREN_OPS[11];
pub(crate) const RIGHTWARDS_ARROW: Op = Op('→');
pub(crate) const DOWNWARDS_ARROW: ParenOp = &PAREN_OPS[12];
pub(crate) const LEFT_RIGHT_ARROW: Op = Op('↔');
pub(crate) const UP_DOWN_ARROW: ParenOp = &PAREN_OPS[13];
pub(crate) const NORTH_WEST_ARROW: Op = Op('↖');
pub(crate) const NORTH_EAST_ARROW: Op = Op('↗');
pub(crate) const SOUTH_EAST_ARROW: Op = Op('↘');
pub(crate) const SOUTH_WEST_ARROW: Op = Op('↙');
pub(crate) const LEFTWARDS_ARROW_WITH_STROKE: Op = Op('↚');
pub(crate) const RIGHTWARDS_ARROW_WITH_STROKE: Op = Op('↛');
// pub(crate) const LEFTWARDS_WAVE_ARROW: Op = Op('↜');
// pub(crate) const RIGHTWARDS_WAVE_ARROW: Op = Op('↝');
// pub(crate) const LEFTWARDS_TWO_HEADED_ARROW: Op = Op('↞');
// pub(crate) const UPWARDS_TWO_HEADED_ARROW: Op = Op('↟');
// pub(crate) const RIGHTWARDS_TWO_HEADED_ARROW: Op = Op('↠');
// pub(crate) const DOWNWARDS_TWO_HEADED_ARROW: Op = Op('↡');
pub(crate) const LEFTWARDS_ARROW_WITH_TAIL: Op = Op('↢');
pub(crate) const RIGHTWARDS_ARROW_WITH_TAIL: Op = Op('↣');
// pub(crate) const LEFTWARDS_ARROW_FROM_BAR: Op = Op('↤');
// pub(crate) const UPWARDS_ARROW_FROM_BAR: Op = Op('↥');
pub(crate) const RIGHTWARDS_ARROW_FROM_BAR: Op = Op('↦');
// pub(crate) const DOWNWARDS_ARROW_FROM_BAR: Op = Op('↧');
// pub(crate) const UP_DOWN_ARROW_WITH_BASE: Op = Op('↨');
pub(crate) const LEFTWARDS_ARROW_WITH_HOOK: Op = Op('↩');
pub(crate) const RIGHTWARDS_ARROW_WITH_HOOK: Op = Op('↪');
pub(crate) const LEFTWARDS_ARROW_WITH_LOOP: Op = Op('↫');
pub(crate) const RIGHTWARDS_ARROW_WITH_LOOP: Op = Op('↬');
pub(crate) const LEFT_RIGHT_WAVE_ARROW: Op = Op('↭');
pub(crate) const LEFT_RIGHT_ARROW_WITH_STROKE: Op = Op('↮');
pub(crate) const DOWNWARDS_ZIGZAG_ARROW: Op = Op('↯');
pub(crate) const UPWARDS_ARROW_WITH_TIP_LEFTWARDS: Op = Op('↰');
pub(crate) const UPWARDS_ARROW_WITH_TIP_RIGHTWARDS: Op = Op('↱');
// pub(crate) const DOWNWARDS_ARROW_WITH_TIP_LEFTWARDS: Op = Op('↲');
// pub(crate) const DOWNWARDS_ARROW_WITH_TIP_RIGHTWARDS: Op = Op('↳');
// pub(crate) const RIGHTWARDS_ARROW_WITH_CORNER_DOWNWARDS: Op = Op('↴');
// pub(crate) const DOWNWARDS_ARROW_WITH_CORNER_LEFTWARDS: Op = Op('↵');
pub(crate) const ANTICLOCKWISE_TOP_SEMICIRCLE_ARROW: Op = Op('↶');
pub(crate) const CLOCKWISE_TOP_SEMICIRCLE_ARROW: Op = Op('↷');
// pub(crate) const NORTH_WEST_ARROW_TO_LONG_BAR: Op = Op('↸');
// pub(crate) const LEFTWARDS_ARROW_TO_BAR_OVER_RIGHTWARDS_ARROW_TO_BAR: Op = Op('↹');
pub(crate) const ANTICLOCKWISE_OPEN_CIRCLE_ARROW: Op = Op('↺');
pub(crate) const CLOCKWISE_OPEN_CIRCLE_ARROW: Op = Op('↻');
pub(crate) const LEFTWARDS_HARPOON_WITH_BARB_UPWARDS: Op = Op('↼');
pub(crate) const LEFTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Op = Op('↽');
pub(crate) const UPWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Op = Op('↾');
pub(crate) const UPWARDS_HARPOON_WITH_BARB_LEFTWARDS: Op = Op('↿');
pub(crate) const RIGHTWARDS_HARPOON_WITH_BARB_UPWARDS: Op = Op('⇀');
pub(crate) const RIGHTWARDS_HARPOON_WITH_BARB_DOWNWARDS: Op = Op('⇁');
pub(crate) const DOWNWARDS_HARPOON_WITH_BARB_RIGHTWARDS: Op = Op('⇂');
pub(crate) const DOWNWARDS_HARPOON_WITH_BARB_LEFTWARDS: Op = Op('⇃');
pub(crate) const RIGHTWARDS_ARROW_OVER_LEFTWARDS_ARROW: Op = Op('⇄');
// pub(crate) const UPWARDS_ARROW_LEFTWARDS_OF_DOWNWARDS_ARROW: Op = Op('⇅');
pub(crate) const LEFTWARDS_ARROW_OVER_RIGHTWARDS_ARROW: Op = Op('⇆');
pub(crate) const LEFTWARDS_PAIRED_ARROWS: Op = Op('⇇');
pub(crate) const UPWARDS_PAIRED_ARROWS: Op = Op('⇈');
pub(crate) const RIGHTWARDS_PAIRED_ARROWS: Op = Op('⇉');
pub(crate) const DOWNWARDS_PAIRED_ARROWS: Op = Op('⇊');
pub(crate) const LEFTWARDS_HARPOON_OVER_RIGHTWARDS_HARPOON: Op = Op('⇋');
pub(crate) const RIGHTWARDS_HARPOON_OVER_LEFTWARDS_HARPOON: Op = Op('⇌');
pub(crate) const LEFTWARDS_DOUBLE_ARROW_WITH_STROKE: Op = Op('⇍');
pub(crate) const LEFT_RIGHT_DOUBLE_ARROW_WITH_STROKE: Op = Op('⇎');
pub(crate) const RIGHTWARDS_DOUBLE_ARROW_WITH_STROKE: Op = Op('⇏');
pub(crate) const LEFTWARDS_DOUBLE_ARROW: Op = Op('⇐');
pub(crate) const UPWARDS_DOUBLE_ARROW: ParenOp = &PAREN_OPS[14];
pub(crate) const RIGHTWARDS_DOUBLE_ARROW: Op = Op('⇒');
pub(crate) const DOWNWARDS_DOUBLE_ARROW: ParenOp = &PAREN_OPS[15];
pub(crate) const LEFT_RIGHT_DOUBLE_ARROW: Op = Op('⇔');
pub(crate) const UP_DOWN_DOUBLE_ARROW: ParenOp = &PAREN_OPS[16];
// pub(crate) const NORTH_WEST_DOUBLE_ARROW: Op = Op('⇖');
// pub(crate) const NORTH_EAST_DOUBLE_ARROW: Op = Op('⇗');
// pub(crate) const SOUTH_EAST_DOUBLE_ARROW: Op = Op('⇘');
// pub(crate) const SOUTH_WEST_DOUBLE_ARROW: Op = Op('⇙');
pub(crate) const LEFTWARDS_TRIPLE_ARROW: Op = Op('⇚');
pub(crate) const RIGHTWARDS_TRIPLE_ARROW: Op = Op('⇛');
// pub(crate) const LEFTWARDS_SQUIGGLE_ARROW: Op = Op('⇜');
pub(crate) const RIGHTWARDS_SQUIGGLE_ARROW: Op = Op('⇝');
// pub(crate) const UPWARDS_ARROW_WITH_DOUBLE_STROKE: Op = Op('⇞');
// pub(crate) const DOWNWARDS_ARROW_WITH_DOUBLE_STROKE: Op = Op('⇟');
// pub(crate) const LEFTWARDS_DASHED_ARROW: Op = Op('⇠');
// pub(crate) const UPWARDS_DASHED_ARROW: Op = Op('⇡');
// pub(crate) const RIGHTWARDS_DASHED_ARROW: Op = Op('⇢');
// pub(crate) const DOWNWARDS_DASHED_ARROW: Op = Op('⇣');
// pub(crate) const LEFTWARDS_ARROW_TO_BAR: Op = Op('⇤');
// pub(crate) const RIGHTWARDS_ARROW_TO_BAR: Op = Op('⇥');
// pub(crate) const LEFTWARDS_WHITE_ARROW: Op = Op('⇦');
// pub(crate) const UPWARDS_WHITE_ARROW: Op = Op('⇧');
// pub(crate) const RIGHTWARDS_WHITE_ARROW: Op = Op('⇨');
// pub(crate) const DOWNWARDS_WHITE_ARROW: Op = Op('⇩');
// pub(crate) const UPWARDS_WHITE_ARROW_FROM_BAR: Op = Op('⇪');
// pub(crate) const UPWARDS_WHITE_ARROW_ON_PEDESTAL: Op = Op('⇫');
// pub(crate) const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_HORIZONTAL_BAR: Op = Op('⇬');
// pub(crate) const UPWARDS_WHITE_ARROW_ON_PEDESTAL_WITH_VERTICAL_BAR: Op = Op('⇭');
// pub(crate) const UPWARDS_WHITE_DOUBLE_ARROW: Op = Op('⇮');
// pub(crate) const UPWARDS_WHITE_DOUBLE_ARROW_ON_PEDESTAL: Op = Op('⇯');
// pub(crate) const RIGHTWARDS_WHITE_ARROW_FROM_WALL: Op = Op('⇰');
// pub(crate) const NORTH_WEST_ARROW_TO_CORNER: Op = Op('⇱');
// pub(crate) const SOUTH_EAST_ARROW_TO_CORNER: Op = Op('⇲');
// pub(crate) const UP_DOWN_WHITE_ARROW: Op = Op('⇳');
// pub(crate) const RIGHT_ARROW_WITH_SMALL_CIRCLE: Op = Op('⇴');
// pub(crate) const DOWNWARDS_ARROW_LEFTWARDS_OF_UPWARDS_ARROW: Op = Op('⇵');
// pub(crate) const THREE_RIGHTWARDS_ARROWS: Op = Op('⇶');
// pub(crate) const LEFTWARDS_ARROW_WITH_VERTICAL_STROKE: Op = Op('⇷');
// pub(crate) const RIGHTWARDS_ARROW_WITH_VERTICAL_STROKE: Op = Op('⇸');
// pub(crate) const LEFT_RIGHT_ARROW_WITH_VERTICAL_STROKE: Op = Op('⇹');
// pub(crate) const LEFTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Op = Op('⇺');
// pub(crate) const RIGHTWARDS_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Op = Op('⇻');
// pub(crate) const LEFT_RIGHT_ARROW_WITH_DOUBLE_VERTICAL_STROKE: Op = Op('⇼');
// pub(crate) const LEFTWARDS_OPEN_HEADED_ARROW: Op = Op('⇽');
// pub(crate) const RIGHTWARDS_OPEN_HEADED_ARROW: Op = Op('⇾');
// pub(crate) const LEFT_RIGHT_OPEN_HEADED_ARROW: Op = Op('⇿');

//
// Unicode Block: Mathematical Operators
//
pub(crate) const FOR_ALL: Op = Op('∀');
pub(crate) const COMPLEMENT: char = '∁'; // not treated as operator
pub(crate) const PARTIAL_DIFFERENTIAL: char = '∂'; // not treated as operator
pub(crate) const THERE_EXISTS: Op = Op('∃');
pub(crate) const THERE_DOES_NOT_EXIST: Op = Op('∄');
pub(crate) const EMPTY_SET: char = '∅';
// pub(crate) const INCREMENT: Op = Op('∆');
pub(crate) const NABLA: char = '∇'; // not treated as operator
pub(crate) const ELEMENT_OF: Op = Op('∈');
pub(crate) const NOT_AN_ELEMENT_OF: Op = Op('∉');
// pub(crate) const SMALL_ELEMENT_OF: Op = Op('∊');
pub(crate) const CONTAINS_AS_MEMBER: Op = Op('∋');
// pub(crate) const DOES_NOT_CONTAIN_AS_MEMBER: Op = Op('∌');
pub(crate) const SMALL_CONTAINS_AS_MEMBER: Op = Op('∍');
// pub(crate) const END_OF_PROOF: Op = Op('∎');
pub(crate) const N_ARY_PRODUCT: Op = Op('∏');
pub(crate) const N_ARY_COPRODUCT: Op = Op('∐');
pub(crate) const N_ARY_SUMMATION: Op = Op('∑');
pub(crate) const MINUS_SIGN: Op = Op('−');
pub(crate) const MINUS_OR_PLUS_SIGN: Op = Op('∓');
pub(crate) const DOT_PLUS: Op = Op('∔');
// pub(crate) const DIVISION_SLASH: Op = Op('∕');
pub(crate) const SET_MINUS: Op = Op('∖');
pub(crate) const ASTERISK_OPERATOR: Op = Op('∗');
pub(crate) const RING_OPERATOR: Op = Op('∘');
pub(crate) const BULLET_OPERATOR: Op = Op('∙');
// pub(crate) const SQUARE_ROOT: Op = Op('√');
// pub(crate) const CUBE_ROOT: Op = Op('∛');
// pub(crate) const FOURTH_ROOT: Op = Op('∜');
pub(crate) const PROPORTIONAL_TO: Op = Op('∝');
pub(crate) const INFINITY: char = '∞';
// pub(crate) const RIGHT_ANGLE: Op = Op('∟');
pub(crate) const ANGLE: char = '∠';
pub(crate) const MEASURED_ANGLE: char = '∡';
pub(crate) const SPHERICAL_ANGLE: char = '∢';
pub(crate) const DIVIDES: Op = Op('∣');
pub(crate) const DOES_NOT_DIVIDE: Op = Op('∤');
pub(crate) const PARALLEL_TO: Op = Op('∥');
pub(crate) const NOT_PARALLEL_TO: Op = Op('∦');
pub(crate) const LOGICAL_AND: Op = Op('∧');
pub(crate) const LOGICAL_OR: Op = Op('∨');
pub(crate) const INTERSECTION: Op = Op('∩');
pub(crate) const UNION: Op = Op('∪');
pub(crate) const INTEGRAL: Op = Op('∫');
pub(crate) const DOUBLE_INTEGRAL: Op = Op('∬');
pub(crate) const TRIPLE_INTEGRAL: Op = Op('∭');
pub(crate) const CONTOUR_INTEGRAL: Op = Op('∮');
pub(crate) const SURFACE_INTEGRAL: Op = Op('∯');
pub(crate) const VOLUME_INTEGRAL: Op = Op('∰');
pub(crate) const CLOCKWISE_INTEGRAL: Op = Op('∱');
pub(crate) const CLOCKWISE_CONTOUR_INTEGRAL: Op = Op('∲');
pub(crate) const ANTICLOCKWISE_CONTOUR_INTEGRAL: Op = Op('∳');
pub(crate) const THEREFORE: Op = Op('∴');
pub(crate) const BECAUSE: Op = Op('∵');
// pub(crate) const RATIO: Op = Op('∶');
pub(crate) const PROPORTION: Op = Op('∷');
// pub(crate) const DOT_MINUS: Op = Op('∸');
pub(crate) const EXCESS: Op = Op('∹');
pub(crate) const GEOMETRIC_PROPORTION: Op = Op('∺');
pub(crate) const HOMOTHETIC: Op = Op('∻');
pub(crate) const TILDE_OPERATOR: Op = Op('∼');
pub(crate) const REVERSED_TILDE: Op = Op('∽');
// pub(crate) const INVERTED_LAZY_S: Op = Op('∾');
// pub(crate) const SINE_WAVE: Op = Op('∿');
pub(crate) const WREATH_PRODUCT: Op = Op('≀');
pub(crate) const NOT_TILDE: Op = Op('≁');
pub(crate) const MINUS_TILDE: Op = Op('≂');
pub(crate) const ASYMPTOTICALLY_EQUAL_TO: Op = Op('≃');
pub(crate) const NOT_ASYMPTOTICALLY_EQUAL_TO: Op = Op('≄');
pub(crate) const APPROXIMATELY_EQUAL_TO: Op = Op('≅');
// pub(crate) const APPROXIMATELY_BUT_NOT_ACTUALLY_EQUAL_TO: Op = Op('≆');
// pub(crate) const NEITHER_APPROXIMATELY_NOR_ACTUALLY_EQUAL_TO: Op = Op('≇');
pub(crate) const ALMOST_EQUAL_TO: Op = Op('≈');
pub(crate) const NOT_ALMOST_EQUAL_TO: Op = Op('≉');
pub(crate) const ALMOST_EQUAL_OR_EQUAL_TO: Op = Op('≊');
// pub(crate) const TRIPLE_TILDE: Op = Op('≋');
// pub(crate) const ALL_EQUAL_TO: Op = Op('≌');
pub(crate) const EQUIVALENT_TO: Op = Op('≍');
pub(crate) const GEOMETRICALLY_EQUIVALENT_TO: Op = Op('≎');
pub(crate) const DIFFERENCE_BETWEEN: Op = Op('≏');
pub(crate) const APPROACHES_THE_LIMIT: Op = Op('≐');
pub(crate) const GEOMETRICALLY_EQUAL_TO: Op = Op('≑');
pub(crate) const APPROXIMATELY_EQUAL_TO_OR_THE_IMAGE_OF: Op = Op('≒');
pub(crate) const IMAGE_OF_OR_APPROXIMATELY_EQUAL_TO: Op = Op('≓');
pub(crate) const COLON_EQUALS: Op = Op('≔');
pub(crate) const EQUALS_COLON: Op = Op('≕');
pub(crate) const RING_IN_EQUAL_TO: Op = Op('≖');
pub(crate) const RING_EQUAL_TO: Op = Op('≗');
pub(crate) const CORRESPONDS_TO: Op = Op('≘');
pub(crate) const ESTIMATES: Op = Op('≙');
pub(crate) const EQUIANGULAR_TO: Op = Op('≚');
pub(crate) const STAR_EQUALS: Op = Op('≛');
pub(crate) const DELTA_EQUAL_TO: Op = Op('≜');
pub(crate) const EQUAL_TO_BY_DEFINITION: Op = Op('≝');
pub(crate) const MEASURED_BY: Op = Op('≞');
pub(crate) const QUESTIONED_EQUAL_TO: Op = Op('≟');
pub(crate) const NOT_EQUAL_TO: Op = Op('≠');
pub(crate) const IDENTICAL_TO: Op = Op('≡');
pub(crate) const NOT_IDENTICAL_TO: Op = Op('≢');
// pub(crate) const STRICTLY_EQUIVALENT_TO: Op = Op('≣');
pub(crate) const LESS_THAN_OR_EQUAL_TO: Op = Op('≤');
pub(crate) const GREATER_THAN_OR_EQUAL_TO: Op = Op('≥');
pub(crate) const LESS_THAN_OVER_EQUAL_TO: Op = Op('≦');
pub(crate) const GREATER_THAN_OVER_EQUAL_TO: Op = Op('≧');
pub(crate) const LESS_THAN_BUT_NOT_EQUAL_TO: Op = Op('≨');
pub(crate) const GREATER_THAN_BUT_NOT_EQUAL_TO: Op = Op('≩');
pub(crate) const MUCH_LESS_THAN: Op = Op('≪');
pub(crate) const MUCH_GREATER_THAN: Op = Op('≫');
// pub(crate) const BETWEEN: Op = Op('≬');
// pub(crate) const NOT_EQUIVALENT_TO: Op = Op('≭');
pub(crate) const NOT_LESS_THAN: Op = Op('≮');
pub(crate) const NOT_GREATER_THAN: Op = Op('≯');
pub(crate) const NEITHER_LESS_THAN_NOR_EQUAL_TO: Op = Op('≰');
pub(crate) const NEITHER_GREATER_THAN_NOR_EQUAL_TO: Op = Op('≱');
pub(crate) const LESS_THAN_OR_EQUIVALENT_TO: Op = Op('≲');
pub(crate) const GREATER_THAN_OR_EQUIVALENT_TO: Op = Op('≳');
pub(crate) const NEITHER_LESS_THAN_NOR_EQUIVALENT_TO: Op = Op('≴');
pub(crate) const NEITHER_GREATER_THAN_NOR_EQUIVALENT_TO: Op = Op('≵');
pub(crate) const LESS_THAN_OR_GREATER_THAN: Op = Op('≶');
pub(crate) const GREATER_THAN_OR_LESS_THAN: Op = Op('≷');
pub(crate) const NEITHER_LESS_THAN_NOR_GREATER_THAN: Op = Op('≸');
pub(crate) const NEITHER_GREATER_THAN_NOR_LESS_THAN: Op = Op('≹');
pub(crate) const PRECEDES: Op = Op('≺');
pub(crate) const SUCCEEDS: Op = Op('≻');
pub(crate) const PRECEDES_OR_EQUAL_TO: Op = Op('≼');
pub(crate) const SUCCEEDS_OR_EQUAL_TO: Op = Op('≽');
pub(crate) const PRECEDES_OR_EQUIVALENT_TO: Op = Op('≾');
pub(crate) const SUCCEEDS_OR_EQUIVALENT_TO: Op = Op('≿');
pub(crate) const DOES_NOT_PRECEDE: Op = Op('⊀');
pub(crate) const DOES_NOT_SUCCEED: Op = Op('⊁');
pub(crate) const SUBSET_OF: Op = Op('⊂');
pub(crate) const SUPERSET_OF: Op = Op('⊃');
pub(crate) const NOT_A_SUBSET_OF: Op = Op('⊄');
pub(crate) const NOT_A_SUPERSET_OF: Op = Op('⊅');
pub(crate) const SUBSET_OF_OR_EQUAL_TO: Op = Op('⊆');
pub(crate) const SUPERSET_OF_OR_EQUAL_TO: Op = Op('⊇');
pub(crate) const NEITHER_A_SUBSET_OF_NOR_EQUAL_TO: Op = Op('⊈');
pub(crate) const NEITHER_A_SUPERSET_OF_NOR_EQUAL_TO: Op = Op('⊉');
pub(crate) const SUBSET_OF_WITH_NOT_EQUAL_TO: Op = Op('⊊');
pub(crate) const SUPERSET_OF_WITH_NOT_EQUAL_TO: Op = Op('⊋');
// pub(crate) const MULTISET: Op = Op('⊌');
// pub(crate) const MULTISET_MULTIPLICATION: Op = Op('⊍');
pub(crate) const MULTISET_UNION: Op = Op('⊎');
pub(crate) const SQUARE_IMAGE_OF: Op = Op('⊏');
pub(crate) const SQUARE_ORIGINAL_OF: Op = Op('⊐');
pub(crate) const SQUARE_IMAGE_OF_OR_EQUAL_TO: Op = Op('⊑');
pub(crate) const SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Op = Op('⊒');
pub(crate) const SQUARE_CAP: Op = Op('⊓');
pub(crate) const SQUARE_CUP: Op = Op('⊔');
pub(crate) const CIRCLED_PLUS: Op = Op('⊕');
pub(crate) const CIRCLED_MINUS: Op = Op('⊖');
pub(crate) const CIRCLED_TIMES: Op = Op('⊗');
pub(crate) const CIRCLED_DIVISION_SLASH: Op = Op('⊘');
pub(crate) const CIRCLED_DOT_OPERATOR: Op = Op('⊙');
pub(crate) const CIRCLED_RING_OPERATOR: Op = Op('⊚');
pub(crate) const CIRCLED_ASTERISK_OPERATOR: Op = Op('⊛');
// pub(crate) const CIRCLED_EQUALS: Op = Op('⊜');
pub(crate) const CIRCLED_DASH: Op = Op('⊝');
pub(crate) const SQUARED_PLUS: Op = Op('⊞');
pub(crate) const SQUARED_MINUS: Op = Op('⊟');
pub(crate) const SQUARED_TIMES: Op = Op('⊠');
pub(crate) const SQUARED_DOT_OPERATOR: Op = Op('⊡');
pub(crate) const RIGHT_TACK: Op = Op('⊢');
pub(crate) const LEFT_TACK: Op = Op('⊣');
pub(crate) const DOWN_TACK: Op = Op('⊤');
pub(crate) const UP_TACK: Op = Op('⊥');
// pub(crate) const ASSERTION: Op = Op('⊦');
// pub(crate) const MODELS: Op = Op('⊧');
pub(crate) const TRUE: Op = Op('⊨');
pub(crate) const FORCES: Op = Op('⊩');
// pub(crate) const TRIPLE_VERTICAL_BAR_RIGHT_TURNSTILE: Op = Op('⊪');
// pub(crate) const DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Op = Op('⊫');
// pub(crate) const DOES_NOT_PROVE: Op = Op('⊬');
// pub(crate) const NOT_TRUE: Op = Op('⊭');
// pub(crate) const DOES_NOT_FORCE: Op = Op('⊮');
// pub(crate) const NEGATED_DOUBLE_VERTICAL_BAR_DOUBLE_RIGHT_TURNSTILE: Op = Op('⊯');
// pub(crate) const PRECEDES_UNDER_RELATION: Op = Op('⊰');
// pub(crate) const SUCCEEDS_UNDER_RELATION: Op = Op('⊱');
pub(crate) const NORMAL_SUBGROUP_OF: Op = Op('⊲');
pub(crate) const CONTAINS_AS_NORMAL_SUBGROUP: Op = Op('⊳');
pub(crate) const NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Op = Op('⊴');
pub(crate) const CONTAINS_AS_NORMAL_SUBGROUP_OR_EQUAL_TO: Op = Op('⊵');
// pub(crate) const ORIGINAL_OF: Op = Op('⊶');
// pub(crate) const IMAGE_OF: Op = Op('⊷');
pub(crate) const MULTIMAP: Op = Op('⊸');
// pub(crate) const HERMITIAN_CONJUGATE_MATRIX: Op = Op('⊹');
pub(crate) const INTERCALATE: Op = Op('⊺');
pub(crate) const XOR: Op = Op('⊻');
pub(crate) const NAND: Op = Op('⊼');
// pub(crate) const NOR: Op = Op('⊽');
// pub(crate) const RIGHT_ANGLE_WITH_ARC: Op = Op('⊾');
// pub(crate) const RIGHT_TRIANGLE: Op = Op('⊿');
pub(crate) const N_ARY_LOGICAL_AND: Op = Op('⋀');
pub(crate) const N_ARY_LOGICAL_OR: Op = Op('⋁');
pub(crate) const N_ARY_INTERSECTION: Op = Op('⋂');
pub(crate) const N_ARY_UNION: Op = Op('⋃');
pub(crate) const DIAMOND_OPERATOR: Op = Op('⋄');
// pub(crate) const DOT_OPERATOR: Op = Op('⋅');
pub(crate) const STAR_OPERATOR: Op = Op('⋆');
pub(crate) const DIVISION_TIMES: Op = Op('⋇');
pub(crate) const BOWTIE: Op = Op('⋈');
pub(crate) const LEFT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Op = Op('⋉');
pub(crate) const RIGHT_NORMAL_FACTOR_SEMIDIRECT_PRODUCT: Op = Op('⋊');
pub(crate) const LEFT_SEMIDIRECT_PRODUCT: Op = Op('⋋');
pub(crate) const RIGHT_SEMIDIRECT_PRODUCT: Op = Op('⋌');
pub(crate) const REVERSED_TILDE_EQUALS: Op = Op('⋍');
pub(crate) const CURLY_LOGICAL_OR: Op = Op('⋎');
pub(crate) const CURLY_LOGICAL_AND: Op = Op('⋏');
pub(crate) const DOUBLE_SUBSET: Op = Op('⋐');
pub(crate) const DOUBLE_SUPERSET: Op = Op('⋑');
pub(crate) const DOUBLE_INTERSECTION: Op = Op('⋒');
pub(crate) const DOUBLE_UNION: Op = Op('⋓');
// pub(crate) const PITCHFORK: Op = Op('⋔');
// pub(crate) const EQUAL_AND_PARALLEL_TO: Op = Op('⋕');
pub(crate) const LESS_THAN_WITH_DOT: Op = Op('⋖');
// pub(crate) const GREATER_THAN_WITH_DOT: Op = Op('⋗');
pub(crate) const VERY_MUCH_LESS_THAN: Op = Op('⋘');
// pub(crate) const VERY_MUCH_GREATER_THAN: Op = Op('⋙');
pub(crate) const LESS_THAN_EQUAL_TO_OR_GREATER_THAN: Op = Op('⋚');
// pub(crate) const GREATER_THAN_EQUAL_TO_OR_LESS_THAN: Op = Op('⋛');
// pub(crate) const EQUAL_TO_OR_LESS_THAN: Op = Op('⋜');
// pub(crate) const EQUAL_TO_OR_GREATER_THAN: Op = Op('⋝');
pub(crate) const EQUAL_TO_OR_PRECEDES: Op = Op('⋞');
pub(crate) const EQUAL_TO_OR_SUCCEEDS: Op = Op('⋟');
pub(crate) const DOES_NOT_PRECEDE_OR_EQUAL: Op = Op('⋠');
pub(crate) const DOES_NOT_SUCCEED_OR_EQUAL: Op = Op('⋡');
// pub(crate) const NOT_SQUARE_IMAGE_OF_OR_EQUAL_TO: Op = Op('⋢');
// pub(crate) const NOT_SQUARE_ORIGINAL_OF_OR_EQUAL_TO: Op = Op('⋣');
// pub(crate) const SQUARE_IMAGE_OF_OR_NOT_EQUAL_TO: Op = Op('⋤');
// pub(crate) const SQUARE_ORIGINAL_OF_OR_NOT_EQUAL_TO: Op = Op('⋥');
// pub(crate) const LESS_THAN_BUT_NOT_EQUIVALENT_TO: Op = Op('⋦');
// pub(crate) const GREATER_THAN_BUT_NOT_EQUIVALENT_TO: Op = Op('⋧');
pub(crate) const PRECEDES_BUT_NOT_EQUIVALENT_TO: Op = Op('⋨');
pub(crate) const SUCCEEDS_BUT_NOT_EQUIVALENT_TO: Op = Op('⋩');
// pub(crate) const NOT_NORMAL_SUBGROUP_OF: Op = Op('⋪');
// pub(crate) const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP: Op = Op('⋫');
// pub(crate) const NOT_NORMAL_SUBGROUP_OF_OR_EQUAL_TO: Op = Op('⋬');
// pub(crate) const DOES_NOT_CONTAIN_AS_NORMAL_SUBGROUP_OR_EQUAL: Op = Op('⋭');
pub(crate) const VERTICAL_ELLIPSIS: Op = Op('⋮');
pub(crate) const MIDLINE_HORIZONTAL_ELLIPSIS: Op = Op('⋯');
// pub(crate) const UP_RIGHT_DIAGONAL_ELLIPSIS: Op = Op('⋰');
pub(crate) const DOWN_RIGHT_DIAGONAL_ELLIPSIS: Op = Op('⋱');
// pub(crate) const ELEMENT_OF_WITH_LONG_HORIZONTAL_STROKE: Op = Op('⋲');
// pub(crate) const ELEMENT_OF_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋳');
// pub(crate) const SMALL_ELEMENT_OF_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋴');
// pub(crate) const ELEMENT_OF_WITH_DOT_ABOVE: Op = Op('⋵');
// pub(crate) const ELEMENT_OF_WITH_OVERBAR: Op = Op('⋶');
// pub(crate) const SMALL_ELEMENT_OF_WITH_OVERBAR: Op = Op('⋷');
// pub(crate) const ELEMENT_OF_WITH_UNDERBAR: Op = Op('⋸');
// pub(crate) const ELEMENT_OF_WITH_TWO_HORIZONTAL_STROKES: Op = Op('⋹');
// pub(crate) const CONTAINS_WITH_LONG_HORIZONTAL_STROKE: Op = Op('⋺');
// pub(crate) const CONTAINS_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋻');
// pub(crate) const SMALL_CONTAINS_WITH_VERTICAL_BAR_AT_END_OF_HORIZONTAL_STROKE: Op = Op('⋼');
// pub(crate) const CONTAINS_WITH_OVERBAR: Op = Op('⋽');
// pub(crate) const SMALL_CONTAINS_WITH_OVERBAR: Op = Op('⋾');
// pub(crate) const Z_NOTATION_BAG_MEMBERSHIP: Op = Op('⋿');

//
// Unicode Block: Miscellaneous Technical
//
pub(crate) const LEFT_CEILING: ParenOp = &PAREN_OPS[17];
pub(crate) const RIGHT_CEILING: ParenOp = &PAREN_OPS[18];
pub(crate) const LEFT_FLOOR: ParenOp = &PAREN_OPS[19];
pub(crate) const RIGHT_FLOOR: ParenOp = &PAREN_OPS[20];
pub(crate) const TOP_LEFT_CORNER: char = '⌜';
pub(crate) const TOP_RIGHT_CORNER: char = '⌝';
pub(crate) const BOTTOM_LEFT_CORNER: char = '⌞';
pub(crate) const BOTTOM_RIGHT_CORNER: char = '⌟';
pub(crate) const FROWN: Op = Op('⌢');
pub(crate) const SMILE: Op = Op('⌣');
pub(crate) const TOP_SQUARE_BRACKET: Op = Op('⎴');
pub(crate) const BOTTOM_SQUARE_BRACKET: Op = Op('⎵');
pub(crate) const TOP_PARENTHESIS: Op = Op('⏜');
pub(crate) const BOTTOM_PARENTHESIS: Op = Op('⏝');
pub(crate) const TOP_CURLY_BRACKET: Op = Op('⏞');
pub(crate) const BOTTOM_CURLY_BRACKET: Op = Op('⏟');

//
// Unicode Block: Enclosed Alphanumerics
//
pub(crate) const CIRCLED_LATIN_CAPITAL_LETTER_R: char = 'Ⓡ'; // not treated as operator
pub(crate) const CIRCLED_LATIN_CAPITAL_LETTER_S: char = 'Ⓢ'; // not treated as operator

//
// Unicode Block: Geometric Shapes
//
pub(crate) const BLACK_SQUARE: char = '■';

pub(crate) const WHITE_UP_POINTING_TRIANGLE: Op = Op('△');
pub(crate) const WHITE_RIGHT_POINTING_TRIANGLE: Op = Op('▷');
pub(crate) const WHITE_DOWN_POINTING_TRIANGLE: Op = Op('▽');
pub(crate) const WHITE_LEFT_POINTING_TRIANGLE: Op = Op('◁');

pub(crate) const LARGE_CIRCLE: Op = Op('◯');

//
// Unicode Block: Miscellaneous Symbols
//
pub(crate) const BLACK_STAR: char = '★';

//
// Unicode Block: Miscellaneous Mathematical Symbols-A
//
pub(crate) const MATHEMATICAL_LEFT_WHITE_SQUARE_BRACKET: ParenOp = &PAREN_OPS[21];
pub(crate) const MATHEMATICAL_RIGHT_WHITE_SQUARE_BRACKET: ParenOp = &PAREN_OPS[22];
pub(crate) const MATHEMATICAL_LEFT_ANGLE_BRACKET: ParenOp = &PAREN_OPS[23];
pub(crate) const MATHEMATICAL_RIGHT_ANGLE_BRACKET: ParenOp = &PAREN_OPS[24];
pub(crate) const MATHEMATICAL_LEFT_FLATTENED_PARENTHESIS: ParenOp = &PAREN_OPS[25];
pub(crate) const MATHEMATICAL_RIGHT_FLATTENED_PARENTHESIS: ParenOp = &PAREN_OPS[26];

//
// Unicode Block: Supplemental Arrows-A
//
pub(crate) const LONG_LEFTWARDS_ARROW: Op = Op('⟵');
pub(crate) const LONG_RIGHTWARDS_ARROW: Op = Op('⟶');
pub(crate) const LONG_LEFT_RIGHT_ARROW: Op = Op('⟷');
pub(crate) const LONG_LEFTWARDS_DOUBLE_ARROW: Op = Op('⟸');
pub(crate) const LONG_RIGHTWARDS_DOUBLE_ARROW: Op = Op('⟹');
pub(crate) const LONG_LEFT_RIGHT_DOUBLE_ARROW: Op = Op('⟺');
// pub(crate) const LONG_LEFTWARDS_ARROW_FROM_BAR: Op = Op('⟻');
pub(crate) const LONG_RIGHTWARDS_ARROW_FROM_BAR: Op = Op('⟼');

//
// Unicode Block: Supplemental Arrows-B
//
pub(crate) const LEFTWARDS_ARROW_TAIL: Op = Op('⤙');
pub(crate) const RIGHTWARDS_ARROW_TAIL: Op = Op('⤚');

//
// Unicode Block: Miscellaneous Mathematical Symbols-B
//
pub(crate) const SQUARED_RISING_DIAGONAL_SLASH: Op = Op('⧄');
pub(crate) const SQUARED_FALLING_DIAGONAL_SLASH: Op = Op('⧅');
pub(crate) const SQUARED_SQUARE: Op = Op('⧈');
pub(crate) const BLACK_LOZENGE: char = '⧫';

//
// Unicode Block: Supplemental Mathematical Operators
//
pub(crate) const N_ARY_CIRCLED_DOT_OPERATOR: Op = Op('⨀');
pub(crate) const N_ARY_CIRCLED_PLUS_OPERATOR: Op = Op('⨁');
pub(crate) const N_ARY_CIRCLED_TIMES_OPERATOR: Op = Op('⨂');
pub(crate) const N_ARY_UNION_OPERATOR_WITH_DOT: Op = Op('⨃');
pub(crate) const N_ARY_UNION_OPERATOR_WITH_PLUS: Op = Op('⨄');
pub(crate) const N_ARY_SQUARE_INTERSECTION_OPERATOR: Op = Op('⨅');
pub(crate) const N_ARY_SQUARE_UNION_OPERATOR: Op = Op('⨆');
// pub(crate) const TWO_LOGICAL_AND_OPERATOR: Op = Op('⨇');
// pub(crate) const TWO_LOGICAL_OR_OPERATOR: Op = Op('⨈');
pub(crate) const N_ARY_TIMES_OPERATOR: Op = Op('⨉');
// pub(crate) const MODULO_TWO_SUM: Op = Op('⨊');
pub(crate) const SUMMATION_WITH_INTEGRAL: Op = Op('⨋');
pub(crate) const QUADRUPLE_INTEGRAL_OPERATOR: Op = Op('⨌');
pub(crate) const FINITE_PARTL_INTEGRAL: Op = Op('⨍');
pub(crate) const INTEGRAL_WITH_DOUBLE_STROKE: Op = Op('⨎');
pub(crate) const INTEGRAL_AVERAGE_WITH_SLASH: Op = Op('⨏');
pub(crate) const CIRCULATION_FUNCTION: Op = Op('⨐');
pub(crate) const ANTICLOCKWISE_INTEGRATION: Op = Op('⨑');
// pub(crate) const LINE_INTEGRATION_WITH_RECTANGULAR_PATH_AROUND_POLE: Op = Op('⨒');
// pub(crate) const LINE_INTEGRATION_WITH_SEMICIRCULAR_PATH_AROUND_POLE: Op = Op('⨓');
// pub(crate) const LINE_INTEGRATION_NOT_INCLUDING_THE_POLE: Op = Op('⨔');
// pub(crate) const INTEGRAL_AROUND_A_POINT_OPERATOR: Op = Op('⨕');
// pub(crate) const QUATERNION_INTEGRAL_OPERATOR: Op = Op('⨖');
// pub(crate) const INTEGRAL_WITH_LEFTWARDS_ARROW_WITH_HOOK: Op = Op('⨗');
// pub(crate) const INTEGRAL_WITH_TIMES_SIGN: Op = Op('⨘');
// pub(crate) const INTEGRAL_WITH_INTERSECTION: Op = Op('⨙');
// pub(crate) const INTEGRAL_WITH_UNION: Op = Op('⨚');
// pub(crate) const INTEGRAL_WITH_OVERBAR: Op = Op('⨛');
// pub(crate) const INTEGRAL_WITH_UNDERBAR: Op = Op('⨜');
// pub(crate) const JOIN: Op = Op('⨝');
// pub(crate) const LARGE_LEFT_TRIANGLE_OPERATOR: Op = Op('⨞');
// pub(crate) const Z_NOTATION_SCHEMA_COMPOSITION: Op = Op('⨟');
// pub(crate) const Z_NOTATION_SCHEMA_PIPING: Op = Op('⨠');
// pub(crate) const Z_NOTATION_SCHEMA_PROJECTION: Op = Op('⨡');
// pub(crate) const PLUS_SIGN_WITH_SMALL_CIRCLE_ABOVE: Op = Op('⨢');
// pub(crate) const PLUS_SIGN_WITH_CIRCUMFLEX_ACCENT_ABOVE: Op = Op('⨣');
// pub(crate) const PLUS_SIGN_WITH_TILDE_ABOVE: Op = Op('⨤');
// pub(crate) const PLUS_SIGN_WITH_DOT_BELOW: Op = Op('⨥');
// pub(crate) const PLUS_SIGN_WITH_TILDE_BELOW: Op = Op('⨦');
// pub(crate) const PLUS_SIGN_WITH_SUBSCRIPT_TWO: Op = Op('⨧');
// pub(crate) const PLUS_SIGN_WITH_BLACK_TRIANGLE: Op = Op('⨨');
// pub(crate) const MINUS_SIGN_WITH_COMMA_ABOVE: Op = Op('⨩');
// pub(crate) const MINUS_SIGN_WITH_DOT_BELOW: Op = Op('⨪');
// pub(crate) const MINUS_SIGN_WITH_FALLING_DOTS: Op = Op('⨫');
// pub(crate) const MINUS_SIGN_WITH_RISING_DOTS: Op = Op('⨬');
// pub(crate) const PLUS_SIGN_IN_LEFT_HALF_CIRCLE: Op = Op('⨭');
// pub(crate) const PLUS_SIGN_IN_RIGHT_HALF_CIRCLE: Op = Op('⨮');
// pub(crate) const VECTOR_OR_CROSS_PRODUCT: Op = Op('⨯');
// pub(crate) const MULTIPLICATION_SIGN_WITH_DOT_ABOVE: Op = Op('⨰');
// pub(crate) const MULTIPLICATION_SIGN_WITH_UNDERBAR: Op = Op('⨱');
// pub(crate) const SEMIDIRECT_PRODUCT_WITH_BOTTOM_CLOSED: Op = Op('⨲');
// pub(crate) const SMASH_PRODUCT: Op = Op('⨳');
// pub(crate) const MULTIPLICATION_SIGN_IN_LEFT_HALF_CIRCLE: Op = Op('⨴');
// pub(crate) const MULTIPLICATION_SIGN_IN_RIGHT_HALF_CIRCLE: Op = Op('⨵');
// pub(crate) const CIRCLED_MULTIPLICATION_SIGN_WITH_CIRCUMFLEX_ACCENT: Op = Op('⨶');
// pub(crate) const MULTIPLICATION_SIGN_IN_DOUBLE_CIRCLE: Op = Op('⨷');
// pub(crate) const CIRCLED_DIVISION_SIGN: Op = Op('⨸');
// pub(crate) const PLUS_SIGN_IN_TRIANGLE: Op = Op('⨹');
// pub(crate) const MINUS_SIGN_IN_TRIANGLE: Op = Op('⨺');
// pub(crate) const MULTIPLICATION_SIGN_IN_TRIANGLE: Op = Op('⨻');
// pub(crate) const INTERIOR_PRODUCT: Op = Op('⨼');
// pub(crate) const RIGHTHAND_INTERIOR_PRODUCT: Op = Op('⨽');
// pub(crate) const Z_NOTATION_RELATIONAL_COMPOSITION: Op = Op('⨾');
pub(crate) const AMALGAMATION_OR_COPRODUCT: Op = Op('⨿');
// pub(crate) const INTERSECTION_WITH_DOT: Op = Op('⩀');
// pub(crate) const UNION_WITH_MINUS_SIGN: Op = Op('⩁');
// pub(crate) const UNION_WITH_OVERBAR: Op = Op('⩂');
// pub(crate) const INTERSECTION_WITH_OVERBAR: Op = Op('⩃');
// pub(crate) const INTERSECTION_WITH_LOGICAL_AND: Op = Op('⩄');
// pub(crate) const UNION_WITH_LOGICAL_OR: Op = Op('⩅');
// pub(crate) const UNION_ABOVE_INTERSECTION: Op = Op('⩆');
// pub(crate) const INTERSECTION_ABOVE_UNION: Op = Op('⩇');
// pub(crate) const UNION_ABOVE_BAR_ABOVE_INTERSECTION: Op = Op('⩈');
// pub(crate) const INTERSECTION_ABOVE_BAR_ABOVE_UNION: Op = Op('⩉');
// pub(crate) const UNION_BESIDE_AND_JOINED_WITH_UNION: Op = Op('⩊');
// pub(crate) const INTERSECTION_BESIDE_AND_JOINED_WITH_INTERSECTION: Op = Op('⩋');
// pub(crate) const CLOSED_UNION_WITH_SERIFS: Op = Op('⩌');
// pub(crate) const CLOSED_INTERSECTION_WITH_SERIFS: Op = Op('⩍');
// pub(crate) const DOUBLE_SQUARE_INTERSECTION: Op = Op('⩎');
// pub(crate) const DOUBLE_SQUARE_UNION: Op = Op('⩏');
// pub(crate) const CLOSED_UNION_WITH_SERIFS_AND_SMASH_PRODUCT: Op = Op('⩐');
// pub(crate) const LOGICAL_AND_WITH_DOT_ABOVE: Op = Op('⩑');
// pub(crate) const LOGICAL_OR_WITH_DOT_ABOVE: Op = Op('⩒');
// pub(crate) const DOUBLE_LOGICAL_AND: Op = Op('⩓');
// pub(crate) const DOUBLE_LOGICAL_OR: Op = Op('⩔');
// pub(crate) const TWO_INTERSECTING_LOGICAL_AND: Op = Op('⩕');
// pub(crate) const TWO_INTERSECTING_LOGICAL_OR: Op = Op('⩖');
// pub(crate) const SLOPING_LARGE_OR: Op = Op('⩗');
// pub(crate) const SLOPING_LARGE_AND: Op = Op('⩘');
// pub(crate) const LOGICAL_OR_OVERLAPPING_LOGICAL_AND: Op = Op('⩙');
// pub(crate) const LOGICAL_AND_WITH_MIDDLE_STEM: Op = Op('⩚');
// pub(crate) const LOGICAL_OR_WITH_MIDDLE_STEM: Op = Op('⩛');
// pub(crate) const LOGICAL_AND_WITH_HORIZONTAL_DASH: Op = Op('⩜');
// pub(crate) const LOGICAL_OR_WITH_HORIZONTAL_DASH: Op = Op('⩝');
// pub(crate) const LOGICAL_AND_WITH_DOUBLE_OVERBAR: Op = Op('⩞');
// pub(crate) const LOGICAL_AND_WITH_UNDERBAR: Op = Op('⩟');
// pub(crate) const LOGICAL_AND_WITH_DOUBLE_UNDERBAR: Op = Op('⩠');
// pub(crate) const SMALL_VEE_WITH_UNDERBAR: Op = Op('⩡');
// pub(crate) const LOGICAL_OR_WITH_DOUBLE_OVERBAR: Op = Op('⩢');
// pub(crate) const LOGICAL_OR_WITH_DOUBLE_UNDERBAR: Op = Op('⩣');
// pub(crate) const Z_NOTATION_DOMAIN_ANTIRESTRICTION: Op = Op('⩤');
// pub(crate) const Z_NOTATION_RANGE_ANTIRESTRICTION: Op = Op('⩥');
pub(crate) const EQUALS_SIGN_WITH_DOT_BELOW: Op = Op('⩦');
// pub(crate) const IDENTICAL_WITH_DOT_ABOVE: Op = Op('⩧');
// pub(crate) const TRIPLE_HORIZONTAL_BAR_WITH_DOUBLE_VERTICAL_STROKE: Op = Op('⩨');
// pub(crate) const TRIPLE_HORIZONTAL_BAR_WITH_TRIPLE_VERTICAL_STROKE: Op = Op('⩩');
// pub(crate) const TILDE_OPERATOR_WITH_DOT_ABOVE: Op = Op('⩪');
// pub(crate) const TILDE_OPERATOR_WITH_RISING_DOTS: Op = Op('⩫');
// pub(crate) const SIMILAR_MINUS_SIMILAR: Op = Op('⩬');
// pub(crate) const CONGRUENT_WITH_DOT_ABOVE: Op = Op('⩭');
// pub(crate) const EQUALS_WITH_ASTERISK: Op = Op('⩮');
// pub(crate) const ALMOST_EQUAL_TO_WITH_CIRCUMFLEX_ACCENT: Op = Op('⩯');
// pub(crate) const APPROXIMATELY_EQUAL_OR_EQUAL_TO: Op = Op('⩰');
// pub(crate) const EQUALS_SIGN_ABOVE_PLUS_SIGN: Op = Op('⩱');
// pub(crate) const PLUS_SIGN_ABOVE_EQUALS_SIGN: Op = Op('⩲');
// pub(crate) const EQUALS_SIGN_ABOVE_TILDE_OPERATOR: Op = Op('⩳');
// pub(crate) const DOUBLE_COLON_EQUAL: Op = Op('⩴');
// pub(crate) const TWO_CONSECUTIVE_EQUALS_SIGNS: Op = Op('⩵');
// pub(crate) const THREE_CONSECUTIVE_EQUALS_SIGNS: Op = Op('⩶');
// pub(crate) const EQUALS_SIGN_WITH_TWO_DOTS_ABOVE_AND_TWO_DOTS_BELOW: Op = Op('⩷');
// pub(crate) const EQUIVALENT_WITH_FOUR_DOTS_ABOVE: Op = Op('⩸');
// pub(crate) const LESS_THAN_WITH_CIRCLE_INSIDE: Op = Op('⩹');
// pub(crate) const GREATER_THAN_WITH_CIRCLE_INSIDE: Op = Op('⩺');
// pub(crate) const LESS_THAN_WITH_QUESTION_MARK_ABOVE: Op = Op('⩻');
// pub(crate) const GREATER_THAN_WITH_QUESTION_MARK_ABOVE: Op = Op('⩼');
pub(crate) const LESS_THAN_OR_SLANTED_EQUAL_TO: Op = Op('⩽');
pub(crate) const GREATER_THAN_OR_SLANTED_EQUAL_TO: Op = Op('⩾');
// pub(crate) const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_INSIDE: Op = Op('⩿');
// pub(crate) const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_INSIDE: Op = Op('⪀');
// pub(crate) const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⪁');
// pub(crate) const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⪂');
// pub(crate) const LESS_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE_RIGHT: Op = Op('⪃');
// pub(crate) const GREATER_THAN_OR_SLANTED_EQUAL_TO_WITH_DOT_ABOVE_LEFT: Op = Op('⪄');
pub(crate) const LESS_THAN_OR_APPROXIMATE: Op = Op('⪅');
pub(crate) const GREATER_THAN_OR_APPROXIMATE: Op = Op('⪆');
pub(crate) const LESS_THAN_AND_SINGLE_LINE_NOT_EQUAL_TO: Op = Op('⪇');
pub(crate) const GREATER_THAN_AND_SINGLE_LINE_NOT_EQUAL_TO: Op = Op('⪈');
// pub(crate) const LESS_THAN_AND_NOT_APPROXIMATE: Op = Op('⪉');
// pub(crate) const GREATER_THAN_AND_NOT_APPROXIMATE: Op = Op('⪊');
pub(crate) const LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_GREATER_THAN: Op = Op('⪋');
// pub(crate) const GREATER_THAN_ABOVE_DOUBLE_LINE_EQUAL_ABOVE_LESS_THAN: Op = Op('⪌');
// pub(crate) const LESS_THAN_ABOVE_SIMILAR_OR_EQUAL: Op = Op('⪍');
// pub(crate) const GREATER_THAN_ABOVE_SIMILAR_OR_EQUAL: Op = Op('⪎');
// pub(crate) const LESS_THAN_ABOVE_SIMILAR_ABOVE_GREATER_THAN: Op = Op('⪏');
// pub(crate) const GREATER_THAN_ABOVE_SIMILAR_ABOVE_LESS_THAN: Op = Op('⪐');
// pub(crate) const LESS_THAN_ABOVE_GREATER_THAN_ABOVE_DOUBLE_LINE_EQUAL: Op = Op('⪑');
// pub(crate) const GREATER_THAN_ABOVE_LESS_THAN_ABOVE_DOUBLE_LINE_EQUAL: Op = Op('⪒');
// pub(crate) const LESS_THAN_ABOVE_SLANTED_EQUAL_ABOVE_GREATER_THAN_ABOVE_SLANTED_EQUAL: Op = Op('⪓');
// pub(crate) const GREATER_THAN_ABOVE_SLANTED_EQUAL_ABOVE_LESS_THAN_ABOVE_SLANTED_EQUAL: Op = Op('⪔');
pub(crate) const SLANTED_EQUAL_TO_OR_LESS_THAN: Op = Op('⪕');
pub(crate) const SLANTED_EQUAL_TO_OR_GREATER_THAN: Op = Op('⪖');
// pub(crate) const SLANTED_EQUAL_TO_OR_LESS_THAN_WITH_DOT_INSIDE: Op = Op('⪗');
// pub(crate) const SLANTED_EQUAL_TO_OR_GREATER_THAN_WITH_DOT_INSIDE: Op = Op('⪘');
// pub(crate) const DOUBLE_LINE_EQUAL_TO_OR_LESS_THAN: Op = Op('⪙');
// pub(crate) const DOUBLE_LINE_EQUAL_TO_OR_GREATER_THAN: Op = Op('⪚');
// pub(crate) const DOUBLE_LINE_SLANTED_EQUAL_TO_OR_LESS_THAN: Op = Op('⪛');
// pub(crate) const DOUBLE_LINE_SLANTED_EQUAL_TO_OR_GREATER_THAN: Op = Op('⪜');
// pub(crate) const SIMILAR_OR_LESS_THAN: Op = Op('⪝');
// pub(crate) const SIMILAR_OR_GREATER_THAN: Op = Op('⪞');
// pub(crate) const SIMILAR_ABOVE_LESS_THAN_ABOVE_EQUALS_SIGN: Op = Op('⪟');
// pub(crate) const SIMILAR_ABOVE_GREATER_THAN_ABOVE_EQUALS_SIGN: Op = Op('⪠');
// pub(crate) const DOUBLE_NESTED_LESS_THAN: Op = Op('⪡');
// pub(crate) const DOUBLE_NESTED_GREATER_THAN: Op = Op('⪢');
// pub(crate) const DOUBLE_NESTED_LESS_THAN_WITH_UNDERBAR: Op = Op('⪣');
// pub(crate) const GREATER_THAN_OVERLAPPING_LESS_THAN: Op = Op('⪤');
// pub(crate) const GREATER_THAN_BESIDE_LESS_THAN: Op = Op('⪥');
// pub(crate) const LESS_THAN_CLOSED_BY_CURVE: Op = Op('⪦');
// pub(crate) const GREATER_THAN_CLOSED_BY_CURVE: Op = Op('⪧');
// pub(crate) const LESS_THAN_CLOSED_BY_CURVE_ABOVE_SLANTED_EQUAL: Op = Op('⪨');
// pub(crate) const GREATER_THAN_CLOSED_BY_CURVE_ABOVE_SLANTED_EQUAL: Op = Op('⪩');
// pub(crate) const SMALLER_THAN: Op = Op('⪪');
// pub(crate) const LARGER_THAN: Op = Op('⪫');
// pub(crate) const SMALLER_THAN_OR_EQUAL_TO: Op = Op('⪬');
// pub(crate) const LARGER_THAN_OR_EQUAL_TO: Op = Op('⪭');
// pub(crate) const EQUALS_SIGN_WITH_BUMPY_ABOVE: Op = Op('⪮');
pub(crate) const PRECEDES_ABOVE_SINGLE_LINE_EQUALS_SIGN: Op = Op('⪯');
pub(crate) const SUCCEEDS_ABOVE_SINGLE_LINE_EQUALS_SIGN: Op = Op('⪰');
// pub(crate) const PRECEDES_ABOVE_SINGLE_LINE_NOT_EQUAL_TO: Op = Op('⪱');
// pub(crate) const SUCCEEDS_ABOVE_SINGLE_LINE_NOT_EQUAL_TO: Op = Op('⪲');
// pub(crate) const PRECEDES_ABOVE_EQUALS_SIGN: Op = Op('⪳');
// pub(crate) const SUCCEEDS_ABOVE_EQUALS_SIGN: Op = Op('⪴');
pub(crate) const PRECEDES_ABOVE_NOT_EQUAL_TO: Op = Op('⪵');
pub(crate) const SUCCEEDS_ABOVE_NOT_EQUAL_TO: Op = Op('⪶');
pub(crate) const PRECEDES_ABOVE_ALMOST_EQUAL_TO: Op = Op('⪷');
pub(crate) const SUCCEEDS_ABOVE_ALMOST_EQUAL_TO: Op = Op('⪸');
pub(crate) const PRECEDES_ABOVE_NOT_ALMOST_EQUAL_TO: Op = Op('⪹');
pub(crate) const SUCCEEDS_ABOVE_NOT_ALMOST_EQUAL_TO: Op = Op('⪺');
// pub(crate) const DOUBLE_PRECEDES: Op = Op('⪻');
// pub(crate) const DOUBLE_SUCCEEDS: Op = Op('⪼');
// pub(crate) const SUBSET_WITH_DOT: Op = Op('⪽');
// pub(crate) const SUPERSET_WITH_DOT: Op = Op('⪾');
// pub(crate) const SUBSET_WITH_PLUS_SIGN_BELOW: Op = Op('⪿');
// pub(crate) const SUPERSET_WITH_PLUS_SIGN_BELOW: Op = Op('⫀');
// pub(crate) const SUBSET_WITH_MULTIPLICATION_SIGN_BELOW: Op = Op('⫁');
// pub(crate) const SUPERSET_WITH_MULTIPLICATION_SIGN_BELOW: Op = Op('⫂');
// pub(crate) const SUBSET_OF_OR_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⫃');
// pub(crate) const SUPERSET_OF_OR_EQUAL_TO_WITH_DOT_ABOVE: Op = Op('⫄');
// pub(crate) const SUBSET_OF_ABOVE_EQUALS_SIGN: Op = Op('⫅');
// pub(crate) const SUPERSET_OF_ABOVE_EQUALS_SIGN: Op = Op('⫆');
// pub(crate) const SUBSET_OF_ABOVE_TILDE_OPERATOR: Op = Op('⫇');
// pub(crate) const SUPERSET_OF_ABOVE_TILDE_OPERATOR: Op = Op('⫈');
// pub(crate) const SUBSET_OF_ABOVE_ALMOST_EQUAL_TO: Op = Op('⫉');
// pub(crate) const SUPERSET_OF_ABOVE_ALMOST_EQUAL_TO: Op = Op('⫊');
pub(crate) const SUBSET_OF_ABOVE_NOT_EQUAL_TO: Op = Op('⫋');
pub(crate) const SUPERSET_OF_ABOVE_NOT_EQUAL_TO: Op = Op('⫌');

//
// Unicode Block: Small Form Variants
//
pub(crate) const SMALL_REVERSE_SOLIDUS: Op = Op('﹨');
