use std::mem;

use crate::mathml_renderer::{
    arena::{Arena, Buffer, StringBuilder},
    ast::Node,
    attribute::{
        FracAttr, LetterAttr, MathSpacing, MathVariant, OpAttr, RowAttr, StretchMode, Style,
        TextTransform,
    },
    length::{Length, LengthUnit},
    symbol,
    table::Alignment,
};

use super::{
    color_defs::get_color,
    commands::get_negated_op,
    error::{LatexErrKind, LatexError, Place},
    lexer::Lexer,
    specifications::{LatexUnit, parse_column_specification, parse_length_specification},
    token::{TokLoc, Token},
};

pub(crate) struct Parser<'arena, 'source> {
    pub(crate) l: Lexer<'source>,
    peek: TokLoc<'source>,
    buffer: Buffer,
    arena: &'arena Arena,
    collector: LetterCollector<'arena>,
    tf_differs_on_upright_letters: bool,
}

/// A struct for managing the state of the sequence parser.
#[derive(Debug, Default)]
struct SequenceState {
    class: Class,
    /// `true` if the boundaries of the sequence are real boundaries;
    /// this is not the case for style-only rows.
    real_boundaries: bool,
}

#[derive(Debug, Default)]
enum Class {
    /// `mathord`
    #[default]
    Default,
    /// `mathopen`
    Open,
    /// `mathrel`
    Relation,
    /// `mathpunct`
    Punctuation,
    /// `mathbin`
    BinaryOp,
    /// `mathop`
    Operator,
}

#[derive(Debug)]
enum SequenceEnd {
    Token(Token<'static>),
    AnyEndToken,
}

impl SequenceEnd {
    #[inline]
    fn matches(&self, other: &Token<'_>) -> bool {
        match self {
            SequenceEnd::Token(token) => token.is_same_kind_as(other),
            SequenceEnd::AnyEndToken => matches!(
                other,
                Token::EOF | Token::GroupEnd | Token::End | Token::Right
            ),
        }
    }
}

#[derive(Debug)]
enum ParseAs {
    Sequence,
    ContinueSequence,
    Arg,
    ArgWithSpace,
}

impl<'arena, 'source> Parser<'arena, 'source>
where
    'source: 'arena, // The reference to the source string will live as long as the arena.
{
    pub(crate) fn new(lexer: Lexer<'source>, arena: &'arena Arena) -> Self {
        let input_length = lexer.input_length;
        let mut p = Parser {
            l: lexer,
            peek: TokLoc(0, Token::EOF),
            buffer: Buffer::new(input_length),
            arena,
            collector: LetterCollector::Inactive,
            tf_differs_on_upright_letters: false,
        };
        // Discard the EOF token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    fn maybe_collect(&mut self) -> TokLoc<'source> {
        if matches!(self.collector, LetterCollector::Collecting) {
            let first_loc = self.peek.location();
            let mut builder = self.buffer.get_builder();
            let mut num_chars = 0usize;
            let mut first_char: Option<char> = None;

            // Loop until we find a non-letter token.
            while let tok @ (Token::Letter(ch) | Token::UprightLetter(ch)) = self.peek.token() {
                // We stop collecting if we encounter an upright letter while the transformation is
                // different on upright letters. Handling upright letters differently wouldn't be
                // possible anymore if we merged these letters // here together with the non-upright
                // letters.
                if matches!(tok, Token::UprightLetter(_)) && self.tf_differs_on_upright_letters {
                    break;
                }
                builder.push_char(*ch);
                if first_char.is_none() {
                    first_char = Some(*ch);
                }
                num_chars += 1;
                // Get the next token for the next iteration.
                self.peek = self.l.next_token();
            }
            // If we collected at least one letter, commit it to the arena and signal with a token
            // that we are done.
            if let Some(ch) = first_char {
                match num_chars.cmp(&1) {
                    std::cmp::Ordering::Equal => {
                        self.collector = LetterCollector::FinishedOneLetter {
                            collected_letter: ch,
                        };
                    }
                    std::cmp::Ordering::Greater => {
                        self.collector = LetterCollector::FinishedManyLetters {
                            collected_letters: builder.finish(self.arena),
                        };
                    }
                    _ => {}
                }
                return TokLoc(first_loc, Token::GetCollectedLetters);
            }
        }
        self.next_token()
    }

    #[inline(never)]
    fn next_token(&mut self) -> TokLoc<'source> {
        next_token(&mut self.peek, &mut self.l)
    }

    #[inline]
    pub(crate) fn parse(&mut self) -> Result<Vec<&'arena Node<'arena>>, LatexError<'source>> {
        let nodes = self.parse_sequence(SequenceEnd::Token(Token::EOF), None)?;
        Ok(nodes)
    }

    /// Parse a sequence of tokens until the given end token is encountered.
    ///
    /// Note that this function does not consume the end token. That's because the end token might
    /// be used by the calling function to emit another node.
    ///
    /// If `real_boundaries` is `true`, the parser will treat the boundaries of the sequence as real.
    /// This is used for sequences that are *not* just style-only rows.
    fn parse_sequence(
        &mut self,
        sequence_end: SequenceEnd,
        sequence_state: Option<&mut SequenceState>,
    ) -> Result<Vec<&'arena Node<'arena>>, LatexError<'source>> {
        let mut nodes = Vec::new();
        let sequence_state = if let Some(seq_state) = sequence_state {
            seq_state
        } else {
            &mut SequenceState {
                class: Class::Open,
                real_boundaries: true,
            }
        };

        // Because we don't want to consume the end token, we just peek here.
        while !sequence_end.matches(self.peek.token()) {
            let cur_tokloc = self.maybe_collect();
            if matches!(cur_tokloc.token(), Token::EOF) {
                // When the input ends without the closing token.
                if let SequenceEnd::Token(end_token) = sequence_end {
                    return Err(LatexError(
                        cur_tokloc.location(),
                        LatexErrKind::UnclosedGroup(end_token),
                    ));
                }
            }
            // Parse the token.
            let target = self.parse_token(cur_tokloc, ParseAs::Sequence, Some(sequence_state))?;

            // Check if there are any superscripts or subscripts following the parsed node.
            let bounds = self.get_bounds()?;

            // If there are superscripts or subscripts, we need to wrap the node we just got into
            // one of the node types for superscripts and subscripts.
            let node = self.commit(match bounds {
                Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                Bounds(Some(symbol), None) => Node::Subscript { target, symbol },
                Bounds(None, Some(symbol)) => Node::Superscript { target, symbol },
                Bounds(None, None) => {
                    nodes.push(target);
                    continue;
                }
            });
            nodes.push(node);
        }
        Ok(nodes)
    }

    /// Put the node onto the heap in the arena and return a reference to it.
    ///
    /// The advantage over using `Box` is that we can store the nodes in a contiguous
    /// memory block, and release all of them at once when the arena is dropped.
    ///
    /// Ideally, the node is constructed directly on the heap, so try to avoid
    /// constructing it on the stack and then moving it.
    fn commit(&self, node: Node<'arena>) -> &'arena Node<'arena> {
        self.arena.push(node)
    }

    /// Parse the given token into a node.
    fn parse_token(
        &mut self,
        cur_tokloc: TokLoc<'source>,
        parse_as: ParseAs,
        mut sequence_state: Option<&mut SequenceState>,
    ) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        let TokLoc(loc, cur_token) = cur_tokloc;
        // Move out the sequence state if it exists, and replace it with the default state.
        let mut prev_state = sequence_state
            .as_mut()
            .map_or_else(SequenceState::default, |state| mem::take(state));
        if let Some(ref mut state) = sequence_state {
            state.real_boundaries = prev_state.real_boundaries;
        };
        let node = match cur_token {
            Token::Number(number) => {
                let mut builder = self.buffer.get_builder();
                builder.push_char(number as u8 as char);
                if matches!(parse_as, ParseAs::Sequence) {
                    // Consume tokens as long as they are `Token::Number` or
                    // `Token::Letter('.')` or `Token::Punctuation(symbol::COMMA)`,
                    // but only if the token *after that* is a digit.
                    loop {
                        let ch = if let Token::Number(number) = self.peek.token() {
                            *number as u8 as char
                        } else {
                            let ch = match self.peek.token() {
                                Token::Letter('.') => Some('.'),
                                Token::Punctuation(symbol::COMMA) => Some(','),
                                _ => None,
                            };
                            if let Some(ch) = ch {
                                if self.l.is_next_digit() {
                                    ch
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        };
                        builder.push_char(ch);
                        next_token(&mut self.peek, &mut self.l);
                    }
                }
                Node::Number(builder.finish(self.arena))
            }
            Token::Letter(x) => Node::IdentifierChar(x, LetterAttr::Default),
            Token::UprightLetter(x) => Node::IdentifierChar(x, LetterAttr::Upright),
            Token::Relation(relation) => {
                if let Some(state) = sequence_state {
                    state.class = Class::Relation;
                };
                let left = if matches!(
                    prev_state.class,
                    Class::Relation | Class::Open | Class::Punctuation
                ) {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                let right = if (matches!(parse_as, ParseAs::Sequence)
                    && matches!(
                        self.peek.token(),
                        Token::Relation(_) | Token::Punctuation(_) | Token::Colon
                    ))
                    || (prev_state.real_boundaries
                        && matches!(
                            self.peek.token(),
                            Token::EOF | Token::GroupEnd | Token::End | Token::Right
                        )) {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                Node::Operator {
                    op: relation.into(),
                    attr: None,
                    left,
                    right,
                }
            }
            Token::Punctuation(punc) => {
                if let Some(state) = sequence_state {
                    state.class = Class::Punctuation;
                };
                Node::Operator {
                    op: punc.into(),
                    attr: None,
                    left: None,
                    right: None,
                }
            }
            Token::Ord(ord) => Node::Operator {
                op: ord.into(),
                attr: None,
                left: None,
                right: None,
            },
            Token::BinaryOp(binary_op) => {
                if let Some(state) = sequence_state {
                    state.class = Class::BinaryOp;
                };
                let spacing = if matches!(
                    prev_state.class,
                    Class::Relation | Class::Punctuation | Class::BinaryOp | Class::Open
                ) || matches!(
                    self.peek.token(),
                    Token::Relation(_) | Token::Punctuation(_)
                ) || (prev_state.real_boundaries
                    && matches!(
                        self.peek.token(),
                        Token::EOF | Token::GroupEnd | Token::End | Token::Right
                    )) {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                Node::Operator {
                    op: binary_op.into(),
                    attr: None,
                    left: spacing,
                    right: spacing,
                }
            }
            Token::OpGreaterThan => Node::PseudoOp {
                name: "&gt;",
                attr: None,
                left: None,
                right: None,
            },
            Token::OpLessThan => Node::PseudoOp {
                name: "&lt;",
                attr: None,
                left: None,
                right: None,
            },
            Token::OpAmpersand => Node::PseudoOp {
                name: "&amp;",
                attr: None,
                left: None,
                right: None,
            },
            Token::PseudoOperator(name) => {
                if let Some(state) = sequence_state {
                    state.class = Class::Operator;
                };
                return self.make_pseudo_operator(name, &prev_state);
            }
            Token::Space(space) => {
                // Spaces pass through the sequence state.
                if let Some(state) = sequence_state {
                    *state = prev_state;
                };
                Node::Space(space)
            }
            Token::CustomSpace => {
                let (loc, length) = self.parse_ascii_text_group()?;
                let space = parse_length_specification(length.trim())
                    .ok_or(LatexError(loc, LatexErrKind::ExpectedLength(length)))?;
                Node::Space(space)
            }
            Token::NonBreakingSpace => Node::Text("\u{A0}"),
            Token::Sqrt => {
                let next = self.next_token();
                if matches!(next.token(), Token::SquareBracketOpen) {
                    let degree =
                        self.parse_sequence(SequenceEnd::Token(Token::SquareBracketClose), None)?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_next(ParseAs::Arg)?;
                    Node::Root(node_vec_to_node(self.arena, degree, true), content)
                } else {
                    Node::Sqrt(self.parse_token(next, ParseAs::Arg, None)?)
                }
            }
            Token::Frac(attr) | Token::Binom(attr) => {
                let num = self.parse_next(ParseAs::Arg)?;
                let denom = self.parse_next(ParseAs::Arg)?;
                if matches!(cur_token, Token::Binom(_)) {
                    let (lt_value, lt_unit) = Length::zero().into_parts();
                    Node::Fenced {
                        open: symbol::LEFT_PARENTHESIS,
                        close: symbol::RIGHT_PARENTHESIS,
                        content: self.commit(Node::Frac {
                            num,
                            denom,
                            lt_value,
                            lt_unit,
                            attr,
                        }),
                        style: None,
                    }
                } else {
                    let (lt_value, lt_unit) = Length::none().into_parts();
                    Node::Frac {
                        num,
                        denom,
                        lt_value,
                        lt_unit,
                        attr,
                    }
                }
            }
            Token::Genfrac => {
                // TODO: This should not just blindly try to parse a node.
                // Rather, we should explicitly attempt to parse a group (aka Row),
                // and if that doesn't work, we try to parse it as an Operator,
                // and if that still doesn't work, we return an error.
                let open = match self.parse_next(ParseAs::Arg)? {
                    Node::StretchableOp(op, _) => *op,
                    Node::Row { nodes: [], .. } => symbol::NULL,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let close = match self.parse_next(ParseAs::Arg)? {
                    Node::StretchableOp(op, _) => *op,
                    Node::Row { nodes: [], .. } => symbol::NULL,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let (loc, length) = self.parse_ascii_text_group()?;
                let lt = match length.trim() {
                    "" => Length::none(),
                    decimal => parse_length_specification(decimal)
                        .ok_or(LatexError(loc, LatexErrKind::ExpectedLength(decimal)))?,
                };
                let style = match self.parse_next(ParseAs::Arg)? {
                    Node::Number(num) => match num.as_bytes() {
                        b"0" => Some(Style::Display),
                        b"1" => Some(Style::Text),
                        b"2" => Some(Style::Script),
                        b"3" => Some(Style::ScriptScript),
                        _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                    },
                    Node::Row { nodes: [], .. } => None,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let num = self.parse_next(ParseAs::Arg)?;
                let denom = self.parse_next(ParseAs::Arg)?;
                let attr = None;
                let (lt_value, lt_unit) = lt.into_parts();
                Node::Fenced {
                    open,
                    close,
                    content: self.commit(Node::Frac {
                        num,
                        denom,
                        lt_value,
                        lt_unit,
                        attr,
                    }),
                    style,
                }
            }
            Token::OverUnder(op, is_over, attr) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                if is_over {
                    Node::OverOp(op.into(), attr, target)
                } else {
                    Node::UnderOp(op.into(), target)
                }
            }
            Token::Overset | Token::Underset => {
                let symbol = self.parse_next(ParseAs::Arg)?;
                let token = self.next_token();
                let target =
                    self.parse_token(token, ParseAs::ContinueSequence, Some(&mut prev_state))?;
                if matches!(cur_token, Token::Overset) {
                    Node::Overset { symbol, target }
                } else {
                    Node::Underset { symbol, target }
                }
            }
            Token::OverUnderBrace(x, is_over) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                let symbol = self.commit(Node::Operator {
                    op: x.into(),
                    attr: None,
                    left: None,
                    right: None,
                });
                let base = if is_over {
                    Node::Overset { symbol, target }
                } else {
                    Node::Underset { symbol, target }
                };
                if (is_over && matches!(self.peek.token(), Token::Circumflex))
                    || (!is_over && matches!(self.peek.token(), Token::Underscore))
                {
                    let target = self.commit(base);
                    self.next_token(); // Discard the circumflex or underscore token.
                    let expl = self.parse_next(ParseAs::Arg)?;
                    if is_over {
                        Node::Overset {
                            symbol: expl,
                            target,
                        }
                    } else {
                        Node::Underset {
                            symbol: expl,
                            target,
                        }
                    }
                } else {
                    base
                }
            }
            Token::BigOp(op) => {
                let target = if matches!(self.peek.token(), Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    self.commit(Node::Operator {
                        op: op.into(),
                        attr: Some(OpAttr::NoMovableLimits),
                        left: None,
                        right: None,
                    })
                } else {
                    self.commit(Node::Operator {
                        op: op.into(),
                        attr: None,
                        left: None,
                        right: None,
                    })
                };
                match self.get_bounds()? {
                    Bounds(Some(under), Some(over)) => Node::UnderOver {
                        target,
                        under,
                        over,
                    },
                    Bounds(Some(symbol), None) => Node::Underset { target, symbol },
                    Bounds(None, Some(symbol)) => Node::Overset { target, symbol },
                    Bounds(None, None) => return Ok(target),
                }
            }
            Token::PseudoOperatorLimits(name) => {
                let movablelimits = if matches!(self.peek.token(), Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    Some(OpAttr::NoMovableLimits)
                } else {
                    Some(OpAttr::ForceMovableLimits)
                };
                if matches!(self.peek.token(), Token::Underscore) {
                    let target = self.commit(Node::PseudoOp {
                        name,
                        attr: movablelimits,
                        left: Some(MathSpacing::ThreeMu),
                        right: Some(MathSpacing::ThreeMu),
                    });
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_next(ParseAs::Arg)?;
                    Node::Underset {
                        target,
                        symbol: under,
                    }
                } else {
                    return self.make_pseudo_operator(name, &prev_state);
                }
            }
            Token::Slashed => {
                let node = self.parse_next(ParseAs::Arg)?;
                Node::Slashed(node)
            }
            Token::Not => {
                // `\not` has to be followed by something:
                match self.next_token().into_token() {
                    Token::Relation(op) => {
                        if let Some(negated) = get_negated_op(op) {
                            Node::Operator {
                                op: negated.into(),
                                attr: None,
                                left: None,
                                right: None,
                            }
                        } else {
                            Node::Operator {
                                op: op.into(),
                                attr: None,
                                left: None,
                                right: None,
                            }
                        }
                    }
                    Token::OpLessThan => Node::Operator {
                        op: symbol::NOT_LESS_THAN.into(),
                        attr: None,
                        left: None,
                        right: None,
                    },
                    Token::OpGreaterThan => Node::Operator {
                        op: symbol::NOT_GREATER_THAN.into(),
                        attr: None,
                        left: None,
                        right: None,
                    },
                    Token::Letter(char) | Token::UprightLetter(char) => {
                        let mut builder = self.buffer.get_builder();
                        builder.push_char(char);
                        builder.push_char('\u{338}');
                        Node::IdentifierStr(builder.finish(self.arena))
                    }
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::CannotBeUsedHere {
                                got: cur_token,
                                correct_place: Place::BeforeSomeOps,
                            },
                        ));
                    }
                }
            }
            Token::Transform(tf) => {
                let old_collector = mem::replace(&mut self.collector, LetterCollector::Collecting);
                let old_tf_differs_on_upright_letters = mem::replace(
                    &mut self.tf_differs_on_upright_letters,
                    tf.differs_on_upright_letters(),
                );
                let content = self.parse_next(ParseAs::Arg)?;
                self.collector = old_collector;
                self.tf_differs_on_upright_letters = old_tf_differs_on_upright_letters;
                Node::TextTransform { content, tf }
            }
            Token::Integral(int) => {
                if matches!(self.peek.token(), Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    let target = self.commit(Node::Operator {
                        op: int.into(),
                        attr: None,
                        left: None,
                        right: None,
                    });
                    match self.get_bounds()? {
                        Bounds(Some(under), Some(over)) => Node::UnderOver {
                            target,
                            under,
                            over,
                        },
                        Bounds(Some(symbol), None) => Node::Underset { target, symbol },
                        Bounds(None, Some(symbol)) => Node::Overset { target, symbol },
                        Bounds(None, None) => return Ok(target),
                    }
                } else {
                    let target = self.commit(Node::Operator {
                        op: int.into(),
                        attr: None,
                        left: None,
                        right: None,
                    });
                    match self.get_bounds()? {
                        Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                        Bounds(Some(symbol), None) => Node::Subscript { target, symbol },
                        Bounds(None, Some(symbol)) => Node::Superscript { target, symbol },
                        Bounds(None, None) => return Ok(target),
                    }
                }
            }
            Token::Colon => {
                // A colon is actually just a relation, but by default, MathML Core gives it
                // punctuation spacing (left: 0, right: 3mu), so we have to explicitly make it have
                // relation spacing (left: 5mu, right: 5mu).
                if let Some(state) = sequence_state {
                    state.class = Class::Relation;
                };
                let left = if !matches!(parse_as, ParseAs::Sequence) {
                    None // Don't add spacing if we are in an argument.
                } else if matches!(
                    prev_state.class,
                    Class::Relation | Class::Open | Class::Punctuation
                ) {
                    Some(MathSpacing::Zero)
                } else {
                    Some(MathSpacing::FiveMu)
                };
                let right = if !matches!(parse_as, ParseAs::Sequence) {
                    None // Don't add spacing if we are in an argument.
                } else if matches!(
                    self.peek.token(),
                    Token::Relation(_) | Token::Punctuation(_) | Token::Colon
                ) {
                    Some(MathSpacing::Zero)
                } else {
                    Some(MathSpacing::FiveMu)
                };
                Node::Operator {
                    op: symbol::COLON.into(),
                    attr: None,
                    left,
                    right,
                }
            }
            Token::GroupBegin => {
                let content = self.parse_sequence(
                    SequenceEnd::Token(Token::GroupEnd),
                    if matches!(parse_as, ParseAs::ContinueSequence) {
                        Some(&mut prev_state)
                    } else {
                        None
                    },
                )?;
                self.next_token(); // Discard the closing token.
                return Ok(node_vec_to_node(
                    self.arena,
                    content,
                    matches!(parse_as, ParseAs::Arg),
                ));
            }
            Token::Delimiter(paren) => Node::StretchableOp(paren, StretchMode::NoStretch),
            Token::SquareBracketOpen => {
                Node::StretchableOp(symbol::LEFT_SQUARE_BRACKET, StretchMode::NoStretch)
            }
            Token::SquareBracketClose => {
                Node::StretchableOp(symbol::RIGHT_SQUARE_BRACKET, StretchMode::NoStretch)
            }
            Token::Left => {
                let TokLoc(loc, next_token) = self.next_token();
                let open_paren = match next_token {
                    Token::Delimiter(open) => open,
                    Token::SquareBracketOpen => symbol::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => symbol::RIGHT_SQUARE_BRACKET,
                    Token::Letter('.') => symbol::NULL,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::MissingParenthesis {
                                location: &Token::Left,
                                got: next_token,
                            },
                        ));
                    }
                };
                let content = self.parse_sequence(SequenceEnd::Token(Token::Right), None)?;
                self.next_token(); // Discard the closing token.
                let TokLoc(loc, next_token) = self.next_token();
                let close_paren = match next_token {
                    Token::Delimiter(close) => close,
                    Token::SquareBracketOpen => symbol::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => symbol::RIGHT_SQUARE_BRACKET,
                    Token::Letter('.') => symbol::NULL,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::MissingParenthesis {
                                location: &Token::Right,
                                got: next_token,
                            },
                        ));
                    }
                };
                Node::Fenced {
                    open: open_paren,
                    close: close_paren,
                    content: node_vec_to_node(self.arena, content, false),
                    style: None,
                }
            }
            Token::Middle => {
                let TokLoc(loc, next_token) = self.next_token();
                let op = match next_token {
                    Token::Delimiter(op) => op,
                    Token::SquareBracketOpen => symbol::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => symbol::RIGHT_SQUARE_BRACKET,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::UnexpectedToken {
                                expected: &Token::Delimiter(symbol::NULL),
                                got: next_token,
                            },
                        ));
                    }
                };
                Node::StretchableOp(op, StretchMode::Middle)
            }
            Token::Big(size) => {
                let TokLoc(loc, next_token) = self.next_token();
                let paren = match next_token {
                    Token::Delimiter(paren) => paren,
                    Token::SquareBracketOpen => symbol::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => symbol::RIGHT_SQUARE_BRACKET,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::UnexpectedToken {
                                expected: &Token::Delimiter(symbol::NULL),
                                got: next_token,
                            },
                        ));
                    }
                };
                Node::SizedParen(size, paren)
            }
            Token::Begin => {
                // Read the environment name.
                let env_name = self.parse_ascii_text_group()?.1;
                let array_spec = if env_name == "array" || env_name == "subarray" {
                    // Parse the array options.
                    let (loc, options) = self.parse_ascii_text_group()?;
                    Some(
                        parse_column_specification(options, self.arena).ok_or_else(|| {
                            LatexError(loc, LatexErrKind::ExpectedColSpec(options.trim()))
                        })?,
                    )
                } else {
                    None
                };
                let content = self.parse_sequence(SequenceEnd::Token(Token::End), None)?;
                let content = self.arena.push_slice(&content);
                let end_token_loc = self.next_token().location();
                let node = match env_name {
                    "align" | "align*" | "aligned" => Node::Table {
                        content,
                        align: Alignment::Alternating,
                        attr: Some(FracAttr::DisplayStyleTrue),
                        with_numbering: env_name == "align",
                    },
                    "cases" => {
                        let align = Alignment::Cases;
                        let content = self.commit(Node::Table {
                            content,
                            align,
                            attr: None,
                            with_numbering: false,
                        });
                        Node::Fenced {
                            open: symbol::LEFT_CURLY_BRACKET,
                            close: symbol::NULL,
                            content,
                            style: None,
                        }
                    }
                    "matrix" => Node::Table {
                        content,
                        align: Alignment::Centered,
                        attr: None,
                        with_numbering: false,
                    },
                    array_variant @ ("array" | "subarray") => {
                        // SAFETY: `array_spec` is guaranteed to be Some because we checked for
                        // "array" and "subarray" above.
                        // TODO: Refactor this to avoid using `unsafe`.
                        let mut spec = unsafe { array_spec.unwrap_unchecked() };
                        let style = if array_variant == "subarray" {
                            spec.is_sub = true;
                            Some(Style::Script)
                        } else {
                            None
                        };
                        Node::Array {
                            style,
                            content,
                            array_spec: self.arena.alloc_array_spec(spec),
                        }
                    }
                    matrix_variant
                    @ ("pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix") => {
                        let align = Alignment::Centered;
                        let (open, close) = match matrix_variant {
                            "pmatrix" => (symbol::LEFT_PARENTHESIS, symbol::RIGHT_PARENTHESIS),
                            "bmatrix" => {
                                (symbol::LEFT_SQUARE_BRACKET, symbol::RIGHT_SQUARE_BRACKET)
                            }
                            "Bmatrix" => (symbol::LEFT_CURLY_BRACKET, symbol::RIGHT_CURLY_BRACKET),
                            "vmatrix" => (symbol::VERTICAL_LINE, symbol::VERTICAL_LINE),
                            "Vmatrix" => {
                                (symbol::DOUBLE_VERTICAL_LINE, symbol::DOUBLE_VERTICAL_LINE)
                            }
                            // SAFETY: `matrix_variant` is one of the strings above.
                            _ => unsafe { std::hint::unreachable_unchecked() },
                        };
                        let attr = None;
                        Node::Fenced {
                            open,
                            close,
                            content: self.commit(Node::Table {
                                content,
                                align,
                                attr,
                                with_numbering: false,
                            }),
                            style: None,
                        }
                    }
                    _ => {
                        return Err(LatexError(loc, LatexErrKind::UnknownEnvironment(env_name)));
                    }
                };
                let end_name = self.parse_ascii_text_group()?.1;
                if end_name != env_name {
                    return Err(LatexError(
                        end_token_loc,
                        LatexErrKind::MismatchedEnvironment {
                            expected: env_name,
                            got: end_name,
                        },
                    ));
                }

                node
            }
            Token::OperatorName => {
                let tokloc = TokLoc(self.peek.location(), *self.peek.token());
                let mut builder = self.buffer.get_builder();
                let mut text_parser =
                    TextModeParser::new(&mut builder, &mut self.peek, &mut self.l);
                text_parser.parse_token_in_text_mode(tokloc)?;
                let letters = builder.finish(self.arena);
                // Discard the last token.
                self.next_token();
                if let Some(ch) = get_single_char(letters) {
                    Node::IdentifierChar(ch, LetterAttr::Upright)
                } else {
                    if let Some(state) = sequence_state {
                        state.class = Class::Operator;
                    };
                    return self.make_pseudo_operator(letters, &prev_state);
                }
            }
            Token::Text(transform) => {
                // Discard any whitespace that immediately follows the `Text` token.
                if matches!(self.peek.token(), Token::Whitespace) {
                    self.next_token();
                }
                // Copy the token out of the peek variable.
                // We do this because we need to turn off text mode while there is still a peek
                // token that is consumed by the `Text` command.
                let tokloc = TokLoc(self.peek.location(), *self.peek.token());
                let mut builder = self.buffer.get_builder();
                let mut text_parser =
                    TextModeParser::new(&mut builder, &mut self.peek, &mut self.l);
                text_parser.parse_token_in_text_mode(tokloc)?;
                let text = builder.finish(self.arena);
                // Now turn off text mode.
                self.l.text_mode = false;
                // Discard the last token that we already processed but we kept it in `peek`,
                // so that we can turn off text mode before new tokens are read.
                self.next_token();
                // Discard any whitespace tokens that are still stored in self.peek_token.
                if matches!(self.peek.token(), Token::Whitespace) {
                    self.next_token();
                }
                if let Some(transform) = transform {
                    Node::TextTransform {
                        content: self.commit(Node::Text(text)),
                        tf: MathVariant::Transform(transform),
                    }
                } else {
                    Node::Text(text)
                }
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Color => {
                let (loc, color_name) = self.parse_ascii_text_group()?;
                let Some(color) = get_color(color_name) else {
                    return Err(LatexError(loc, LatexErrKind::UnknownColor(color_name)));
                };
                let content =
                    self.parse_sequence(SequenceEnd::AnyEndToken, Some(&mut prev_state))?;
                Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attr: color,
                }
            }
            Token::Style(style) => {
                let content =
                    self.parse_sequence(SequenceEnd::AnyEndToken, Some(&mut prev_state))?;
                Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attr: RowAttr::Style(style),
                }
            }
            Token::UnknownCommand(name) => {
                return Err(LatexError(loc, LatexErrKind::UnknownCommand(name)));
            }
            Token::Prime => {
                let target = self.commit(Node::Row {
                    nodes: &[],
                    attr: RowAttr::None,
                });
                let symbol = self.commit(Node::Operator {
                    op: symbol::PRIME.into(),
                    attr: None,
                    left: None,
                    right: None,
                });
                Node::Superscript { target, symbol }
            }
            tok @ (Token::Underscore | Token::Circumflex) => {
                let symbol = self.parse_next(ParseAs::Arg)?;
                if !matches!(self.peek.token(), Token::EOF | Token::GroupEnd | Token::End) {
                    let base = self.parse_next(ParseAs::Sequence)?;
                    let (sub, sup) = if matches!(tok, Token::Underscore) {
                        (Some(symbol), None)
                    } else {
                        (None, Some(symbol))
                    };
                    Node::Multiscript { base, sub, sup }
                } else {
                    let empty_row = self.commit(Node::Row {
                        nodes: &[],
                        attr: RowAttr::None,
                    });
                    if matches!(tok, Token::Underscore) {
                        Node::Subscript {
                            target: empty_row,
                            symbol,
                        }
                    } else {
                        Node::Superscript {
                            target: empty_row,
                            symbol,
                        }
                    }
                }
            }
            Token::Limits => {
                return Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::AfterBigOp,
                    },
                ));
            }
            Token::EOF => return Err(LatexError(loc, LatexErrKind::UnexpectedEOF)),
            Token::End | Token::Right | Token::GroupEnd => {
                return Err(LatexError(loc, LatexErrKind::UnexpectedClose(cur_token)));
            }
            Token::CustomCmd(num_args, predefined) => {
                let mut nodes = Vec::with_capacity(num_args);
                for _ in 0..num_args {
                    let token = self.next_token();
                    let node = self.parse_token(token, ParseAs::Arg, None)?;
                    nodes.push(node);
                }
                let args = self.arena.push_slice(&nodes);
                Node::CustomCmd {
                    predefined: predefined.into_ref(),
                    args,
                }
            }
            Token::CustomCmdArg(arg_num) => Node::CustomCmdArg(arg_num),
            Token::GetCollectedLetters => match self.collector {
                LetterCollector::FinishedOneLetter { collected_letter } => {
                    self.collector = LetterCollector::Collecting;
                    Node::IdentifierChar(collected_letter, LetterAttr::Default)
                }
                LetterCollector::FinishedManyLetters { collected_letters } => {
                    self.collector = LetterCollector::Collecting;
                    Node::IdentifierStr(collected_letters)
                }
                _ => {
                    return Err(LatexError(
                        loc,
                        LatexErrKind::CannotBeUsedHere {
                            got: cur_token,
                            correct_place: Place::AfterOpOrIdent,
                        },
                    ));
                }
            },
            Token::HardcodedMathML(mathml) => Node::HardcodedMathML(mathml),
            // The following are text-mode-only tokens.
            Token::Whitespace | Token::TextModeAccent(_) => {
                return Err(LatexError(
                    loc,
                    // TODO: Find a better error.
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::BeforeSomeOps,
                    },
                ));
            }
        };
        Ok(self.commit(node))
    }

    /// Same as `parse_token`, but also gets the next token.
    #[inline]
    fn parse_next(
        &mut self,
        parse_as: ParseAs,
    ) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        let token = self.next_token();
        self.parse_token(token, parse_as, None)
    }

    /// Parse the contents of a group, `{...}`, which may only contain ASCII text.
    fn parse_ascii_text_group(&mut self) -> Result<(usize, &'source str), LatexError<'source>> {
        // First check whether there is an opening `{` token.
        if !matches!(self.peek.token(), Token::GroupBegin) {
            let TokLoc(loc, token) = self.next_token();
            return Err(LatexError(
                loc,
                LatexErrKind::UnexpectedToken {
                    expected: &Token::GroupBegin,
                    got: token,
                },
            ));
        }
        // Read the text.
        let result = self.l.read_ascii_text_group();
        // Discard the opening `{` token (which is still stored as `peek`).
        let opening_loc = self.next_token().location();
        result
            .map(|r| (opening_loc, r))
            .ok_or(LatexError(opening_loc, LatexErrKind::DisallowedChars))
    }

    /// Parse the bounds of an integral, sum, or product.
    /// These bounds are preceeded by `_` or `^`.
    fn get_bounds(&mut self) -> Result<Bounds<'arena>, LatexError<'source>> {
        let mut primes = self.prime_check();
        // Check whether the first bound is specified and is a lower bound.
        let first_underscore = matches!(self.peek.token(), Token::Underscore);
        let first_circumflex = matches!(self.peek.token(), Token::Circumflex);

        let (sub, sup) = if first_underscore || first_circumflex {
            let first_bound = Some(self.get_sub_or_sub(first_circumflex)?);

            // If the first bound was a subscript *and* we didn't encounter primes yet,
            // we check once more for primes.
            if first_underscore && primes.is_empty() {
                primes = self.prime_check();
            }

            // Check whether both an upper and a lower bound were specified.
            let second_underscore = matches!(self.peek.token(), Token::Underscore);
            let second_circumflex = matches!(self.peek.token(), Token::Circumflex);

            if (first_circumflex && second_circumflex) || (first_underscore && second_underscore) {
                let TokLoc(loc, token) = self.next_token();
                return Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: token,
                        correct_place: Place::AfterOpOrIdent,
                    },
                ));
            }

            if (first_underscore && second_circumflex) || (first_circumflex && second_underscore) {
                let second_bound = Some(self.get_sub_or_sub(second_circumflex)?);
                // Depending on whether the underscore or the circumflex came first,
                // we have to swap the bounds.
                if first_underscore {
                    (first_bound, second_bound)
                } else {
                    (second_bound, first_bound)
                }
            } else if first_underscore {
                (first_bound, None)
            } else {
                (None, first_bound)
            }
        } else {
            (None, None)
        };

        let sup = if !primes.is_empty() {
            if let Some(sup) = sup {
                primes.push(sup);
            }
            Some(node_vec_to_node(self.arena, primes, false))
        } else {
            sup
        };

        Ok(Bounds(sub, sup))
    }

    /// Check for primes and aggregate them into a single node.
    fn prime_check(&mut self) -> Vec<&'arena Node<'arena>> {
        let mut primes = Vec::new();
        let mut prime_count = 0usize;
        while matches!(self.peek.token(), Token::Prime) {
            self.next_token(); // Discard the prime token.
            prime_count += 1;
        }
        static PRIME_SELECTION: [symbol::Ord; 4] = [
            symbol::PRIME,
            symbol::DOUBLE_PRIME,
            symbol::TRIPLE_PRIME,
            symbol::QUADRUPLE_PRIME,
        ];
        if prime_count > 0 {
            // If we have between 1 and 4 primes, we can use the predefined prime operators.
            if let Some(op) = PRIME_SELECTION.get(prime_count - 1) {
                primes.push(self.commit(Node::Operator {
                    op: op.into(),
                    attr: None,
                    left: None,
                    right: None,
                }));
            } else {
                for _ in 0..prime_count {
                    primes.push(self.commit(Node::Operator {
                        op: symbol::PRIME.into(),
                        attr: None,
                        left: None,
                        right: None,
                    }));
                }
            }
        }
        primes
    }

    /// Parse the node after a `_` or `^` token.
    fn get_sub_or_sub(
        &mut self,
        is_sup: bool,
    ) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        self.next_token(); // Discard the underscore or circumflex token.
        let next = self.next_token();
        if matches!(
            next.token(),
            Token::Underscore | Token::Circumflex | Token::Prime
        ) {
            return Err(LatexError(
                next.location(),
                LatexErrKind::CannotBeUsedHere {
                    got: next.into_token(),
                    correct_place: Place::AfterOpOrIdent,
                },
            ));
        }
        let node = self.parse_token(next, ParseAs::Arg, None);

        // If the bound was a superscript, it may *not* be followed by a prime.
        if is_sup && matches!(self.peek.token(), Token::Prime) {
            return Err(LatexError(
                self.peek.location(),
                LatexErrKind::CannotBeUsedHere {
                    got: Token::Prime,
                    correct_place: Place::AfterOpOrIdent,
                },
            ));
        }

        node
    }

    fn make_pseudo_operator(
        &mut self,
        name: &'arena str,
        prev_state: &SequenceState,
    ) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        Ok(self.commit(Node::PseudoOp {
            name,
            attr: None,
            left: if matches!(
                prev_state.class,
                Class::Relation | Class::Punctuation | Class::Operator | Class::Open
            ) {
                Some(MathSpacing::Zero)
            } else {
                Some(MathSpacing::ThreeMu)
            },
            right: if matches!(
                self.peek.token(),
                Token::Punctuation(_)
                    | Token::Relation(_)
                    | Token::Delimiter(_)
                    | Token::SquareBracketOpen
                    | Token::SquareBracketClose
            ) || (prev_state.real_boundaries
                && matches!(
                    self.peek.token(),
                    Token::EOF | Token::GroupEnd | Token::End | Token::Right
                )) {
                Some(MathSpacing::Zero)
            } else {
                Some(MathSpacing::ThreeMu)
            },
        }))
    }
}

#[inline]
fn next_token<'source>(peek: &mut TokLoc<'source>, lexer: &mut Lexer<'source>) -> TokLoc<'source> {
    let peek_token = lexer.next_token();
    // Return the previous peek token and store the new peek token.
    mem::replace(peek, peek_token)
}

// Turn a vector of nodes into a single node.
//
// This is done either by returning the single node if there is only one,
// or by creating a row node if there are multiple nodes.
pub(crate) fn node_vec_to_node<'arena>(
    arena: &'arena Arena,
    nodes: Vec<&'arena Node<'arena>>,
    reset_spacing: bool,
) -> &'arena Node<'arena> {
    if nodes.len() == 1 {
        // Safety: We checked that there is an element.
        let single = unsafe { nodes.into_iter().next().unwrap_unchecked() };
        if reset_spacing {
            if let Node::Operator {
                op,
                attr,
                left: _,
                right: _,
            } = single
            {
                arena.push(Node::Operator {
                    op: *op,
                    attr: *attr,
                    left: None,
                    right: None,
                })
            } else {
                single
            }
        } else {
            single
        }
    } else {
        let nodes = arena.push_slice(&nodes);
        arena.push(Node::Row {
            nodes,
            attr: RowAttr::None,
        })
    }
}

struct Bounds<'arena>(Option<&'arena Node<'arena>>, Option<&'arena Node<'arena>>);

enum LetterCollector<'arena> {
    Inactive,
    Collecting,
    FinishedOneLetter { collected_letter: char },
    FinishedManyLetters { collected_letters: &'arena str },
}

struct TextModeParser<'builder, 'source, 'parser> {
    builder: &'builder mut StringBuilder<'parser>,
    peek: &'parser mut TokLoc<'source>,
    lexer: &'parser mut Lexer<'source>,
    tf: Option<TextTransform>,
}

impl<'builder, 'source, 'parser> TextModeParser<'builder, 'source, 'parser> {
    fn new(
        builder: &'builder mut StringBuilder<'parser>,
        peek: &'parser mut TokLoc<'source>,
        lexer: &'parser mut Lexer<'source>,
    ) -> Self {
        Self {
            builder,
            peek,
            lexer,
            tf: None,
        }
    }

    fn next_token(&mut self) -> TokLoc<'source> {
        next_token(self.peek, self.lexer)
    }

    /// Parse the given token in text mode.
    ///
    /// This function may read in more tokens from the lexer, but it will always leave the last
    /// processed token in `peek`. This is important for turning of the text mode in the lexer at
    /// the right time.
    fn parse_token_in_text_mode(
        &mut self,
        tokloc: TokLoc<'source>,
    ) -> Result<(), LatexError<'source>> {
        let c: char = match tokloc.token() {
            Token::Letter(c) | Token::UprightLetter(c) => *c,
            Token::Whitespace | Token::NonBreakingSpace => '\u{A0}',
            Token::Delimiter(op) => (*op).into(),
            Token::BinaryOp(op) => op.as_op().into(),
            Token::Relation(op) => op.as_op().into(),
            Token::SquareBracketOpen => symbol::LEFT_SQUARE_BRACKET.into(),
            Token::Number(digit) => *digit as u8 as char,
            Token::Prime => '’',
            Token::Colon => ':',
            Token::Punctuation(op) => (*op).as_op().into(),
            Token::PseudoOperator(name) | Token::PseudoOperatorLimits(name) => {
                // We don't transform these strings.
                self.builder.push_str(name);
                return Ok(());
            }
            Token::Space(length) => {
                let length = *length;
                if length == Length::new(1.0, LengthUnit::Em) {
                    '\u{2003}'
                } else if length == LatexUnit::Mu.length_with_unit(5.0) {
                    '\u{2004}'
                } else if length == LatexUnit::Mu.length_with_unit(4.0) {
                    '\u{205F}'
                } else if length == LatexUnit::Mu.length_with_unit(3.0) {
                    '\u{2009}'
                } else {
                    return Ok(());
                }
            }
            Token::TextModeAccent(accent) => {
                // Discard `TextModeAccent` token.
                self.next_token();
                let tokloc = TokLoc(self.peek.location(), *self.peek.token());
                self.parse_token_in_text_mode(tokloc)?;
                self.builder.push_char(*accent);
                return Ok(());
            }
            Token::Text(tf) => {
                // Discard `Text` token.
                self.next_token();
                let old_tf = mem::replace(&mut self.tf, *tf);
                let tokloc = TokLoc(self.peek.location(), *self.peek.token());
                self.parse_token_in_text_mode(tokloc)?;
                self.tf = old_tf;
                return Ok(());
            }
            Token::GroupBegin => {
                // Discard opening token.
                self.next_token();
                while !self.peek.token().is_same_kind_as(&Token::GroupEnd) {
                    let tokloc = TokLoc(self.peek.location(), *self.peek.token());
                    self.parse_token_in_text_mode(tokloc)?;
                    // Discard the last token.
                    // We must do this here, because `parse_token_in_text_mode` always leaves the
                    // last token in `peek`, but we want to continue here, so we need to discard it.
                    self.next_token();
                }
                return Ok(());
            }
            Token::EOF => {
                return Err(LatexError(
                    tokloc.location(),
                    LatexErrKind::UnclosedGroup(Token::GroupEnd),
                ));
            }
            Token::End | Token::Right | Token::GroupEnd => {
                return Err(LatexError(
                    tokloc.location(),
                    LatexErrKind::UnexpectedClose(tokloc.into_token()),
                ));
            }
            Token::UnknownCommand(command) => {
                return Err(LatexError(
                    tokloc.location(),
                    LatexErrKind::UnknownCommand(command),
                ));
            }
            _ => {
                return Err(LatexError(
                    tokloc.location(),
                    LatexErrKind::NotValidInTextMode(tokloc.into_token()),
                ));
            }
        };
        self.builder
            .push_char(self.tf.map(|tf| tf.transform(c, false)).unwrap_or(c));
        Ok(())
    }
}

fn get_single_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c), // Exactly one char
        _ => None,                  // Zero or multiple chars
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_ron_snapshot;

    use super::*;

    #[test]
    fn ast_test() {
        let problems = [
            ("slightly_more_complex_fraction", r"\frac123"),
            ("frac_with_subscript", r"\frac12_x"),
            ("integral_with_reversed_limits", r"\int\limits^1_0 dx"),
            ("matrix", r"\begin{pmatrix} x \\ y \end{pmatrix}"),
            ("number_with_dot", r"3.14"),
            ("number_with_dot_at_end", r"3.14."),
            ("number_with_two_inner_dots", r"3..14"),
            ("number_with_dot_and_letter", r"4.x"),
            ("sqrt_number_with_dot", r"\sqrt{4.}"),
            ("sqrt_degree_and_number", r"\sqrt[3]21"),
            ("sqrt_subscript", r"\sqrt x_i"),
            ("sqrt_subscript_and_degree", r"\sqrt[3] x_i"),
            ("double_prime", r"f''"),
            ("textbf", r"\textbf{abc}"),
            ("mathit_greek", r"\mathit{\Alpha\Beta}"),
            ("mathrm_mathit_nested", r"\mathrm{\mathit{a}b}"),
            ("mathrm_mathit_nested_multi", r"\mathrm{ab\mathit{cd}ef}"),
            ("mathit_mathrm_nested", r"\mathit{\mathrm{a}b}"),
            ("mathit_of_max", r"\mathit{ab \max \alpha\beta}"),
            ("boldsymbol_greek_var", r"\boldsymbol{\Gamma\varGamma}"),
            ("mathit_func", r"\mathit{ab \log cd}"),
            ("mathrm_subscript", r"\mathrm{x_x y_y}"),
            ("mathrm_sqrt", r"\mathrm{\sqrt xy}"),
            ("big_paren", r"\big("),
            ("sub_big_paren", r"x_\big("),
            ("pmod_subscript", r"\pmod{3}_4"),
            ("sub_number", r"x_123"),
            ("text_number", r"\text123"),
            ("operatorname_number", r"\operatorname123"),
            ("number_after_underscore", r"x_12"),
            ("number_after_circumflex", r"x^12"),
            ("number_after_lim", r"\sum\limits_12"),
            ("number_after_overbrace", r"\overbrace12"),
            ("number_with_spaces", r"1 2  3    4"),
            ("number_with_spaces_with_dots", r"1 2. 3  ,  4"),
            ("number_with_spaces_in_text", r"\text{1 2  3    4}"),
            ("comment", "\\text{% comment}\n\\%as}"),
            ("colon_fusion_in_subscript", r"x_:\equiv, x_:="),
            ("colon_fusion_stop", r":2=:="),
            ("scriptstyle_without_braces", r"x\scriptstyle y"),
            (
                "displaystyle_ended_by_right",
                r"\left(\displaystyle \int\right)\int",
            ),
            (
                "displaystyle_ended_by_end",
                r"\begin{matrix}\sum\displaystyle\sum\end{matrix}",
            ),
            ("overset_digits", r"\overset12"),
            ("genfrac", r"\genfrac(){1pt}{0}{1}{2}"),
            ("mspace", r"\mspace{1mu}"),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let l = Lexer::new(problem, false, None);
            let mut p = Parser::new(l, &arena);
            let ast = p.parse().expect("Parsing failed");
            assert_ron_snapshot!(name, &ast, problem);
        }
    }
}
