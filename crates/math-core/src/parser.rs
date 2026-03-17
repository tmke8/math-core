use std::{fmt::Write as _, mem, num::NonZeroU16, ops::Range};

use mathml_renderer::{
    arena::{Arena, Buffer},
    ast::Node,
    attribute::{
        LetterAttr, MathSpacing, MathVariant, OpAttrs, ParenType, RowAttr, Style, TextTransform,
    },
    length::Length,
    symbol::{
        self, DelimiterSpacing, OpCategory, OrdCategory, RelCategory, StretchableOp, Stretchy,
    },
};
use rustc_hash::FxHashMap;

use crate::{
    character_class::Class,
    color_defs::get_color,
    commands::get_negated_op,
    environments::{Env, NumberedEnvState},
    error::{DelimiterModifier, LatexErrKind, LatexError, LimitedUsabilityToken, Place},
    lexer::{Lexer, recover_limited_ascii},
    specifications::{parse_column_specification, parse_length_specification},
    token::{EndToken, Mode, TokSpan, Token},
    token_queue::{MacroArgument, OneOrNone, TokenQueue},
};

pub(crate) struct Parser<'config, 'source, 'arena> {
    pub(super) tokens: TokenQueue<'config, 'source>,
    pub(super) buffer: Buffer,
    pub(super) arena: &'arena Arena,
    equation_counter: &'arena mut u16,
    label_map: &'arena mut FxHashMap<Box<str>, NonZeroU16>,
    state: ParserState<'source, 'arena>,
}

struct ParserState<'source, 'arena> {
    cmd_args: Vec<TokSpan<'source>>,
    cmd_arg_offsets: [usize; 9],
    transform: Option<MathVariant>,
    /// `true` if the boundaries at the end of a  sequence are not real boundaries;
    /// this is not the case for style-only rows.
    /// This is currently a hack, which should be replaced by a more robust solution later.
    right_boundary_hack: bool,
    /// `true` if we are inside an environment that allows columns (`&`).
    allow_columns: bool,
    /// `true` if we should treat newlines as meaningful (i.e., in `align` environments).
    meaningful_newlines: bool,
    numbered: Option<NumberedEnvState<'arena>>,
    /// `true` if we are within a group where the style is `\scriptstyle` or smaller
    script_style: bool,
}

#[derive(Debug, Clone, Copy)]
enum SequenceEnd {
    EndToken(EndToken),
    AnyEndToken,
}

impl SequenceEnd {
    #[inline]
    fn matches(self, other: &Token<'_>) -> bool {
        match self {
            SequenceEnd::EndToken(token) => token.matches(other),
            SequenceEnd::AnyEndToken => matches!(
                other,
                Token::Eoi | Token::GroupEnd | Token::End(_) | Token::Right
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ParseAs {
    /// A sequence starts with a fresh sequence state.
    Sequence,
    /// A continued sequence keeps the previous sequence state even if a new group is
    /// started.
    ContinueSequence,
    /// For an `Arg`, all spacing is ignored, so we may as well strip it away.
    Arg,
    /// For an `ArgWithSpace`, operator spacing is significant, so we have to be
    /// careful to set it correctly.
    ArgWithSpace,
}

impl ParseAs {
    #[inline]
    fn in_sequence(self) -> bool {
        matches!(self, ParseAs::Sequence | ParseAs::ContinueSequence)
    }
}

#[derive(Debug, Clone, Copy)]
enum ControlFlow {
    SkipToken,
    ProcessToken,
}

pub(super) type ParseResult<T> = Result<T, Box<LatexError>>;

impl<'config, 'source, 'arena> Parser<'config, 'source, 'arena>
where
    'config: 'source, // The config will live as long as the source.
    'source: 'arena,
{
    pub(crate) fn new(
        lexer: Lexer<'config, 'source>,
        arena: &'arena Arena,
        equation_counter: &'arena mut u16,
        label_map: &'arena mut FxHashMap<Box<str>, NonZeroU16>,
    ) -> ParseResult<Self> {
        let input_length = lexer.input_length();
        Ok(Parser {
            tokens: TokenQueue::new(lexer)?,
            buffer: Buffer::new(input_length),
            arena,
            equation_counter,
            label_map,
            state: ParserState {
                cmd_args: Vec::new(),
                cmd_arg_offsets: [0; 9],
                transform: None,
                right_boundary_hack: false,
                allow_columns: false,
                meaningful_newlines: false,
                numbered: None,
                script_style: false,
            },
        })
    }

    #[inline(never)]
    fn next_token(&mut self) -> ParseResult<TokSpan<'source>> {
        self.tokens.next()
    }

    #[inline]
    pub(crate) fn parse(&mut self) -> ParseResult<Vec<&'arena Node<'arena>>> {
        self.parse_sequence(SequenceEnd::EndToken(EndToken::Eoi), Class::Open, true)
    }

    /// Parse a sequence of tokens until the given end token is encountered.
    ///
    /// If `keep_end_token` is set to `true`, this function does not consume the end token.
    /// This is helpful in cases where the end token is used by the calling function to emit
    /// another node.
    fn parse_sequence(
        &mut self,
        sequence_end: SequenceEnd,
        prev_class: Class,
        keep_end_token: bool,
    ) -> ParseResult<Vec<&'arena Node<'arena>>> {
        let mut nodes = Vec::new();

        let mut prev_class = prev_class;
        let old_tf = self.state.transform;

        // Because we don't want to consume the end token, we just peek here.
        while !sequence_end.matches(self.tokens.peek().token()) {
            // Check whether we need to collect letters.
            let (class, target) = if let Some(collected) = self.merge_and_transform_letters()? {
                collected
            } else {
                // Get the current token.
                let cur_tokloc = self.next_token();
                if let Ok(tokloc) = &cur_tokloc {
                    match self.handle_tokens_without_output(tokloc, sequence_end)? {
                        ControlFlow::SkipToken => continue,
                        ControlFlow::ProcessToken => {}
                    }
                }
                // Parse the token.
                self.parse_token(cur_tokloc, ParseAs::Sequence, prev_class)?
            };
            prev_class = class;

            // Check if there are any superscripts or subscripts following the parsed node.
            let bounds = self.get_bounds()?;

            // If there are superscripts or subscripts, we need to wrap the node we just got into
            // one of the node types for superscripts and subscripts.
            let node = self.commit(match bounds {
                Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                Bounds(Some(symbol), None) => Node::Sub { target, symbol },
                Bounds(None, Some(symbol)) => Node::Sup { target, symbol },
                Bounds(None, None) => {
                    nodes.push(target);
                    continue;
                }
            });
            nodes.push(node);
        }
        if !keep_end_token {
            // Discard the end token.
            self.next_token()?;
        }
        self.state.transform = old_tf;
        Ok(nodes)
    }

    #[inline]
    fn handle_tokens_without_output(
        &mut self,
        tokspan: &TokSpan<'source>,
        sequence_end: SequenceEnd,
    ) -> ParseResult<ControlFlow> {
        let span = tokspan.span().into();
        let result: Result<(), LatexError> = match tokspan.token() {
            Token::Eoi => {
                if let SequenceEnd::EndToken(end_token) = sequence_end {
                    // The input has ended without the closing token.
                    Err(LatexError(span, LatexErrKind::UnclosedGroup(end_token)))
                } else {
                    return Ok(ControlFlow::ProcessToken);
                }
            }
            Token::TransformSwitch(tf) => {
                self.state.transform = Some(*tf);
                Ok(())
            }
            Token::NoNumber => {
                if let Some(numbered_state) = &mut self.state.numbered {
                    numbered_state.suppress_next_number = true;
                }
                Ok(())
            }
            Token::Tag => {
                let (tag_name, literal_span) = self.parse_string_literal()?;
                if let Some(numbered_state) = &mut self.state.numbered {
                    // For now, we only support numeric tags.
                    if let Ok(tag_num) = tag_name.trim().parse::<u16>()
                        && tag_num != 0
                    {
                        numbered_state.custom_next_number = NonZeroU16::new(tag_num);
                        Ok(())
                    } else {
                        Err(LatexError(
                            literal_span,
                            LatexErrKind::ExpectedNumber(tag_name.into()),
                        ))
                    }
                } else {
                    Err(LatexError(
                        span.into(),
                        LatexErrKind::CannotBeUsedHere {
                            got: LimitedUsabilityToken::Tag,
                            correct_place: Place::NumberedEnv,
                        },
                    ))
                }
            }
            Token::Label => {
                let (label_name, _) = self.parse_string_literal()?;
                if let Some(numbered_state) = &mut self.state.numbered {
                    if numbered_state.label.is_some() {
                        Err(LatexError(span.into(), LatexErrKind::MoreThanOneLabel))
                    } else {
                        numbered_state.label = Some(label_name);
                        Ok(())
                    }
                } else {
                    Err(LatexError(
                        span.into(),
                        LatexErrKind::CannotBeUsedHere {
                            got: LimitedUsabilityToken::Label,
                            correct_place: Place::NumberedEnv,
                        },
                    ))
                }
            }
            _ => {
                return Ok(ControlFlow::ProcessToken);
            }
        };
        match result {
            Ok(()) => Ok(ControlFlow::SkipToken),
            Err(e) => Err(Box::new(e)),
        }
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
        cur_tokloc: ParseResult<TokSpan<'source>>,
        parse_as: ParseAs,
        prev_class: Class,
    ) -> ParseResult<(Class, &'arena Node<'arena>)> {
        let (cur_token, span) = cur_tokloc?.into_parts();
        let mut class = Class::default();
        let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;
        let next_class = if self.state.right_boundary_hack && matches!(next_class, Class::End) {
            Class::Default
        } else {
            next_class
        };
        let node: Result<Node, LatexError> = match cur_token {
            Token::Digit(number) => 'digit: {
                if let Some(MathVariant::Transform(tf)) = self.state.transform {
                    break 'digit Ok(Node::IdentifierChar(
                        tf.transform(number, false),
                        LetterAttr::Default,
                    ));
                }
                let mut builder = self.buffer.get_builder();
                builder.push_char(number);
                if matches!(parse_as, ParseAs::Sequence) {
                    // Consume tokens as long as they are `Token::Number` or
                    // `Token::Letter('.')`,
                    // but the latter only if the token *after that* is a digit.
                    loop {
                        let ch = if let Token::Digit(number) = self.tokens.peek().token() {
                            *number
                        } else {
                            let ch = if matches!(
                                self.tokens.peek().token(),
                                Token::Letter('.', Mode::MathOrText)
                            ) {
                                Some('.')
                            } else {
                                None
                            };
                            if let Some(ch) = ch {
                                if matches!(self.tokens.peek_second()?.token(), Token::Digit(_)) {
                                    ch
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        };
                        builder.push_char(ch);
                        self.tokens.next()?;
                    }
                }
                Ok(Node::Number(builder.finish(self.arena)))
            }
            tok @ (Token::Letter(c, _) | Token::UprightLetter(c)) => {
                let mut is_upright = matches!(tok, Token::UprightLetter(_));
                let mut with_tf = false;
                let ch = if let Some(tf) = self.state.transform {
                    match tf {
                        MathVariant::Transform(tf) => {
                            with_tf = true;
                            tf.transform(c, is_upright)
                        }
                        MathVariant::Normal => {
                            is_upright = true;
                            c
                        }
                    }
                } else {
                    c
                };
                if let Some(MathVariant::Transform(
                    tf @ (TextTransform::ScriptChancery | TextTransform::ScriptRoundhand),
                )) = self.state.transform
                {
                    // We need to append Unicode variant selectors for these transforms.
                    let mut builder = self.buffer.get_builder();
                    builder.push_char(ch);
                    builder.push_char(if matches!(tf, TextTransform::ScriptChancery) {
                        '\u{FE00}' // VARIATION SELECTOR-1
                    } else {
                        '\u{FE01}' // VARIATION SELECTOR-2
                    });
                    Ok(Node::IdentifierStr(builder.finish(self.arena)))
                } else {
                    Ok(Node::IdentifierChar(
                        ch,
                        if is_upright && !with_tf {
                            LetterAttr::ForcedUpright
                        } else {
                            LetterAttr::Default
                        },
                    ))
                }
            }
            ref tok @ (Token::Relation(relation) | Token::StretchyRel(relation)) => {
                class = Class::Relation;
                let attr = match relation.category() {
                    // Category A relations are stretchy by default
                    RelCategory::A => {
                        // We let it be stretchy if it's explicitly marked as stretchy
                        if matches!(tok, Token::StretchyRel(_)) {
                            OpAttrs::empty()
                        } else {
                            OpAttrs::STRETCHY_FALSE
                        }
                    }
                    RelCategory::Default => OpAttrs::empty(),
                };
                let (left, right) = self.state.relation_spacing(prev_class, next_class);
                Ok(Node::Operator {
                    op: relation.as_op(),
                    attrs: attr,
                    left,
                    right,
                })
            }
            Token::Punctuation(punc) => {
                class = Class::Punctuation;
                let right = if matches!(next_class, Class::End) || self.state.script_style {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                Ok(Node::Operator {
                    op: punc.as_op(),
                    attrs: OpAttrs::empty(),
                    left: None,
                    right,
                })
            }
            Token::ForcePunctuation(op) => {
                class = Class::Punctuation;
                let right = if matches!(next_class, Class::End) || self.state.script_style {
                    Some(MathSpacing::Zero)
                } else {
                    Some(MathSpacing::ThreeMu)
                };
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left: Some(MathSpacing::Zero),
                    right,
                })
            }
            Token::Ord(ord) => {
                let attr = if matches!(ord.category(), OrdCategory::FGandForceDefault) {
                    // Category F+G operators will stretch in pre- and postfix positions,
                    // so we explicitly set the stretchy attribute to false to prevent that.
                    // Alternatively, we could set `form="infix"` on them.
                    OpAttrs::STRETCHY_FALSE
                } else {
                    OpAttrs::empty()
                };
                let (left, right) = if matches!(
                    ord.category(),
                    OrdCategory::KButUsedToBeB | OrdCategory::FGandForceDefault
                ) {
                    // Category B and ForceDefault have non-zero spacing.
                    // We suppress this by setting the spacing to zero.
                    (Some(MathSpacing::Zero), Some(MathSpacing::Zero))
                } else {
                    (None, None)
                };
                Ok(Node::Operator {
                    op: ord.as_op(),
                    attrs: attr,
                    left,
                    right,
                })
            }
            Token::BinaryOp(binary_op) => {
                class = Class::BinaryOp;
                let spacing = self.state.bin_op_spacing(
                    parse_as.in_sequence(),
                    prev_class,
                    next_class,
                    false,
                );
                Ok(Node::Operator {
                    op: binary_op.as_op(),
                    attrs: OpAttrs::empty(),
                    left: spacing,
                    right: spacing,
                })
            }
            Token::ForceBinaryOp(op) => {
                class = Class::BinaryOp;
                let spacing =
                    self.state
                        .bin_op_spacing(parse_as.in_sequence(), prev_class, next_class, true);
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left: spacing,
                    right: spacing,
                })
            }
            Token::Mathbin => 'mathbin: {
                let tokspan = match self.tokens.read_argument(false)?.into_one_or_none()? {
                    OneOrNone::One(tokspan) => tokspan,
                    OneOrNone::None(span) => {
                        break 'mathbin Err(LatexError(
                            span,
                            LatexErrKind::ExpectedAtLeastOneToken,
                        ));
                    }
                };
                let (tok, span) = tokspan.into_parts();
                let op = match tok {
                    Token::Ord(op) | Token::Open(op) | Token::Close(op) => op.as_op(),
                    Token::Op(op) | Token::Inner(op) => op.as_op(),
                    Token::BinaryOp(op) => op.as_op(),
                    Token::Relation(op) => op.as_op(),
                    Token::Punctuation(op) => op.as_op(),
                    Token::ForceRelation(op) | Token::ForceClose(op) | Token::ForceBinaryOp(op) => {
                        op
                    }
                    Token::SquareBracketOpen => symbol::LEFT_SQUARE_BRACKET.as_op(),
                    Token::SquareBracketClose => symbol::RIGHT_SQUARE_BRACKET.as_op(),
                    _ => {
                        break 'mathbin Err(LatexError(
                            span.into(),
                            LatexErrKind::ExpectedRelation,
                        ));
                    }
                };
                class = Class::BinaryOp;
                // Recompute the next class:
                let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;
                let spacing =
                    self.state
                        .bin_op_spacing(parse_as.in_sequence(), prev_class, next_class, true);
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::STRETCHY_FALSE,
                    left: spacing,
                    right: spacing,
                })
            }
            Token::Inner(op) => {
                class = Class::Inner;
                let left = if matches!(
                    prev_class,
                    Class::Relation
                        | Class::Punctuation
                        | Class::Operator
                        | Class::BinaryOp
                        | Class::Open
                ) || self.state.script_style
                {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                let right = if matches!(
                    next_class,
                    Class::Relation | Class::BinaryOp | Class::Close | Class::End
                ) || (self.state.script_style
                    && !matches!(next_class, Class::Operator))
                {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                Ok(Node::Operator {
                    op: op.as_op(),
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                })
            }
            Token::OpGreaterThan => {
                let (left, right) = self.state.relation_spacing(prev_class, next_class);
                class = Class::Relation;
                Ok(Node::PseudoOp {
                    name: "&gt;",
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                })
            }
            Token::OpLessThan => {
                let (left, right) = self.state.relation_spacing(prev_class, next_class);
                class = Class::Relation;
                Ok(Node::PseudoOp {
                    name: "&lt;",
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                })
            }
            Token::OpAmpersand => Ok(Node::PseudoOp {
                name: "&amp;",
                attrs: OpAttrs::empty(),
                left: None,
                right: None,
            }),
            Token::PseudoOperator(name) => {
                let (left, right) = self.big_operator_spacing(parse_as, prev_class, true)?;
                class = Class::Operator;
                Ok(Node::PseudoOp {
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    name,
                })
            }
            Token::Enclose(notation) => {
                let content = self.parse_next(ParseAs::ArgWithSpace)?;
                Ok(Node::Enclose { content, notation })
            }
            Token::Space(space) => {
                // Spaces pass through the symbol class.
                class = prev_class;
                Ok(Node::Space(space))
            }
            Token::CustomSpace => {
                let (length, span) = self.parse_string_literal()?;
                match parse_length_specification(length.trim()) {
                    Some(space) => Ok(Node::Space(space)),
                    None => Err(LatexError(
                        span,
                        LatexErrKind::ExpectedLength(length.into()),
                    )),
                }
            }
            Token::NonBreakingSpace => Ok(Node::Text(None, "\u{A0}")),
            Token::Sqrt => {
                let next = self.next_token();
                if let Ok(tokloc) = next
                    && matches!(tokloc.token(), Token::SquareBracketOpen)
                {
                    // FIXME: We should perhaps use set `right_boundary_hack` here.
                    let degree = self.parse_sequence(
                        SequenceEnd::EndToken(EndToken::SquareBracketClose),
                        Class::Open,
                        false,
                    )?;
                    let content = self.parse_next(ParseAs::Arg)?;
                    Ok(Node::Root(
                        node_vec_to_node(self.arena, &degree, true),
                        content,
                    ))
                } else {
                    Ok(Node::Sqrt(
                        self.parse_token(next, ParseAs::Arg, Class::Default)?.1,
                    ))
                }
            }
            Token::Frac(attr) | Token::Binom(attr) => {
                let num = self.parse_next(ParseAs::Arg)?;
                let denom = self.parse_next(ParseAs::Arg)?;
                if matches!(cur_token, Token::Binom(_)) {
                    let (lt_value, lt_unit) = Length::zero().into_parts();
                    const OPEN_PAREN: StretchableOp =
                        symbol::LEFT_PARENTHESIS.as_stretchable_op().unwrap();
                    const CLOSE_PAREN: StretchableOp =
                        symbol::RIGHT_PARENTHESIS.as_stretchable_op().unwrap();
                    Ok(Node::Fenced {
                        open: Some(OPEN_PAREN),
                        close: Some(CLOSE_PAREN),
                        content: self.commit(Node::Frac {
                            num,
                            denom,
                            lt_value,
                            lt_unit,
                            attr,
                        }),
                        style: None,
                    })
                } else {
                    let (lt_value, lt_unit) = Length::none().into_parts();
                    Ok(Node::Frac {
                        num,
                        denom,
                        lt_value,
                        lt_unit,
                        attr,
                    })
                }
            }
            Token::Genfrac => 'genfrac: {
                fn get_delimiter<'config, 'source, 'arena>(
                    parser: &mut Parser<'config, 'source, 'arena>,
                ) -> Result<Option<StretchableOp>, Box<LatexError>>
                where
                    'config: 'source,
                    'source: 'arena,
                {
                    let tok = parser.tokens.read_argument(false)?.into_one_or_none()?;
                    Ok(match tok {
                        OneOrNone::One(tok) => {
                            Some(extract_delimiter(tok, DelimiterModifier::Genfrac)?)
                        }
                        OneOrNone::None(_) => None,
                    })
                }
                let open = get_delimiter(self)?;
                let close = get_delimiter(self)?;
                let (length, span) = self.parse_string_literal()?;
                let lt = match length.trim() {
                    "" => Length::none(),
                    decimal => parse_length_specification(decimal).ok_or_else(|| {
                        Box::new(LatexError(
                            span,
                            LatexErrKind::ExpectedLength(decimal.into()),
                        ))
                    })?,
                };
                let style_token: Option<TokSpan> =
                    self.tokens.read_argument(false)?.into_one_or_none()?.into();
                let style = if let Some(tokspan) = style_token {
                    if let Token::Digit(num) = tokspan.token() {
                        match num {
                            '0' => Some(Style::Display),
                            '1' => Some(Style::Text),
                            '2' => Some(Style::Script),
                            '3' => Some(Style::ScriptScript),
                            _ => {
                                break 'genfrac Err(LatexError(
                                    tokspan.span().into(),
                                    LatexErrKind::ExpectedArgumentGotEOI,
                                ));
                            }
                        }
                    } else {
                        break 'genfrac Err(LatexError(
                            tokspan.span().into(),
                            LatexErrKind::ExpectedArgumentGotEOI,
                        ));
                    }
                } else {
                    None
                };
                let num = self.parse_next(ParseAs::Arg)?;
                let denom = self.parse_next(ParseAs::Arg)?;
                let attr = None;
                let (lt_value, lt_unit) = lt.into_parts();
                Ok(Node::Fenced {
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
                })
            }
            Token::Accent(op, is_over, attr) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                if is_over {
                    Ok(Node::OverAccent(op.as_op(), attr, target))
                } else {
                    Ok(Node::UnderAccent(op.as_op(), target))
                }
            }
            Token::Overset | Token::Underset => {
                let old_script_style = mem::replace(&mut self.state.script_style, true);
                let symbol = self.parse_next(ParseAs::Arg)?;
                self.state.script_style = old_script_style;
                let token = self.next_token();
                let old_boundary_hack = mem::replace(&mut self.state.right_boundary_hack, true);
                let (cls, target) =
                    self.parse_token(token, ParseAs::ContinueSequence, prev_class)?;
                self.state.right_boundary_hack = old_boundary_hack;
                class = cls;
                if matches!(cur_token, Token::Overset) {
                    Ok(Node::Over { symbol, target })
                } else {
                    Ok(Node::Under { symbol, target })
                }
            }
            Token::OverUnderBrace(x, is_over) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                let symbol = self.commit(Node::Operator {
                    op: x.as_op(),
                    attrs: OpAttrs::empty(),
                    left: None,
                    right: None,
                });
                let base = if is_over {
                    Node::Over { symbol, target }
                } else {
                    Node::Under { symbol, target }
                };
                if (is_over && matches!(self.tokens.peek().token(), Token::Circumflex))
                    || (!is_over && matches!(self.tokens.peek().token(), Token::Underscore))
                {
                    let target = self.commit(base);
                    self.next_token()?; // Discard the circumflex or underscore token.
                    let expl = self.parse_next(ParseAs::Arg)?;
                    if is_over {
                        Ok(Node::Over {
                            symbol: expl,
                            target,
                        })
                    } else {
                        Ok(Node::Under {
                            symbol: expl,
                            target,
                        })
                    }
                } else {
                    Ok(base)
                }
            }
            Token::Op(op) => {
                class = Class::Operator;
                let has_movable_limits = matches!(op.category(), OpCategory::J);
                let has_bounds = !matches!(op.category(), OpCategory::C);

                let limits = has_bounds && matches!(self.tokens.peek().token(), Token::Limits);
                if limits {
                    self.next_token()?; // Discard the limits token.
                }
                let bounds = if has_bounds {
                    self.get_bounds()?
                } else {
                    Bounds(None, None)
                };
                let (left, right) = self.big_operator_spacing(parse_as, prev_class, false)?;
                let attr = if has_movable_limits && limits {
                    OpAttrs::NO_MOVABLE_LIMITS
                } else {
                    OpAttrs::empty()
                };
                let target = self.commit(Node::Operator {
                    op: op.as_op(),
                    attrs: attr,
                    left,
                    right,
                });
                let use_underover = has_movable_limits || limits;
                if use_underover {
                    match bounds {
                        Bounds(Some(under), Some(over)) => Ok(Node::UnderOver {
                            target,
                            under,
                            over,
                        }),
                        Bounds(Some(symbol), None) => Ok(Node::Under { target, symbol }),
                        Bounds(None, Some(symbol)) => Ok(Node::Over { target, symbol }),
                        Bounds(None, None) => return Ok((class, target)),
                    }
                } else {
                    match bounds {
                        Bounds(Some(sub), Some(sup)) => Ok(Node::SubSup { target, sub, sup }),
                        Bounds(Some(symbol), None) => Ok(Node::Sub { target, symbol }),
                        Bounds(None, Some(symbol)) => Ok(Node::Sup { target, symbol }),
                        Bounds(None, None) => return Ok((class, target)),
                    }
                }
            }
            Token::PseudoOperatorLimits(name) => {
                let movablelimits = if matches!(self.tokens.peek().token(), Token::Limits) {
                    self.next_token()?; // Discard the limits token.
                    OpAttrs::NO_MOVABLE_LIMITS
                } else {
                    OpAttrs::FORCE_MOVABLE_LIMITS
                };
                class = Class::Operator;
                let bounds = self.get_bounds()?;
                // Compute spacing after getting the bounds, so that we don't
                // consider tokens that are part of the bounds for spacing calculations.
                let (left, right) = self.big_operator_spacing(parse_as, prev_class, true)?;
                let op = self.commit(Node::PseudoOp {
                    attrs: if matches!(bounds, Bounds(None, None)) {
                        OpAttrs::empty()
                    } else {
                        movablelimits
                    },
                    left,
                    right,
                    name,
                });
                let node = match bounds {
                    Bounds(Some(under), Some(over)) => Node::UnderOver {
                        target: op,
                        under,
                        over,
                    },
                    Bounds(Some(symbol), None) => Node::Under { target: op, symbol },
                    Bounds(None, Some(symbol)) => Node::Over { target: op, symbol },
                    Bounds(None, None) => {
                        return Ok((class, op));
                    }
                };
                Ok(node)
            }
            Token::Slashed => {
                let node = self.parse_next(ParseAs::Arg)?;
                Ok(Node::Slashed(node))
            }
            Token::Not => {
                // `\not` has to be followed by something:
                let (tok, new_span) = self.next_token()?.into_parts();
                // Recompute the next class:
                let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;
                match tok {
                    Token::Relation(op) => {
                        let (left, right) = self.state.relation_spacing(prev_class, next_class);
                        if let Some(negated) = get_negated_op(op) {
                            Ok(Node::Operator {
                                op: negated.as_op(),
                                attrs: OpAttrs::empty(),
                                left,
                                right,
                            })
                        } else {
                            Ok(Node::Operator {
                                op: op.as_op(),
                                attrs: OpAttrs::empty(),
                                left,
                                right,
                            })
                        }
                    }
                    tok @ (Token::OpLessThan | Token::OpGreaterThan) => {
                        let (left, right) = self.state.relation_spacing(prev_class, next_class);
                        Ok(Node::Operator {
                            op: if matches!(tok, Token::OpLessThan) {
                                symbol::NOT_LESS_THAN.as_op()
                            } else {
                                symbol::NOT_GREATER_THAN.as_op()
                            },
                            attrs: OpAttrs::empty(),
                            left,
                            right,
                        })
                    }
                    // We have to special-case `\exists` here because it is not a relation.
                    Token::Ord(symbol::THERE_EXISTS) => Ok(Node::Operator {
                        op: symbol::THERE_DOES_NOT_EXIST.as_op(),
                        attrs: OpAttrs::empty(),
                        left: None,
                        right: None,
                    }),
                    Token::Letter(char, _) | Token::UprightLetter(char) => {
                        let mut builder = self.buffer.get_builder();
                        builder.push_char(char);
                        builder.push_char('\u{338}');
                        Ok(Node::IdentifierStr(builder.finish(self.arena)))
                    }
                    _ => Err(LatexError(new_span.into(), LatexErrKind::ExpectedRelation)),
                }
            }
            Token::Transform(tf) => {
                let old_tf = self.state.transform.replace(tf);
                let content = self.parse_next(ParseAs::Arg)?;
                self.state.transform = old_tf;
                return Ok((Class::Close, content));
            }
            Token::TransformSwitch(_) | Token::NoNumber | Token::Tag | Token::Label => Err(
                LatexError(span.into(), LatexErrKind::CannotBeUsedAsArgument),
            ),
            Token::ForceRelation(op) => {
                class = Class::Relation;
                let (left, right) = if parse_as.in_sequence() {
                    let (left, right) = self.state.relation_spacing(prev_class, next_class);
                    // We have to turn `None` into explicit relation spacing.
                    (
                        left.or(Some(MathSpacing::FiveMu)),
                        right.or(Some(MathSpacing::FiveMu)),
                    )
                } else {
                    // Don't add spacing if we are in an argument.
                    (None, None)
                };
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                })
            }
            Token::ForceClose(op) => {
                class = Class::Close;
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left: None,
                    right: None,
                })
            }
            Token::GroupBegin => {
                let content = self.parse_sequence(
                    SequenceEnd::EndToken(EndToken::GroupClose),
                    if matches!(parse_as, ParseAs::ContinueSequence) {
                        prev_class
                    } else {
                        Class::Open
                    },
                    false,
                )?;
                return Ok((
                    Class::Default,
                    node_vec_to_node(self.arena, &content, matches!(parse_as, ParseAs::Arg)),
                ));
            }
            ref tok @ (Token::Open(paren) | Token::Close(paren)) => {
                let open = matches!(tok, Token::Open(_));
                if open {
                    class = Class::Open;
                }
                let mut attr = if matches!(paren.category(), OrdCategory::FGandForceDefault) {
                    // For this category of symbol, we have to force the form attribute
                    // in order to get correct spacing.
                    if open {
                        OpAttrs::FORM_PREFIX
                    } else {
                        OpAttrs::FORM_POSTFIX
                    }
                } else {
                    OpAttrs::empty()
                };
                if matches!(
                    paren.category(),
                    OrdCategory::F | OrdCategory::G | OrdCategory::FGandForceDefault
                ) {
                    // Symbols from these categories are automatically stretchy,
                    // so we have to explicitly disable that here.
                    attr |= OpAttrs::STRETCHY_FALSE;
                }
                Ok(Node::Operator {
                    op: paren.as_op(),
                    attrs: attr,
                    left: None,
                    right: None,
                })
            }
            Token::SquareBracketOpen => {
                class = Class::Open;
                Ok(Node::Operator {
                    op: symbol::LEFT_SQUARE_BRACKET.as_op(),
                    attrs: OpAttrs::STRETCHY_FALSE,
                    left: None,
                    right: None,
                })
            }
            Token::SquareBracketClose => Ok(Node::Operator {
                op: symbol::RIGHT_SQUARE_BRACKET.as_op(),
                attrs: OpAttrs::STRETCHY_FALSE,
                left: None,
                right: None,
            }),
            Token::Left => {
                let tok_loc = self.next_token()?;
                let open_paren = if matches!(tok_loc.token(), Token::Letter('.', Mode::MathOrText))
                {
                    None
                } else {
                    Some(extract_delimiter(tok_loc, DelimiterModifier::Left)?)
                };
                let content = self.parse_sequence(
                    SequenceEnd::EndToken(EndToken::Right),
                    Class::Open,
                    false,
                )?;
                let tok_loc = self.next_token()?;
                let close_paren = if matches!(tok_loc.token(), Token::Letter('.', Mode::MathOrText))
                {
                    None
                } else {
                    Some(extract_delimiter(tok_loc, DelimiterModifier::Right)?)
                };
                Ok(Node::Fenced {
                    open: open_paren,
                    close: close_paren,
                    content: node_vec_to_node(self.arena, &content, false),
                    style: None,
                })
            }
            Token::Middle => {
                let tok_loc = self.next_token()?;
                let op = extract_delimiter(tok_loc, DelimiterModifier::Middle)?;
                let spacing = if matches!(op.spacing, DelimiterSpacing::Zero) {
                    None
                } else {
                    Some(MathSpacing::Zero)
                };
                Ok(Node::Operator {
                    op: op.as_op(),
                    attrs: middle_stretch_attrs(op),
                    left: spacing,
                    right: spacing,
                })
            }
            Token::Big(size, paren_type) => {
                let tok_loc = self.next_token()?;
                let paren = extract_delimiter(tok_loc, DelimiterModifier::Big)?;
                // `\big` commands without the "l" or "r" really produce `Class::Default`.
                class = match paren_type {
                    Some(ParenType::Open) => Class::Open,
                    Some(ParenType::Close) => Class::Close,
                    None => Class::Default,
                };
                Ok(Node::SizedParen(size, paren, paren_type))
            }
            Token::Begin(env) => 'begin_env: {
                let array_spec = if matches!(env, Env::Array | Env::Subarray) {
                    // Parse the array options.
                    let (options, span) = self.parse_string_literal()?;
                    let Some(mut spec) = parse_column_specification(options, self.arena) else {
                        break 'begin_env Err(LatexError(
                            span,
                            LatexErrKind::ExpectedColSpec(options.into()),
                        ));
                    };
                    if matches!(env, Env::Subarray) {
                        spec.is_sub = true;
                    }
                    Some(self.arena.alloc_array_spec(spec))
                } else {
                    None
                };

                let old_allow_columns =
                    mem::replace(&mut self.state.allow_columns, env.allows_columns());
                let old_meaningful_newlines = mem::replace(
                    &mut self.state.meaningful_newlines,
                    env.meaningful_newlines(),
                );
                let old_script_style =
                    mem::replace(&mut self.state.script_style, matches!(env, Env::Subarray));
                let old_numbered =
                    mem::replace(&mut self.state.numbered, env.get_numbered_env_state());

                let content = self.arena.push_slice(&self.parse_sequence(
                    SequenceEnd::EndToken(EndToken::End),
                    Class::Open,
                    true, // keep_end_token
                )?);

                self.state.allow_columns = old_allow_columns;
                self.state.meaningful_newlines = old_meaningful_newlines;
                self.state.script_style = old_script_style;
                let numbered_state = mem::replace(&mut self.state.numbered, old_numbered);

                // Get the \end{env} token in order to verify that it matches the \begin{env}.
                let (end_env, end_span) = self.next_token()?.into_parts();
                let Token::End(end_env) = end_env else {
                    // This should never happen because `parse_sequence` should have
                    // stopped at the `\end` token.
                    // We report an internal error here.
                    break 'begin_env Err(LatexError(end_span.into(), LatexErrKind::Internal));
                };

                if end_env != env {
                    break 'begin_env Err(LatexError(
                        end_span.into(),
                        LatexErrKind::MismatchedEnvironment {
                            expected: env,
                            got: end_env,
                        },
                    ));
                }

                let (last_equation_num, num_rows) = if let Some(mut n) = numbered_state {
                    match n.next_equation_number(self.equation_counter, true) {
                        Ok(num) => (num, n.num_rows),
                        Err(()) => {
                            break 'begin_env Err(LatexError(
                                span.into(),
                                LatexErrKind::HardLimitExceeded,
                            ));
                        }
                    }
                } else {
                    (None, None)
                };
                class = Class::Close;

                Ok(
                    env.construct_node(
                        content,
                        array_spec,
                        self.arena,
                        last_equation_num,
                        num_rows,
                    ),
                )
            }
            Token::OperatorName(with_limits) => {
                let snippets = self.extract_text(None, false)?;
                let mut builder = self.buffer.get_builder();
                for (_style, text) in snippets {
                    builder.push_str(text);
                }
                let letters = builder.finish(self.arena);
                let (left, right) = self.big_operator_spacing(parse_as, prev_class, true)?;
                let op = self.commit(Node::PseudoOp {
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    name: letters,
                });
                if with_limits {
                    let node = match self.get_bounds()? {
                        Bounds(Some(under), Some(over)) => Node::UnderOver {
                            target: op,
                            under,
                            over,
                        },
                        Bounds(Some(symbol), None) => Node::Under { target: op, symbol },
                        Bounds(None, Some(symbol)) => Node::Over { target: op, symbol },
                        Bounds(None, None) => {
                            return Ok((Class::Operator, op));
                        }
                    };
                    Ok(node)
                } else {
                    return Ok((Class::Operator, op));
                }
            }
            Token::Text(transform) => {
                let snippets = self.extract_text(transform, true)?;
                let nodes = snippets
                    .into_iter()
                    .map(|(style, text)| self.commit(Node::Text(style, text)))
                    .collect::<Vec<_>>();
                return Ok((Class::Close, node_vec_to_node(self.arena, &nodes, false)));
            }
            Token::NewColumn => {
                if self.state.allow_columns {
                    class = Class::Close;
                    Ok(Node::ColumnSeparator)
                } else {
                    Err(LatexError(
                        span.into(),
                        LatexErrKind::CannotBeUsedHere {
                            got: LimitedUsabilityToken::Ampersand,
                            correct_place: Place::TableEnv,
                        },
                    ))
                }
            }
            Token::NewLine => 'new_line: {
                if !self.state.meaningful_newlines {
                    // TODO: Return something other than a row here, so that we can avoid creating
                    //       empty rows in places where they are not needed.
                    Ok(Node::Row {
                        nodes: &[],
                        attr: None,
                    })
                } else if let Some(numbered_state) = &mut self.state.numbered {
                    if let Some(row_counter) = &mut numbered_state.num_rows {
                        match row_counter.checked_add(1) {
                            Some(new_counter) => {
                                *row_counter = new_counter;
                            }
                            None => {
                                break 'new_line Err(LatexError(
                                    span.into(),
                                    LatexErrKind::HardLimitExceeded,
                                ));
                            }
                        }
                    }
                    match numbered_state.next_equation_number(self.equation_counter, false) {
                        Ok(tag) => {
                            let link_target = numbered_state.label.take();
                            if let Some(label) = link_target
                                && let Some(tag) = tag
                            {
                                self.label_map.insert(label.into(), tag);
                            }
                            Ok(Node::RowSeparator { tag, link_target })
                        }
                        Err(()) => Err(LatexError(span.into(), LatexErrKind::HardLimitExceeded)),
                    }
                } else {
                    Ok(Node::RowSeparator {
                        tag: None,
                        link_target: None,
                    })
                }
            }
            Token::EqRef => 'eqref: {
                let (label_name, literal_span) = self.parse_string_literal()?;
                let Some(tag) = self.label_map.get(label_name) else {
                    break 'eqref Err(LatexError(
                        literal_span,
                        LatexErrKind::UndefinedLabel(label_name.into()),
                    ));
                };
                let mut builder = self.buffer.get_builder();
                let _ = write!(builder, r##"<a href="#{label_name}">({tag})</a>"##);
                Ok(Node::Text(None, builder.finish(self.arena)))
            }
            Token::Color => 'color: {
                let (color_name, span) = self.parse_string_literal()?;
                let Some(color) = get_color(color_name) else {
                    break 'color Err(LatexError(
                        span,
                        LatexErrKind::UnknownColor(color_name.into()),
                    ));
                };
                let content = self.parse_sequence(SequenceEnd::AnyEndToken, prev_class, true)?;
                Ok(Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attr: Some(color),
                })
            }
            Token::Style(style) => {
                let old_script_style = mem::replace(
                    &mut self.state.script_style,
                    matches!(style, Style::Script | Style::ScriptScript),
                );
                let content = self.parse_sequence(SequenceEnd::AnyEndToken, prev_class, true)?;
                self.state.script_style = old_script_style;
                Ok(Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attr: Some(RowAttr::Style(style)),
                })
            }
            Token::Prime => {
                let target = self.commit(Node::Row {
                    nodes: &[],
                    attr: None,
                });
                let symbol = self.commit(Node::Operator {
                    op: symbol::PRIME.as_op(),
                    attrs: OpAttrs::empty(),
                    left: None,
                    right: None,
                });
                Ok(Node::Sup { target, symbol })
            }
            tok @ (Token::Underscore | Token::Circumflex) => {
                let symbol = self.parse_next(ParseAs::Arg)?;
                if matches!(
                    self.tokens.peek().token(),
                    Token::Eoi | Token::GroupEnd | Token::End(_)
                ) {
                    // If nothing follows the sub- or superscript, we use an empty row as the base.
                    let empty_row = self.commit(Node::Row {
                        nodes: &[],
                        attr: None,
                    });
                    if matches!(tok, Token::Underscore) {
                        Ok(Node::Sub {
                            target: empty_row,
                            symbol,
                        })
                    } else {
                        Ok(Node::Sup {
                            target: empty_row,
                            symbol,
                        })
                    }
                } else {
                    // If something follows the sub- or superscript, we parse it as a sequence and
                    // use it as the base.
                    let base = self.parse_next(ParseAs::Sequence)?;
                    let (sub, sup) = if matches!(tok, Token::Underscore) {
                        (Some(symbol), None)
                    } else {
                        (None, Some(symbol))
                    };
                    Ok(Node::Multiscript { base, sub, sup })
                }
            }
            Token::Limits => Err(LatexError(
                span.into(),
                LatexErrKind::CannotBeUsedHere {
                    got: LimitedUsabilityToken::Limits,
                    correct_place: Place::AfterBigOp,
                },
            )),
            Token::Eoi => Err(LatexError(
                span.into(),
                LatexErrKind::ExpectedArgumentGotEOI,
            )),
            tok @ (Token::End(_) | Token::Right | Token::GroupEnd) => {
                if parse_as.in_sequence() {
                    let end = match tok {
                        Token::GroupEnd => EndToken::GroupClose,
                        Token::Right => EndToken::Right,
                        Token::End(_) => EndToken::End,
                        _ => unreachable!(),
                    };
                    Err(LatexError(span.into(), LatexErrKind::UnmatchedClose(end)))
                } else {
                    Err(LatexError(
                        span.into(),
                        LatexErrKind::ExpectedArgumentGotClose,
                    ))
                }
            }
            Token::Whitespace | Token::MathOrTextMode(_, _) => {
                // These tokens should have been skipped.
                // We report an internal error here.
                Err(LatexError(span.into(), LatexErrKind::Internal))
            }
            Token::TextMode(_) => Err(LatexError(span.into(), LatexErrKind::NotValidInMathMode)),
            Token::CustomCmd(num_args, token_stream) => {
                if num_args > 0 {
                    // The fact that we only clear for `num_args > 0` is a hack to
                    // allow zero-argument token streams to be used within
                    // non-zero-argument token streams.
                    self.state.cmd_args.clear();
                }
                for arg_num in 0..num_args {
                    let tokloc = self.next_token()?;
                    if matches!(tokloc.token(), Token::GroupBegin) {
                        self.tokens.record_group(&mut self.state.cmd_args, true)?;
                    } else {
                        self.state.cmd_args.push(tokloc);
                    }
                    if let Some(offset) = self.state.cmd_arg_offsets.get_mut(arg_num as usize) {
                        *offset = self.state.cmd_args.len();
                    }
                }
                self.tokens.queue_in_front(token_stream);
                let token = self.next_token();
                // TODO: Use `become` here once it is stable.
                return self.parse_token(token, parse_as, prev_class);
            }
            Token::CustomCmdArg(arg_num) => {
                let start = self
                    .state
                    .cmd_arg_offsets
                    .get(arg_num.wrapping_sub(1) as usize)
                    .copied()
                    .unwrap_or(0);
                let end = self
                    .state
                    .cmd_arg_offsets
                    .get(arg_num as usize)
                    .copied()
                    .unwrap_or(self.state.cmd_args.len());
                if let Some(arg) = self.state.cmd_args.get(start..end) {
                    self.tokens.queue_in_front(arg);
                    let token = self.next_token();
                    return self.parse_token(token, parse_as, prev_class);
                } else {
                    // We somehow cannot find the requested argument.
                    Err(LatexError(span.into(), LatexErrKind::Internal))
                }
            }
            Token::UnknownCommand(name) => Ok(Node::UnknownCommand(name)),
            Token::HardcodedMathML(mathml) => Ok(Node::HardcodedMathML(mathml)),
            Token::InternalStringLiteral(content) => {
                if let Some(MathVariant::Transform(tf)) = self.state.transform {
                    let mut builder = self.buffer.get_builder();
                    for c in content.chars() {
                        builder.push_char(tf.transform(c, false));
                    }
                    Ok(Node::IdentifierStr(builder.finish(self.arena)))
                } else {
                    Ok(Node::IdentifierStr(content))
                }
            }
        };
        match node {
            Ok(n) => Ok((class, self.commit(n))),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Same as `parse_token`, but also gets the next token.
    #[inline]
    fn parse_next(&mut self, parse_as: ParseAs) -> ParseResult<&'arena Node<'arena>> {
        let token = self.next_token();
        self.parse_token(token, parse_as, Class::Default)
            .map(|(_, node)| node)
    }

    /// Parse the bounds of an integral, sum, or product.
    /// These bounds are preceeded by `_` or `^`.
    fn get_bounds(&mut self) -> ParseResult<Bounds<'arena>> {
        let mut primes = self.prime_check()?;
        // Check whether the first bound is specified and is a lower bound.
        let first_underscore = matches!(self.tokens.peek().token(), Token::Underscore);
        let first_circumflex = matches!(self.tokens.peek().token(), Token::Circumflex);

        let (sub, sup) = if first_underscore || first_circumflex {
            let first_bound = Some(self.get_sub_or_sup(first_circumflex)?);

            // If the first bound was a subscript *and* we didn't encounter primes yet,
            // we check once more for primes.
            if first_underscore && primes.is_empty() {
                primes = self.prime_check()?;
            }

            // Check whether both an upper and a lower bound were specified.
            let second_underscore = matches!(self.tokens.peek().token(), Token::Underscore);
            let second_circumflex = matches!(self.tokens.peek().token(), Token::Circumflex);

            if (first_circumflex && second_circumflex) || (first_underscore && second_underscore) {
                let span = self.next_token()?.span();
                return Err(Box::new(LatexError(
                    span.into(),
                    LatexErrKind::DuplicateSubOrSup,
                )));
            }

            if (first_underscore && second_circumflex) || (first_circumflex && second_underscore) {
                let second_bound = Some(self.get_sub_or_sup(second_circumflex)?);
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

        let sup = if primes.is_empty() {
            sup
        } else {
            if let Some(sup) = sup {
                primes.push(sup);
            }
            Some(node_vec_to_node(self.arena, &primes, false))
        };

        Ok(Bounds(sub, sup))
    }

    /// Check for primes and aggregate them into a single node.
    fn prime_check(&mut self) -> ParseResult<Vec<&'arena Node<'arena>>> {
        let mut primes = Vec::new();
        let mut prime_count = 0usize;
        while matches!(self.tokens.peek().token(), Token::Prime) {
            self.next_token()?; // Discard the prime token.
            prime_count += 1;
        }
        static PRIME_SELECTION: [symbol::OrdLike; 4] = [
            symbol::PRIME,
            symbol::DOUBLE_PRIME,
            symbol::TRIPLE_PRIME,
            symbol::QUADRUPLE_PRIME,
        ];
        if prime_count > 0 {
            // If we have between 1 and 4 primes, we can use the predefined prime operators.
            if let Some(op) = PRIME_SELECTION.get(prime_count - 1) {
                primes.push(self.commit(Node::Operator {
                    op: op.as_op(),
                    attrs: OpAttrs::empty(),
                    left: None,
                    right: None,
                }));
            } else {
                for _ in 0..prime_count {
                    primes.push(self.commit(Node::Operator {
                        op: symbol::PRIME.as_op(),
                        attrs: OpAttrs::empty(),
                        left: None,
                        right: None,
                    }));
                }
            }
        }
        Ok(primes)
    }

    /// Parse the node after a `_` or `^` token.
    fn get_sub_or_sup(&mut self, is_sup: bool) -> ParseResult<&'arena Node<'arena>> {
        self.next_token()?; // Discard the underscore or circumflex token.
        let next = self.next_token();
        if let Ok(tokloc) = next
            && let (Token::Underscore | Token::Circumflex | Token::Prime, span) =
                tokloc.into_parts()
        {
            return Err(Box::new(LatexError(
                span.into(),
                LatexErrKind::BoundFollowedByBound,
            )));
        }
        let old_script_style = mem::replace(&mut self.state.script_style, true);
        let node = self.parse_token(next, ParseAs::Arg, Class::Default);
        self.state.script_style = old_script_style;

        // If the bound was a superscript, it may *not* be followed by a prime.
        if is_sup && matches!(self.tokens.peek().token(), Token::Prime) {
            return Err(Box::new(LatexError(
                self.tokens.peek().span().into(),
                LatexErrKind::DuplicateSubOrSup,
            )));
        }

        node.map(|(_, n)| n)
    }

    fn big_operator_spacing(
        &mut self,
        parse_as: ParseAs,
        prev_class: Class,
        explicit: bool,
    ) -> ParseResult<(Option<MathSpacing>, Option<MathSpacing>)> {
        // We re-determine the next class here, because the next token may have changed
        // because we discarded bounds tokens.
        let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;
        Ok((
            if matches!(
                prev_class,
                Class::Relation
                    | Class::Punctuation
                    | Class::Operator
                    | Class::Inner
                    | Class::BinaryOp
                    | Class::Open
            ) {
                Some(MathSpacing::Zero)
            } else if explicit {
                Some(MathSpacing::ThreeMu)
            } else {
                None
            },
            if matches!(
                next_class,
                Class::Relation | Class::Punctuation | Class::Open | Class::Close | Class::End
            ) || (self.state.script_style && matches!(next_class, Class::Inner))
            {
                Some(MathSpacing::Zero)
            } else if explicit {
                Some(MathSpacing::ThreeMu)
            } else {
                None
            },
        ))
    }

    #[inline]
    fn merge_and_transform_letters(
        &mut self,
    ) -> ParseResult<Option<(Class, &'arena Node<'arena>)>> {
        let Some(tf) = self.state.transform else {
            return Ok(None);
        };
        let mut builder = self.buffer.get_builder();
        let mut num_chars = 0usize;
        // We store the first character separately, because if we only collect
        // one character, we need it as a `char` and not as a `String`.
        let mut first_char: Option<char> = None;

        // Loop until we find a non-letter token.
        while let tok @ (Token::Letter(ch, _) | Token::UprightLetter(ch) | Token::Digit(ch)) =
            self.tokens.peek().token()
        {
            if matches!(tok, Token::Digit(_)) && matches!(tf, MathVariant::Normal) {
                // Don't collect digits in normal math variant.
                break;
            }
            let is_upright = matches!(tok, Token::UprightLetter(_));
            let c = if let MathVariant::Transform(tf) = tf {
                tf.transform(*ch, is_upright)
            } else {
                *ch
            };
            builder.push_char(c);
            if first_char.is_none() {
                first_char = Some(c);
            }
            num_chars += 1;

            if let MathVariant::Transform(
                tf @ (TextTransform::ScriptChancery | TextTransform::ScriptRoundhand),
            ) = tf
            {
                // We need to append Unicode variant selectors for these transforms.
                builder.push_char(if matches!(tf, TextTransform::ScriptChancery) {
                    '\u{FE00}' // VARIATION SELECTOR-1
                } else {
                    '\u{FE01}' // VARIATION SELECTOR-2
                });
                num_chars += 1;
            }
            // Get the next token for the next iteration.
            self.tokens.next()?;
        }
        // If we collected at least one letter, commit it to the arena and return
        // the corresponding AST node.
        let Some(ch) = first_char else {
            return Ok(None);
        };
        let node = self.arena.push(if num_chars == 1 {
            let attr = if matches!(tf, MathVariant::Normal) {
                LetterAttr::ForcedUpright
            } else {
                LetterAttr::Default
            };
            Node::IdentifierChar(ch, attr)
        } else {
            Node::IdentifierStr(builder.finish(self.arena))
        });
        Ok(Some((Class::Default, node)))
    }

    pub(super) fn parse_string_literal(
        &mut self,
    ) -> Result<(&'arena str, Range<usize>), Box<LatexError>> {
        let (tokens, span) = match self.tokens.read_argument(true)? {
            MacroArgument::Group(tokens, span) => (tokens, span),
            MacroArgument::Token(tokspan) => {
                if let (Token::InternalStringLiteral(content), span) = tokspan.into_parts() {
                    return Ok((content, span.into()));
                } else {
                    let span = tokspan.span();
                    (vec![tokspan], span.into())
                }
            }
        };
        let mut builder = self.buffer.get_builder();
        let mut token_iter = tokens.into_iter();
        let mut custom_arg_iter: Option<std::slice::Iter<TokSpan<'source>>> = None;
        loop {
            let tokloc = if let Some(iter) = &mut custom_arg_iter {
                if let Some(tokloc) = iter.next() {
                    *tokloc
                } else {
                    // Finished reading the custom command argument.
                    custom_arg_iter = None;
                    continue;
                }
            } else if let Some(tokloc) = token_iter.next() {
                tokloc
            } else {
                break;
            };
            let (tok, span) = tokloc.into_parts();
            if let Token::CustomCmdArg(arg_num) = tok {
                // Queue the custom command argument tokens.
                let start = self
                    .state
                    .cmd_arg_offsets
                    .get(arg_num.wrapping_sub(1) as usize)
                    .copied()
                    .unwrap_or(0);
                let end = self
                    .state
                    .cmd_arg_offsets
                    .get(arg_num as usize)
                    .copied()
                    .unwrap_or(self.state.cmd_args.len());
                if let Some(arg) = self.state.cmd_args.get(start..end) {
                    custom_arg_iter = Some(arg.iter());
                }
                continue;
            }
            let Some(ch) = recover_limited_ascii(tok) else {
                return Err(Box::new(LatexError(
                    span.into(),
                    LatexErrKind::ExpectedText,
                )));
            };
            builder.push_char(ch);
        }
        Ok((builder.finish(self.arena), span))
    }
}

impl ParserState<'_, '_> {
    fn relation_spacing(
        &self,
        prev_class: Class,
        next_class: Class,
    ) -> (Option<MathSpacing>, Option<MathSpacing>) {
        (
            if matches!(
                prev_class,
                Class::Relation | Class::Open | Class::Punctuation
            ) || self.script_style
            {
                Some(MathSpacing::Zero)
            } else {
                None
            },
            if matches!(
                next_class,
                Class::Relation | Class::Punctuation | Class::Close | Class::End
            ) || self.script_style
            {
                Some(MathSpacing::Zero)
            } else {
                None
            },
        )
    }

    fn bin_op_spacing(
        &self,
        in_sequence: bool,
        prev_class: Class,
        next_class: Class,
        force: bool,
    ) -> Option<MathSpacing> {
        if !in_sequence {
            // Don't add spacing if we are in an argument.
            None
        } else if matches!(
            prev_class,
            Class::Relation | Class::Punctuation | Class::BinaryOp | Class::Operator | Class::Open
        ) || matches!(
            next_class,
            Class::Relation | Class::Punctuation | Class::Close | Class::End
        ) || self.script_style
        {
            Some(MathSpacing::Zero)
        } else if force {
            Some(MathSpacing::FourMu) // force binary op spacing
        } else {
            None
        }
    }
}

// Turn a vector of nodes into a single node.
//
// This is done either by returning the single node if there is only one,
// or by creating a row node if there are multiple nodes.
pub(crate) fn node_vec_to_node<'arena>(
    arena: &'arena Arena,
    nodes: &[&'arena Node<'arena>],
    reset_spacing: bool,
) -> &'arena Node<'arena> {
    if let [single] = nodes {
        if reset_spacing {
            if let Node::Operator {
                op,
                attrs: attr,
                left: _,
                right: _,
            } = single
            {
                arena.push(Node::Operator {
                    op: *op,
                    attrs: *attr,
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
        let nodes = arena.push_slice(nodes);
        arena.push(Node::Row { nodes, attr: None })
    }
}

/// Get the attributes for a middle operator (which needs to stretch symmetrically).
fn middle_stretch_attrs(op: StretchableOp) -> OpAttrs {
    match op.stretchy {
        Stretchy::PrePostfix | Stretchy::Never => OpAttrs::STRETCHY_TRUE,
        Stretchy::AlwaysAsymmetric => OpAttrs::SYMMETRIC_TRUE,
        Stretchy::Always => OpAttrs::empty(),
    }
}

fn extract_delimiter(tok: TokSpan<'_>, location: DelimiterModifier) -> ParseResult<StretchableOp> {
    let (tok, span) = tok.into_parts();
    const SQ_L_BRACKET: StretchableOp = symbol::LEFT_SQUARE_BRACKET.as_stretchable_op().unwrap();
    const SQ_R_BRACKET: StretchableOp = symbol::RIGHT_SQUARE_BRACKET.as_stretchable_op().unwrap();
    let delim = match tok {
        Token::Open(paren) | Token::Close(paren) => paren.as_stretchable_op(),
        Token::Ord(ord) => ord.as_stretchable_op(),
        Token::Relation(rel) => rel.as_stretchable_op(),
        Token::SquareBracketOpen => Some(SQ_L_BRACKET),
        Token::SquareBracketClose => Some(SQ_R_BRACKET),
        _ => None,
    };
    let Some(delim) = delim else {
        return Err(Box::new(LatexError(
            span.into(),
            LatexErrKind::ExpectedDelimiter(location),
        )));
    };
    Ok(delim)
}

struct Bounds<'arena>(Option<&'arena Node<'arena>>, Option<&'arena Node<'arena>>);

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
            ("sout", r"\sout{abc}"),
            ("sum_relation", r"{\sum = 4}"),
            ("int_relation", r"{\int = 4}"),
            ("int_bounds_relation", r"{\int_0^\infty = 4}"),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let mut equation_counter = 0u16;
            let mut label_map = FxHashMap::default();
            let l = Lexer::new(problem, false, None);
            let mut p = Parser::new(l, &arena, &mut equation_counter, &mut label_map).unwrap();
            let ast = p.parse().expect("Parsing failed");
            assert_ron_snapshot!(name, &ast, problem);
        }
    }

    #[test]
    fn ast_from_token_stream_test() {
        use crate::token::Token::*;
        let problems: [(&'static str, &'static [Token]); 3] = [
            (
                "text_internal_string_literal",
                &[Text(None), InternalStringLiteral("hi")],
            ),
            (
                "text_internal_string_literal_and_other",
                &[
                    Text(None),
                    GroupBegin,
                    Letter('a', Mode::MathOrText),
                    InternalStringLiteral("hi"),
                    GroupEnd,
                ],
            ),
            (
                "space_internal_string_literal",
                &[CustomSpace, InternalStringLiteral("3em")],
            ),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let mut equation_counter = 0u16;
            let mut label_map = FxHashMap::default();
            let l = Lexer::new("", false, None);
            let mut p = Parser::new(l, &arena, &mut equation_counter, &mut label_map).unwrap();
            p.tokens.queue_in_front(problem);
            let ast = p.parse().expect("Parsing failed");
            let problem = format!("{:?}", problem);
            assert_ron_snapshot!(name, &ast, &problem);
        }
    }
}
