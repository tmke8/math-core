use std::{fmt::Write as _, mem, ops::Range};

use mathml_renderer::{
    arena::{Arena, Buffer},
    ast::{AHref, MultiscriptPair, Node, RowAttrs},
    attribute::{FracAttr, LetterAttr, MathSpacing, OpAttrs, Style},
    length::{Length, LengthUnit},
    super_char::SuperChar,
    symbol::{self, OpCategory, OrdCategory, OrdLike, RelCategory},
    table::RowLabelInfo,
};
use rustc_hash::FxHashMap;

use crate::{
    atof::limited_float_parse,
    character_class::{
        Class, DelimiterSpacing, MathVariant, ParenType, StretchableOp, Stretchy, fenced,
    },
    color_defs::get_color,
    environments::{
        CLOSE_BRACE, CLOSE_BRACKET, CLOSE_PAREN, Env, NumberedEnvState, OPEN_BRACE, OPEN_BRACKET,
        OPEN_PAREN,
    },
    error::{DelimiterModifier, LatexErrKind, LatexError, LimitedUsabilityToken, Place},
    lexer::{Lexer, recover_limited_ascii},
    specifications::{LatexUnit, parse_column_specification, parse_length_specification},
    split_on_ascii::split_on_ascii,
    text_parser::TextSnippet,
    token::{
        EndToken, InfixDelim, LimitsKind, MathClassKind, Mode, PhantomKind, PrimeDirection,
        PrimeKind, Span, TokSpan, Token, UnitKind,
    },
    token_queue::{MacroArgument, OneOrNone, TokenQueue},
};

const FULL_STOP_TOKEN: Token<'static> = Token::Letter(SuperChar::from_char('.'), Mode::MathOrText);

pub(crate) struct Parser<'config, 'source, 'arena> {
    pub(super) tokens: TokenQueue<'config, 'source>,
    pub(super) buffer: Buffer,
    pub(super) arena: &'arena Arena,
    equation_counter: &'arena mut usize,
    label_map: &'arena mut FxHashMap<Box<str>, Box<str>>,
    unicode_substitution: crate::UnicodeSubstitution,
    state: ParserState<'source, 'arena>,
}

#[derive(Debug)]
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
    /// The current style (display/text/script/scriptscript) for the surrounding group.
    style: Style,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlFlow {
    SkipToken,
    ProcessToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoundStarterKind {
    /// `_`
    Underscore,
    /// `^`
    Circumflex,
    /// `'` and Unicode friends
    Prime(PrimeKind),
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
        equation_counter: &'arena mut usize,
        label_map: &'arena mut FxHashMap<Box<str>, Box<str>>,
        style: Style,
        unicode_substitution: crate::UnicodeSubstitution,
    ) -> ParseResult<Self> {
        let input_length = lexer.input_length();
        Ok(Parser {
            tokens: TokenQueue::new(lexer)?,
            buffer: Buffer::new(input_length),
            arena,
            equation_counter,
            unicode_substitution,
            label_map,
            state: ParserState {
                cmd_args: Vec::new(),
                cmd_arg_offsets: [0; 9],
                transform: None,
                right_boundary_hack: false,
                allow_columns: false,
                meaningful_newlines: false,
                numbered: None,
                style,
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
        let mut infix_frac: Option<(Vec<&'arena Node<'arena>>, bool, Option<InfixDelim>)> = None;

        let mut prev_class = prev_class;
        let old_tf = self.state.transform;
        let old_style = self.state.style;

        // Because we don't want to consume the end token, we just peek here.
        while !sequence_end.matches(self.tokens.peek().token()) {
            // Check whether we need to collect letters.
            let (class, target) = if let Some(collected) = self.merge_and_transform_letters()? {
                collected
            } else {
                // Get the current token.
                let cur_tokloc = self.next_token();
                if let Ok(tokloc) = &cur_tokloc {
                    match self.handle_tokens_without_output(
                        tokloc,
                        sequence_end,
                        &mut nodes,
                        &mut infix_frac,
                    )? {
                        ControlFlow::SkipToken => continue,
                        ControlFlow::ProcessToken => {}
                    }
                }
                // Parse the token.
                self.parse_token(cur_tokloc, ParseAs::Sequence, prev_class)?
            };
            prev_class = class;

            // Check if there are any superscripts or subscripts following the parsed node.
            let bounds = self.get_bounds(None)?.ensure_no_explicit_limits()?;

            match target {
                Node::Multiscripts {
                    base,
                    pre,
                    post: [],
                } if !bounds.is_trivial() => {
                    let post = self.arena.alloc_multiscript_pairs(&[bounds.into()]);
                    let node = self.commit(Node::Multiscripts { base, pre, post });
                    nodes.push(node);
                }
                _ => {
                    let node = match bounds.try_wrap_node_subsup(target) {
                        Some(node) => self.arena.push(node),
                        None => target,
                    };

                    nodes.push(node)
                }
            }
            // If there are superscripts or subscripts, we need to wrap the node we just got into
            // one of the node types for superscripts and subscripts.
        }
        if let Some((numerator, with_line, delim)) = infix_frac {
            let denominator = mem::replace(&mut nodes, Vec::with_capacity(1));
            let (lt_value, lt_unit) = if with_line {
                Length::none().into_parts()
            } else {
                Length::zero().into_parts()
            };
            let frac = self.commit(Node::Frac {
                num: node_vec_to_node(self.arena, &numerator, false),
                denom: node_vec_to_node(self.arena, &denominator, false),
                lt_value,
                lt_unit,
                attr: None,
            });
            let node = if let Some(delim) = delim {
                let (open, close) = match delim {
                    InfixDelim::Paren => (OPEN_PAREN, CLOSE_PAREN),
                    InfixDelim::Brace => (OPEN_BRACE, CLOSE_BRACE),
                    InfixDelim::Brack => (OPEN_BRACKET, CLOSE_BRACKET),
                };
                self.commit(fenced(
                    self.arena,
                    vec![frac],
                    Some(open),
                    Some(close),
                    None,
                ))
            } else {
                frac
            };
            nodes.push(node);
        }
        if !keep_end_token {
            // Discard the end token.
            self.next_token()?;
        }
        self.state.transform = old_tf;
        self.state.style = old_style;
        Ok(nodes)
    }

    #[inline]
    fn handle_tokens_without_output(
        &mut self,
        tokspan: &TokSpan<'source>,
        sequence_end: SequenceEnd,
        collected_nodes: &mut Vec<&'arena Node<'arena>>,
        infix_frac: &mut Option<(Vec<&'arena Node<'arena>>, bool, Option<InfixDelim>)>,
    ) -> ParseResult<ControlFlow> {
        let span = tokspan.span().into();
        let result: Result<(), LatexError> = match *tokspan.token() {
            Token::Eoi => {
                if let SequenceEnd::EndToken(end_token) = sequence_end {
                    // The input has ended without the closing token.
                    Err(LatexError(span, LatexErrKind::UnclosedGroup(end_token)))
                } else {
                    return Ok(ControlFlow::ProcessToken);
                }
            }
            Token::InfixGenFrac { with_line, delim } => {
                if infix_frac.is_none() {
                    *infix_frac = Some((mem::take(collected_nodes), with_line, delim));
                    // The numerator was already parsed in the surrounding style (we only
                    // learn it's a fraction here), but we can at least shrink the style
                    // for the denominator. `parse_sequence` restores the style on exit.
                    self.state.style = self.state.style.shrink();
                    Ok(())
                } else {
                    Err(LatexError(span, LatexErrKind::MoreThanOneInfixCmd))
                }
            }
            Token::TransformSwitch(tf) => {
                self.state.transform = Some(tf);
                Ok(())
            }
            Token::NoNumber => {
                if let Some(numbered_state) = &mut self.state.numbered {
                    numbered_state.suppress_next_number = true;
                }
                Ok(())
            }
            Token::Tag => {
                let (tag_name, _) = self.parse_string_literal()?;
                if let Some(numbered_state) = &mut self.state.numbered {
                    numbered_state.custom_next_tag = Some(tag_name.into());
                    Ok(())
                } else {
                    Err(LatexError(
                        span,
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
                        Err(LatexError(span, LatexErrKind::MoreThanOneLabel))
                    } else {
                        numbered_state.label = Some(label_name);
                        Ok(())
                    }
                } else {
                    Err(LatexError(
                        span,
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
                        tf.transform(number.into(), false),
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
                        let ch = if let Token::Digit(number) = *self.tokens.peek().token() {
                            number
                        } else {
                            let ch = if matches!(self.tokens.peek().token(), &FULL_STOP_TOKEN) {
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

                Ok(Node::IdentifierChar(
                    ch,
                    if is_upright && !with_tf {
                        LetterAttr::ForcedUpright
                    } else {
                        LetterAttr::Default
                    },
                ))
            }
            Token::Relation(relation) => {
                class = Class::Relation;
                let attrs = relation_attrs(relation.category());
                let (left, right) = self.state.relation_spacing(prev_class, next_class, false);
                Ok(Node::Operator {
                    op: relation.as_op(),
                    attrs,
                    left,
                    right,
                    size: None,
                })
            }
            Token::Punctuation(punc) => {
                class = Class::Punctuation;
                let (left, right) = self.state.punctuation_spacing(next_class, false);
                Ok(Node::Operator {
                    op: punc.as_op(),
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    size: None,
                })
            }
            Token::ForcePunctuation(op) => {
                class = Class::Punctuation;
                let (left, right) = self.state.punctuation_spacing(next_class, true);
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    size: None,
                })
            }
            Token::ForceLargeOp(op) => {
                class = Class::Operator;

                let bounds_with_limits = self.get_bounds(None)?;
                let bounds = bounds_with_limits.bounds;
                let (left, right) = self.mathop_spacing(parse_as, prev_class, true)?;
                let (use_underover, attrs) = match bounds_with_limits.limits() {
                    _ if bounds.is_trivial() => {
                        (true, OpAttrs::SYMMETRIC_TRUE | OpAttrs::LARGEOP_TRUE)
                    }
                    None | Some(LimitsKind::Display) => (
                        true,
                        OpAttrs::SYMMETRIC_TRUE
                            | OpAttrs::LARGEOP_TRUE
                            | OpAttrs::FORCE_MOVABLE_LIMITS,
                    ),
                    Some(LimitsKind::Always) => {
                        (true, OpAttrs::SYMMETRIC_TRUE | OpAttrs::LARGEOP_TRUE)
                    }
                    Some(LimitsKind::Never) => {
                        (false, OpAttrs::SYMMETRIC_TRUE | OpAttrs::LARGEOP_TRUE)
                    }
                };

                let target = self.commit(Node::Operator {
                    op,
                    attrs,
                    left,
                    right,
                    size: None,
                });
                if use_underover {
                    if let Some(node) = bounds.try_wrap_node_underover(target) {
                        Ok(node)
                    } else {
                        return Ok((class, target));
                    }
                } else if let Some(node) = bounds.try_wrap_node_subsup(target) {
                    Ok(node)
                } else {
                    return Ok((class, target));
                }
            }
            Token::Ord(ord) => {
                let attrs = if matches!(
                    ord.category(),
                    OrdCategory::F
                        | OrdCategory::G
                        | OrdCategory::FG
                        | OrdCategory::FGandForceDefault
                ) {
                    // Category F+G operators will stretch in pre- and postfix positions,
                    // so we explicitly set the stretchy attribute to false to prevent that.
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
                    attrs,
                    left,
                    right,
                    size: None,
                })
            }
            Token::BinaryOp(binary_op) => {
                let spacing = self.state.bin_op_spacing(
                    parse_as.in_sequence(),
                    prev_class,
                    next_class,
                    false,
                );
                class = if matches!(spacing, Some(MathSpacing::Zero)) {
                    // If the spacing is zero, this operator behaves like an ordinary symbol in
                    // terms of spacing.
                    Class::Default
                } else {
                    Class::BinaryOp
                };
                Ok(Node::Operator {
                    op: binary_op.as_op(),
                    attrs: OpAttrs::empty(),
                    left: spacing,
                    right: spacing,
                    size: None,
                })
            }
            Token::ForceBinaryOp(op) => {
                let spacing =
                    self.state
                        .bin_op_spacing(parse_as.in_sequence(), prev_class, next_class, true);
                class = if matches!(spacing, Some(MathSpacing::Zero)) {
                    // If the spacing is zero, this operator behaves like an ordinary symbol in
                    // terms of spacing.
                    Class::Default
                } else {
                    Class::BinaryOp
                };
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left: spacing,
                    right: spacing,
                    size: None,
                })
            }
            Token::MathClass(kind) => {
                let tok_span = self.next_token()?;
                let (_, node) = self.parse_token(Ok(tok_span), parse_as, prev_class)?;
                // Recompute the next class:
                let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;
                let (left, right) = match kind {
                    MathClassKind::Ord => {
                        class = Class::Default;
                        (Some(MathSpacing::Zero), Some(MathSpacing::Zero))
                    }
                    MathClassKind::Op => {
                        class = Class::Operator;
                        self.mathop_spacing(parse_as, prev_class, true)?
                    }
                    MathClassKind::Bin => {
                        class = Class::BinaryOp;
                        let spacing = self.state.bin_op_spacing(
                            parse_as.in_sequence(),
                            prev_class,
                            next_class,
                            true,
                        );
                        (spacing, spacing)
                    }
                    MathClassKind::Rel => {
                        class = Class::Relation;
                        self.state.relation_spacing(prev_class, next_class, true)
                    }
                    MathClassKind::Open => {
                        class = Class::Open;
                        (Some(MathSpacing::Zero), Some(MathSpacing::Zero))
                    }
                    MathClassKind::Close => {
                        class = Class::Close;
                        (Some(MathSpacing::Zero), Some(MathSpacing::Zero))
                    }
                    MathClassKind::Punct => {
                        class = Class::Punctuation;
                        self.state.punctuation_spacing(next_class, true)
                    }
                    MathClassKind::Inner => {
                        class = Class::Inner;
                        self.state.mathinner_spacing(prev_class, next_class, true)
                    }
                };
                match *node {
                    Node::Operator {
                        op,
                        attrs,
                        size,
                        left: _,
                        right: _,
                    } => Ok(Node::Operator {
                        op,
                        attrs: attrs | OpAttrs::STRETCHY_FALSE,
                        left,
                        right,
                        size,
                    }),
                    Node::Row {
                        nodes: [],
                        attrs:
                            RowAttrs {
                                color: None,
                                style: None,
                                math_shift_compact: false,
                            },
                    } => Ok(Node::Operator {
                        // An empty `<mo></mo>` produces no spacing in Firefox
                        op: const { symbol::INVISIBLE_SEPARATOR.as_op() },
                        attrs: OpAttrs::empty(),
                        left,
                        right,
                        size: None,
                    }),
                    _ => Ok(Node::Padded {
                        node,
                        width_0: false,
                        height_0: false,
                        left,
                        right,
                    }),
                }
            }
            Token::Inner(op) => {
                class = Class::Inner;
                let (left, right) = self.state.mathinner_spacing(prev_class, next_class, false);
                Ok(Node::Operator {
                    op: op.as_op(),
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    size: None,
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
            Token::CustomSpace(kind) => {
                let (length, span) = self.parse_string_literal()?;
                let trimmed = length.trim_ascii();
                match parse_length_specification(trimmed) {
                    Some((space, unit, is_math_unit)) => {
                        let math_unit_expected = matches!(kind, UnitKind::MathUnits);
                        if is_math_unit == math_unit_expected {
                            Ok(Node::Space(space))
                        } else {
                            Err(LatexError(
                                span,
                                LatexErrKind::IllegalUnit {
                                    unit: unit.into(),
                                    math_unit_expected,
                                },
                            ))
                        }
                    }
                    None => Err(LatexError(
                        span,
                        LatexErrKind::ExpectedLength(length.into()),
                    )),
                }
            }
            Token::KernOrSkip(kind) => {
                // Spaces pass through the symbol class.
                class = prev_class;
                Ok(self.parse_kern_or_skip(kind, span.end())?)
            }
            Token::NonBreakingSpace => Ok(Node::Text(None, None, "\u{A0}")),
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
                let inner_style = match attr {
                    None => self.state.style.shrink(),
                    Some(FracAttr::CFracStyle | FracAttr::DisplayStyleTrue) => Style::Text,
                    Some(FracAttr::DisplayStyleFalse) => Style::Script,
                };
                let old_style = mem::replace(&mut self.state.style, inner_style);
                let num = self.parse_next(ParseAs::Arg)?;
                let denom = self.parse_next(ParseAs::Arg)?;
                self.state.style = old_style;
                if matches!(cur_token, Token::Binom(_)) {
                    let (lt_value, lt_unit) = Length::zero().into_parts();
                    Ok(fenced(
                        self.arena,
                        vec![self.commit(Node::Frac {
                            num,
                            denom,
                            lt_value,
                            lt_unit,
                            attr,
                        })],
                        Some(OPEN_PAREN),
                        Some(CLOSE_PAREN),
                        None,
                    ))
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
                let lt = match length.trim_ascii() {
                    "" => Length::none(),
                    decimal => {
                        parse_length_specification(decimal)
                            .ok_or_else(|| {
                                Box::new(LatexError(
                                    span,
                                    LatexErrKind::ExpectedLength(decimal.into()),
                                ))
                            })?
                            .0
                    }
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
                Ok(fenced(
                    self.arena,
                    vec![self.commit(Node::Frac {
                        num,
                        denom,
                        lt_value,
                        lt_unit,
                        attr,
                    })],
                    open,
                    close,
                    style,
                ))
            }
            Token::Accent(op, is_over, attr) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                if is_over {
                    Ok(Node::OverAccent(op, attr, target))
                } else {
                    Ok(Node::UnderAccent(op, attr, target))
                }
            }
            Token::Overset | Token::Underset => {
                let old_style = mem::replace(&mut self.state.style, Style::Script);
                let symbol = self.parse_next(ParseAs::Arg)?;
                self.state.style = old_style;
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
                    size: None,
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

                let bounds_limits = if has_bounds {
                    self.get_bounds(None)?
                } else {
                    BoundsWithLimits::default()
                };

                let bounds = bounds_limits.bounds;

                let (left, right) = self.mathop_spacing(parse_as, prev_class, false)?;

                let (use_underover, attrs) = match (bounds_limits.limits(), has_movable_limits) {
                    (_, _) if bounds.is_trivial() => (false, OpAttrs::empty()),
                    (None, has_movable_limits)
                    | (Some(LimitsKind::Display), has_movable_limits @ true) => {
                        (has_movable_limits, OpAttrs::empty())
                    }
                    (Some(LimitsKind::Display), false) => (true, OpAttrs::FORCE_MOVABLE_LIMITS),
                    (Some(LimitsKind::Always), false) => (true, OpAttrs::empty()),
                    (Some(LimitsKind::Always), true) => (true, OpAttrs::NO_MOVABLE_LIMITS),
                    (Some(LimitsKind::Never), _) => (false, OpAttrs::empty()),
                };

                let target = self.commit(Node::Operator {
                    op: op.as_op(),
                    attrs,
                    left,
                    right,
                    size: None,
                });

                if use_underover {
                    match bounds.try_wrap_node_underover(target) {
                        Some(node) => Ok(node),
                        None => return Ok((class, target)),
                    }
                } else {
                    match bounds.try_wrap_node_subsup(target) {
                        Some(node) => Ok(node),
                        None => return Ok((class, target)),
                    }
                }
            }
            ref tok @ (Token::PseudoOperator(name) | Token::PseudoOperatorLimits(name)) => {
                class = Class::Operator;
                let bounds_limits = self.get_bounds(None)?;
                let bounds = bounds_limits.bounds;

                let limits_by_default = matches!(tok, Token::PseudoOperatorLimits(_));
                let (use_underover, attrs) = match (bounds_limits.limits(), limits_by_default) {
                    _ if bounds.is_trivial() => (false, OpAttrs::empty()),
                    (None, true) | (Some(LimitsKind::Display), _) => {
                        (true, OpAttrs::FORCE_MOVABLE_LIMITS)
                    }
                    (None, false) | (Some(LimitsKind::Never), _) => (false, OpAttrs::empty()),
                    (Some(LimitsKind::Always), _) => (true, OpAttrs::empty()),
                };

                // Compute spacing after getting the bounds, so that we don't
                // consider tokens that are part of the bounds for spacing calculations.
                let (left, right) = self.mathop_spacing(parse_as, prev_class, true)?;
                let target = self.commit(Node::PseudoOp {
                    attrs,
                    left,
                    right,
                    name,
                });

                if use_underover {
                    match bounds.try_wrap_node_underover(target) {
                        Some(node) => Ok(node),
                        None => return Ok((class, target)),
                    }
                } else {
                    match bounds.try_wrap_node_subsup(target) {
                        Some(node) => Ok(node),
                        None => return Ok((class, target)),
                    }
                }
            }
            Token::Overlay(overlay) => {
                // `\not` has to be followed by something:

                let tok_span = self.next_token()?;
                let new_span = tok_span.span();
                let (cls, node) = self.parse_token(Ok(tok_span), parse_as, prev_class)?;
                class = cls;

                /// Helper for `Node::PseudoOp` and `Node::IdentifierStr` below.
                /// Finds the byte offset of the location immediately after the
                /// first character in the string, and if the character is followed
                /// by a variation selector, then after that as well.
                ///
                /// (We don't want to insert an overlay between a base char and its variation selector)
                fn after_first_char_and_vs(s: &str) -> usize {
                    let mut indices = s.char_indices();
                    if indices.next().is_none() {
                        // empty string
                        return 0;
                    }

                    let Some((after_first_char_idx, snd_char)) = indices.next() else {
                        // string is 1 char long
                        return s.len();
                    };

                    if matches!(snd_char, '\u{FE00}'..'\u{FE0F}') {
                        // There's a variation selector (3 bytes in utf8)
                        after_first_char_idx + 3
                    } else {
                        // No variation selector
                        after_first_char_idx
                    }
                }

                match *node {
                    Node::Operator {
                        op,
                        attrs,
                        size,
                        left,
                        right,
                    } => Ok(Node::Operator {
                        op: op.with_overlay(overlay),
                        attrs,
                        size,
                        left,
                        right,
                    }),

                    Node::IdentifierChar(ident, letter_attr) => Ok(Node::IdentifierChar(
                        ident.with_overlay(overlay),
                        letter_attr,
                    )),

                    Node::PseudoOp {
                        name,
                        attrs,
                        left,
                        right,
                    } => {
                        let mut builder = self.buffer.get_builder();
                        let insert_idx = after_first_char_and_vs(name);
                        builder.push_str(&name[..insert_idx]);
                        builder.push_char(overlay.into());
                        builder.push_str(&name[insert_idx..]);
                        let name = builder.finish(self.arena);
                        Ok(Node::PseudoOp {
                            name,
                            attrs,
                            left,
                            right,
                        })
                    }

                    Node::IdentifierStr(str) => {
                        let mut builder = self.buffer.get_builder();
                        let insert_idx = after_first_char_and_vs(str);
                        builder.push_str(&str[..insert_idx]);
                        builder.push_char(overlay.into());
                        builder.push_str(&str[insert_idx..]);
                        let str = builder.finish(self.arena);
                        Ok(Node::IdentifierStr(str))
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
            Token::TransformSwitch(_)
            | Token::NoNumber
            | Token::Tag
            | Token::Label
            | Token::InfixGenFrac { .. } => Err(LatexError(
                span.into(),
                LatexErrKind::CannotBeUsedAsArgument,
            )),
            Token::ForceRelation(op) => {
                class = Class::Relation;
                let (left, right) = if parse_as.in_sequence() {
                    self.state.relation_spacing(prev_class, next_class, true)
                } else {
                    // Don't add spacing if we are in an argument.
                    (None, None)
                };
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    size: None,
                })
            }
            Token::ForceOpen(op, _) => {
                class = Class::Open;
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::FORM_PREFIX,
                    left: Some(MathSpacing::Zero),
                    right: Some(MathSpacing::Zero),
                    size: None,
                })
            }
            Token::ForceClose(op, _) => {
                class = Class::Close;
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::FORM_POSTFIX,
                    left: Some(MathSpacing::Zero),
                    right: Some(MathSpacing::Zero),
                    size: None,
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
                let mut attrs = if matches!(
                    paren.category(),
                    OrdCategory::FG | OrdCategory::FGandForceDefault | OrdCategory::DE
                ) {
                    // For these categories of symbol, both prefix and postfix forms exist, so we
                    // explicitly set the form attributes based on the token type (Open vs Close).
                    // For `FGandForceDefault`, the form attribute also affects spacing.
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
                    OrdCategory::F
                        | OrdCategory::G
                        | OrdCategory::FG
                        | OrdCategory::FGandForceDefault
                ) {
                    // Symbols from these categories are automatically stretchy,
                    // so we have to explicitly disable that here.
                    attrs |= OpAttrs::STRETCHY_FALSE;
                }
                Ok(Node::Operator {
                    op: paren.as_op(),
                    attrs,
                    left: None,
                    right: None,
                    size: None,
                })
            }
            Token::SquareBracketOpen => {
                class = Class::Open;
                Ok(Node::Operator {
                    op: symbol::LEFT_SQUARE_BRACKET.as_op(),
                    attrs: OpAttrs::STRETCHY_FALSE,
                    left: None,
                    right: None,
                    size: None,
                })
            }
            Token::SquareBracketClose => Ok(Node::Operator {
                op: symbol::RIGHT_SQUARE_BRACKET.as_op(),
                attrs: OpAttrs::STRETCHY_FALSE,
                left: None,
                right: None,
                size: None,
            }),
            Token::Left => {
                let tok_loc = self.next_token()?;
                let open_paren = if matches!(tok_loc.token(), &FULL_STOP_TOKEN) {
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
                let close_paren = if matches!(tok_loc.token(), &FULL_STOP_TOKEN) {
                    None
                } else {
                    Some(extract_delimiter(tok_loc, DelimiterModifier::Right)?)
                };
                Ok(fenced(self.arena, content, open_paren, close_paren, None))
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
                    size: None,
                })
            }
            Token::Big(size, paren_type) => {
                let tok_loc = self.next_token()?;
                let paren = extract_delimiter(tok_loc, DelimiterModifier::Big)?;
                // `\big` commands without the "l" or "r" really produce `Class::Default`.
                class = match paren_type {
                    Some(ParenType::Left) => Class::Open,
                    Some(ParenType::Right) => Class::Close,
                    Some(ParenType::Middle) => Class::Relation,
                    None => Class::Default,
                };
                // Convert stretchy property to OpAttrs.
                let mut attrs = match paren.stretchy {
                    Stretchy::PrePostfix | Stretchy::Never => {
                        OpAttrs::STRETCHY_TRUE | OpAttrs::SYMMETRIC_TRUE
                    }
                    Stretchy::AlwaysAsymmetric => OpAttrs::SYMMETRIC_TRUE,
                    Stretchy::Always => OpAttrs::empty(),
                };
                // Determine form and spacing attributes based on paren_type
                // and delimiter spacing.
                let (left, right) = if matches!(paren_type, Some(ParenType::Middle)) {
                    // We need to achieve relation spacing here.
                    let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;
                    if matches!(paren.spacing, DelimiterSpacing::InfixRelation) {
                        attrs |= OpAttrs::FORM_INFIX;
                    }
                    self.state.relation_spacing(
                        prev_class,
                        next_class,
                        !matches!(
                            paren.spacing,
                            DelimiterSpacing::InfixRelation | DelimiterSpacing::Relation
                        ),
                    )
                } else {
                    // We need to achieve open/close spacing here (i.e., zero spacing)
                    // If the delimiter has relation spacing only in infix positions, then we can
                    // get spacing to zero by setting the form attributes.
                    if let Some(paren_type) = paren_type
                        && matches!(paren.spacing, DelimiterSpacing::InfixRelation)
                    {
                        // We already handled the spacing for middle delimiters above, so we only
                        // need to set the form attributes for left and right delimiters here.
                        if matches!(paren_type, ParenType::Left) {
                            attrs |= OpAttrs::FORM_PREFIX;
                        } else {
                            attrs |= OpAttrs::FORM_POSTFIX;
                        }
                        (None, None)
                    } else if matches!(
                        paren.spacing,
                        DelimiterSpacing::InfixRelation
                            | DelimiterSpacing::Relation
                            | DelimiterSpacing::Other
                    ) {
                        (Some(MathSpacing::Zero), Some(MathSpacing::Zero))
                    } else {
                        (None, None)
                    }
                };
                Ok(Node::Operator {
                    op: paren.as_op(),
                    attrs,
                    size: Some(size),
                    left,
                    right,
                })
            }
            Token::Begin(env) => 'begin_env: {
                let array_spec = if matches!(env, Env::Array | Env::DArray | Env::Subarray) {
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
                let old_style = mem::replace(&mut self.state.style, env.style());
                let old_numbered =
                    mem::replace(&mut self.state.numbered, env.get_numbered_env_state());

                let content = self.arena.push_slice(&self.parse_sequence(
                    SequenceEnd::EndToken(EndToken::End),
                    Class::Open,
                    true, // keep_end_token
                )?);

                self.state.allow_columns = old_allow_columns;
                self.state.meaningful_newlines = old_meaningful_newlines;
                self.state.style = old_style;
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

                let (last_row_info, num_rows) = if let Some(mut n) = numbered_state {
                    match n.next_equation_tag(self.equation_counter, true) {
                        Ok(tag) => {
                            let link_target = n.label.take();
                            let info = if let Some(tag) = tag {
                                let tag_arena = self.arena.alloc_str(&tag);
                                if let Some(label) = link_target {
                                    self.label_map.insert(label.into(), tag);
                                }
                                Some(self.arena.alloc_row_label_info(RowLabelInfo {
                                    tag: tag_arena,
                                    link_target,
                                }))
                            } else {
                                None
                            };
                            (info, n.num_rows)
                        }
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

                Ok(env.construct_node(content, array_spec, self.arena, last_row_info, num_rows))
            }
            Token::OperatorName { with_limits } => {
                let snippets = self.extract_text(None, false)?;
                let mut builder = self.buffer.get_builder();
                for TextSnippet(_style, _size, text) in snippets {
                    builder.push_str(text);
                }
                let letters = builder.finish(self.arena);
                let (left, right) = self.mathop_spacing(parse_as, prev_class, true)?;
                let op = self.commit(Node::PseudoOp {
                    attrs: OpAttrs::empty(),
                    left,
                    right,
                    name: letters,
                });
                if with_limits {
                    let bounds = self.get_bounds(None)?.ensure_no_explicit_limits()?;
                    let node = match bounds.try_wrap_node_underover(op) {
                        Some(node) => node,
                        None => return Ok((Class::Operator, op)),
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
                    .map(|TextSnippet(style, size, text)| {
                        self.commit(Node::Text(style, size, text))
                    })
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
                class = Class::Open;
                if !self.state.meaningful_newlines {
                    // TODO: Return something other than a row here, so that we can avoid creating
                    //       empty rows in places where they are not needed.
                    Ok(Node::EMPTY_ROW)
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
                    match numbered_state.next_equation_tag(self.equation_counter, false) {
                        Ok(tag) => {
                            let link_target = numbered_state.label.take();
                            let label_info = if let Some(tag) = tag {
                                let tag_arena = self.arena.alloc_str(&tag);
                                if let Some(label) = link_target {
                                    self.label_map.insert(label.into(), tag);
                                }
                                Some(self.arena.alloc_row_label_info(RowLabelInfo {
                                    tag: tag_arena,
                                    link_target,
                                }))
                            } else {
                                // If we don't have a tag, we're not setting a link target either
                                None
                            };
                            Ok(Node::RowSeparator(label_info))
                        }
                        Err(()) => Err(LatexError(span.into(), LatexErrKind::HardLimitExceeded)),
                    }
                } else {
                    Ok(Node::RowSeparator(None))
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
                let href = {
                    let mut builder = self.buffer.get_builder();
                    write!(builder, "#{label_name}").unwrap();
                    builder.finish(self.arena)
                };
                let text = {
                    let mut builder = self.buffer.get_builder();
                    write!(builder, "({tag})").unwrap();
                    builder.finish(self.arena)
                };

                Ok(Node::AHref(self.arena.alloc_ahref(AHref { href, text })))
            }
            Token::Cramped => {
                // Optional style argument in square brackets (e.g. `\cramped[\scriptstyle]{b}`),
                // handled in the same style as `\sqrt`'s optional degree argument.
                let next = self.next_token();
                let (style, inner) = if let Ok(tokloc) = next
                    && matches!(tokloc.token(), Token::SquareBracketOpen)
                {
                    let style_tok = self.next_token()?;
                    let style = match *style_tok.token() {
                        Token::Style(style) => {
                            let closing_tok = self.next_token()?;
                            if *closing_tok.token() != Token::SquareBracketClose {
                                return Err(Box::new(LatexError(
                                    closing_tok.span().into(),
                                    LatexErrKind::ExpectedAtMostOneToken,
                                )));
                            }
                            Some(style)
                        }
                        Token::SquareBracketClose => None,
                        _ => {
                            return Err(Box::new(LatexError(
                                style_tok.span().into(),
                                LatexErrKind::ExpectedStyle,
                            )));
                        }
                    };

                    let inner = self.parse_next(ParseAs::Arg)?;
                    (style, inner)
                } else {
                    let inner = self.parse_token(next, ParseAs::Arg, prev_class)?.1;
                    (None, inner)
                };

                Ok(Node::Row {
                    nodes: self.arena.push_slice(&[inner]),
                    attrs: RowAttrs {
                        math_shift_compact: true,
                        style,
                        ..RowAttrs::DEFAULT
                    },
                })
            }
            Token::Color => 'color: {
                let color = if Token::SquareBracketOpen == *self.tokens.peek().token() {
                    let next = self.next_token()?;
                    debug_assert_eq!(&Token::SquareBracketOpen, next.token(),);
                    let mut type_name_builder = self.buffer.get_builder();
                    loop {
                        let next = self.tokens.next();
                        match next.as_ref().map(TokSpan::token) {
                            Ok(Token::Letter(c, _)) => {
                                type_name_builder.push_superchar(*c);
                            }
                            Ok(Token::SquareBracketClose) => break,
                            _ => {
                                break 'color Err(LatexError(
                                    span.into(),
                                    LatexErrKind::UnknownColor(
                                        type_name_builder.finish(self.arena).into(),
                                    ),
                                ));
                            }
                        }
                    }
                    let type_name = type_name_builder.finish(self.arena);
                    let Ok((color_description, span)) = self.parse_string_literal() else {
                        break 'color Err(LatexError(
                            span.into(),
                            LatexErrKind::ExpectedArgumentGotEOI,
                        ));
                    };
                    match type_name {
                        "rgb" => {
                            let mut parts = split_on_ascii(color_description, b',');
                            let (Some(r), Some(g), Some(b)) =
                                (parts.next(), parts.next(), parts.next())
                            else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(color_description.into()),
                                ));
                            };
                            let (Some(r), Some(g), Some(b)) = (
                                limited_float_parse(r.trim()),
                                limited_float_parse(g.trim()),
                                limited_float_parse(b.trim()),
                            ) else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(color_description.into()),
                                ));
                            };
                            (
                                (r * 255.0).round() as u8,
                                (g * 255.0).round() as u8,
                                (b * 255.0).round() as u8,
                            )
                        }
                        "RGB" => {
                            let mut parts = split_on_ascii(color_description, b',');
                            let (Some(r), Some(g), Some(b)) =
                                (parts.next(), parts.next(), parts.next())
                            else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(color_description.into()),
                                ));
                            };
                            let (Some(r), Some(g), Some(b)) = (
                                limited_float_parse(r.trim()),
                                limited_float_parse(g.trim()),
                                limited_float_parse(b.trim()),
                            ) else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(color_description.into()),
                                ));
                            };
                            (r as u8, g as u8, b as u8)
                        }
                        "HTML" => {
                            fn hex(h: u8) -> u8 {
                                match h {
                                    b'0'..=b'9' => h - b'0',
                                    b'a'..=b'f' => h - b'a' + 10,
                                    b'A'..=b'F' => h - b'A' + 10,
                                    _ => 0,
                                }
                            }
                            match color_description.as_bytes() {
                                &[r1, r2, g1, g2, b1, b2] => (
                                    hex(r1) * 16 + hex(r2),
                                    hex(g1) * 16 + hex(g2),
                                    hex(b1) * 16 + hex(b2),
                                ),
                                _ => {
                                    break 'color Err(LatexError(
                                        span,
                                        LatexErrKind::UnknownColor(color_description.into()),
                                    ));
                                }
                            }
                        }
                        unexpected => {
                            break 'color Err(LatexError(
                                span,
                                LatexErrKind::UnknownColor(unexpected.into()),
                            ));
                        }
                    }
                } else {
                    let (color_name, span) = self.parse_string_literal()?;
                    let Some(color) = get_color(color_name) else {
                        break 'color Err(LatexError(
                            span,
                            LatexErrKind::UnknownColor(color_name.into()),
                        ));
                    };
                    color
                };
                let content = self.parse_sequence(SequenceEnd::AnyEndToken, prev_class, true)?;
                Ok(Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attrs: RowAttrs {
                        color: Some(color),
                        ..RowAttrs::DEFAULT
                    },
                })
            }
            Token::Phantom(kind) => {
                let inner = self.parse_next(ParseAs::Arg)?;
                match kind {
                    PhantomKind::Full => Ok(Node::Phantom { node: inner }),
                    PhantomKind::H => Ok(Node::Padded {
                        node: self.arena.push(Node::Phantom { node: inner }),
                        width_0: false,
                        height_0: true,
                        left: None,
                        right: None,
                    }),
                    PhantomKind::V => Ok(Node::Padded {
                        node: self.arena.push(Node::Phantom { node: inner }),
                        width_0: true,
                        height_0: false,
                        left: None,
                        right: None,
                    }),
                }
            }
            Token::Style(style) => {
                let old_style = mem::replace(&mut self.state.style, style);
                let content = self.parse_sequence(SequenceEnd::AnyEndToken, prev_class, true)?;
                self.state.style = old_style;
                Ok(Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attrs: RowAttrs {
                        style: Some(style),
                        ..RowAttrs::DEFAULT
                    },
                })
            }
            tok @ (Token::Underscore | Token::Circumflex | Token::Prime(_)) => {
                let bounds = self
                    .get_bounds(Some((tok, span)))?
                    .ensure_no_explicit_limits()?;

                // We use an empty row as the base.
                let target = self.commit(Node::EMPTY_ROW);

                match bounds.try_wrap_node_subsup(target) {
                    Some(node) => Ok(node),
                    None => unreachable!(),
                }
            }
            Token::Prescript => {
                let sup = self.parse_next(ParseAs::Arg)?;
                let sub = self.parse_next(ParseAs::Arg)?;
                let base = self.parse_next(ParseAs::Arg)?;
                let pre = self
                    .arena
                    .alloc_multiscript_pairs(&[MultiscriptPair { sub, sup }]);
                Ok(Node::Multiscripts {
                    base,
                    pre,
                    post: &const { &[] },
                })
            }
            Token::Sideset => {
                // Collect arguments
                let (pre_bounds_limits, mut after_pre_bounds) = self.get_bounds_arg()?;
                let pre_bounds = pre_bounds_limits.ensure_no_explicit_limits()?;
                let (post_bounds_limits, mut after_post_bounds) = self.get_bounds_arg()?;
                // TeX allows `\limits` here, but no good way to represent it in MathML
                let post_bounds = post_bounds_limits.ensure_no_explicit_limits()?;
                let base = self.tokens.read_argument(false)?.into_one()?;

                // Construct node for base op
                let op = match *base.token() {
                    Token::Op(op) if matches!(op.category(), OpCategory::H | OpCategory::J) => op,
                    _ => {
                        return Err(Box::new(LatexError(
                            base.span().into(),
                            LatexErrKind::ExpectedLargeOp,
                        )));
                    }
                };
                let has_movable_limits: bool = matches!(op.category(), OpCategory::J);
                let (left, right) = self.mathop_spacing(parse_as, prev_class, false)?;
                let attrs = if has_movable_limits {
                    OpAttrs::NO_MOVABLE_LIMITS
                } else {
                    OpAttrs::empty()
                };
                let op_node = self.arena.push(Node::Operator {
                    op: op.as_op(),
                    attrs,
                    size: None,
                    left,
                    right,
                });

                // Add `post_pre_bounds` tokens to op node
                let op_and_after_pre_bounds_node = if after_pre_bounds.is_empty() {
                    op_node
                } else {
                    after_pre_bounds.push(op_node);

                    self.arena.push(Node::Row {
                        nodes: self.arena.push_slice(&after_pre_bounds),
                        attrs: RowAttrs::DEFAULT,
                    })
                };

                // construct sidesetted node
                // FIXME: use `msub`/`msup`/`msubsup` where possible
                let pre = self.arena.alloc_multiscript_pairs(&[pre_bounds.into()]);
                let post = self.arena.alloc_multiscript_pairs(&[post_bounds.into()]);
                let sidesetted_node = Node::Multiscripts {
                    base: op_and_after_pre_bounds_node,
                    pre,
                    post,
                };

                // Add `post_pre_bounds` tokens to sidesetted node
                let sidesetted_with_after_post_bounds_node = if after_post_bounds.is_empty() {
                    sidesetted_node
                } else {
                    after_post_bounds.insert(0, self.arena.push(sidesetted_node));

                    Node::Row {
                        nodes: self.arena.push_slice(&after_post_bounds),
                        attrs: RowAttrs::DEFAULT,
                    }
                };

                // add trailing bounds to sidesettend node
                let trailing_bounds = self.get_bounds(None)?;
                if !trailing_bounds.bounds.is_trivial() {
                    let trailing_limits = trailing_bounds.limits();
                    let use_underover = matches!(trailing_limits, Some(LimitsKind::Always))
                        || (self.state.style == Style::Display
                            && matches!(trailing_limits, None | Some(LimitsKind::Display)));

                    if use_underover {
                        // construct node for under/overscripts
                        Ok(trailing_bounds
                            .bounds
                            .try_wrap_node_underover(
                                self.arena.push(sidesetted_with_after_post_bounds_node),
                            )
                            .unwrap_or_else(|| unreachable!()))
                    } else {
                        // construct node for super/subscripts
                        Ok(trailing_bounds
                            .bounds
                            .try_wrap_node_subsup(
                                self.arena.push(sidesetted_with_after_post_bounds_node),
                            )
                            .unwrap_or_else(|| unreachable!()))
                    }
                } else {
                    Ok(sidesetted_with_after_post_bounds_node)
                }
            }
            Token::Limits(kind) => Err(LatexError(
                span.into(),
                LatexErrKind::CannotBeUsedHere {
                    got: kind.into(),
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
            Token::XArrow(rel) => {
                // The leading and trailing 5mu spaces are ignored for character class
                // considerations; the class of the whole construct is that of a relation.
                class = Class::Relation;

                // Parse the over-argument in the same state the original token-stream
                // expansion used: `style` is set by the outer `\overset`, and
                // `right_boundary_hack` is set by the inner `\overset`'s target group.
                let old_style = mem::replace(&mut self.state.style, Style::Script);
                let old_boundary_hack = mem::replace(&mut self.state.right_boundary_hack, true);

                // Optional under-argument in square brackets (e.g. `\xrightarrow[a]{b}`),
                // handled in the same style as `\sqrt`'s optional degree argument.
                let next = self.next_token();
                let (under_arg, over_arg) = if let Ok(tokloc) = next
                    && matches!(tokloc.token(), Token::SquareBracketOpen)
                {
                    let nodes = self.parse_sequence(
                        SequenceEnd::EndToken(EndToken::SquareBracketClose),
                        Class::Open,
                        false,
                    )?;
                    let under = node_vec_to_node(self.arena, &nodes, false);
                    let over = self.parse_next(ParseAs::Arg)?;
                    (Some(under), over)
                } else {
                    let over = self.parse_token(next, ParseAs::Arg, Class::Default)?.1;
                    (None, over)
                };

                self.state.style = old_style;
                self.state.right_boundary_hack = old_boundary_hack;
                // Re-compute the next class.
                let next_class = self.tokens.peek_class_token(parse_as.in_sequence())?;

                let pad = &const { Node::Space(LatexUnit::Em.length_with_unit(0.4286)) };
                let label_space = &const { Node::Space(LatexUnit::Em.length_with_unit(3.5)) };
                let over_label = self.commit(Node::Over {
                    symbol: label_space,
                    target: node_vec_to_node(self.arena, &[pad, over_arg, pad], false),
                });

                // Stretchy relation: an arrow from the `A` relation category is stretchy
                // by default; otherwise we need to explicitly request stretching. The
                // spacing of the arrow is computed as for a plain `Token::Relation`, using
                // the classes of the characters surrounding the whole `\xarrow` construct
                // (the inner 5mu spaces are ignored for this).
                let attrs = match rel.category() {
                    RelCategory::A => OpAttrs::empty(),
                    RelCategory::Default | RelCategory::DandForceDefault => OpAttrs::STRETCHY_TRUE,
                };
                let (left, right) = self.state.relation_spacing(prev_class, next_class, false);
                let arrow = self.commit(Node::Operator {
                    op: rel.as_op(),
                    attrs,
                    left,
                    right,
                    size: None,
                });

                let center = if let Some(under_arg) = under_arg {
                    let under_label = self.commit(Node::Under {
                        symbol: label_space,
                        target: node_vec_to_node(self.arena, &[pad, under_arg, pad], false),
                    });
                    self.commit(Node::UnderOver {
                        target: arrow,
                        under: under_label,
                        over: over_label,
                    })
                } else {
                    self.commit(Node::Over {
                        symbol: over_label,
                        target: arrow,
                    })
                };

                let outer_space = &const { Node::Space(LatexUnit::Mu.length_with_unit(5.0)) };
                Ok(Node::Row {
                    nodes: self.arena.push_slice(&[outer_space, center, outer_space]),
                    attrs: RowAttrs::DEFAULT,
                })
            }
            Token::CompositeRelation {
                rel_category,
                combined,
                parts,
            } => {
                class = Class::Relation;
                if let crate::UnicodeSubstitution::Conventional = self.unicode_substitution {
                    // Use the `combined` character if unicode substitution is enabled.
                    let attrs = relation_attrs(rel_category);
                    let (left, right) = self.state.relation_spacing(prev_class, next_class, false);
                    Ok(Node::Operator {
                        op: combined.as_op(),
                        attrs,
                        left,
                        right,
                        size: None,
                    })
                } else {
                    self.tokens.queue_in_front(parts);
                    let token = self.next_token();
                    // TODO: Use `become` here once it is stable.
                    return self.parse_token(token, parse_as, prev_class);
                }
            }
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
            Token::InternalStringLiteral(content) => {
                if let Some(MathVariant::Transform(tf)) = self.state.transform {
                    let mut builder = self.buffer.get_builder();
                    for c in content.chars() {
                        builder.push_superchar(tf.transform_char(c, false));
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
    /// These bounds are preceeded by `_`, `^`, or `'`.
    /// `first` should be `Some(Token::Underscore | Token::Circumflex | Token::Prime)`
    /// to indicate that one of these was already consumed as a start of the bound,
    /// or `None` otherwise.
    fn get_bounds(
        &mut self,
        mut first: Option<(Token, Span)>,
    ) -> ParseResult<BoundsWithLimits<'arena>> {
        debug_assert!(matches!(
            first,
            Some((Token::Underscore | Token::Circumflex | Token::Prime(_), _)) | None
        ));

        let mut ret: BoundsWithLimits = BoundsWithLimits::default();

        loop {
            // retreive and consume next token
            let (next_token, next_span) = first.unwrap_or_else(|| self.tokens.peek().into_parts());
            let next_token = next_token.unwrap_math();
            if matches!(
                next_token,
                Token::Underscore | Token::Circumflex | Token::Prime(_) | Token::Limits(_)
            ) && first.take().is_none()
            {
                self.tokens.next()?;
            }

            // parse it into a bound
            let (bound_starter_kind, bound_to_replace) = match next_token {
                Token::Circumflex => (BoundStarterKind::Circumflex, &mut ret.bounds.1),
                Token::Prime(kind) => (BoundStarterKind::Prime(kind), &mut ret.bounds.1),
                Token::Underscore => (BoundStarterKind::Underscore, &mut ret.bounds.0),
                Token::Limits(kind) => {
                    ret.limits_span = Some((kind, next_span));
                    continue;
                }
                // we are done!
                _ => break Ok(ret),
            };

            let node = self.get_sub_or_sup(bound_starter_kind)?;
            if bound_to_replace.replace(node).is_some() {
                return Err(Box::new(LatexError(
                    next_span.into(),
                    LatexErrKind::DuplicateSubOrSup,
                )));
            }
        }
    }

    /// Helper for parsing `\sideset`.
    /// Get bounds that are an argument to a macro
    /// (either wrapped in braces, or a single bare `'`).
    /// The group can contain additional tokens after the bounds;
    /// these are returned in the tuple's second element.
    fn get_bounds_arg(
        &mut self,
    ) -> ParseResult<(BoundsWithLimits<'arena>, Vec<&'arena Node<'arena>>)> {
        let (first_tok, first_span) = self.tokens.peek().into_parts();
        let first_tok = first_tok.unwrap_math();
        match first_tok {
            Token::Prime(prime_kind) => {
                self.tokens.next()?;
                Ok((
                    BoundsWithLimits {
                        bounds: Bounds(None, Some(prime_kind.to_node())),
                        limits_span: None,
                    },
                    Vec::new(),
                ))
            }
            Token::GroupBegin => {
                self.tokens.next()?; // skip over group begin token

                let bounds = self.get_bounds(None)?;
                let after_bounds = self.parse_sequence(
                    SequenceEnd::EndToken(EndToken::GroupClose),
                    Class::Open,
                    false,
                )?;
                Ok((bounds, after_bounds))
            }
            Token::Limits(kind) => {
                self.tokens.next()?;
                Ok((
                    BoundsWithLimits {
                        bounds: Bounds(None, None),
                        limits_span: Some((kind, first_span)),
                    },
                    Vec::new(),
                ))
            }
            _ => {
                let next = self.parse_next(ParseAs::Arg)?;
                Ok((
                    BoundsWithLimits {
                        bounds: Bounds(None, None),
                        limits_span: None,
                    },
                    vec![next],
                ))
            }
        }
    }

    /// Parse the node after a `_` or `^` token.
    /// (We assume that token was already consumed.)
    fn get_sub_or_sup(&mut self, mut kind: BoundStarterKind) -> ParseResult<&'arena Node<'arena>> {
        let mut nodes = Vec::with_capacity(1);

        if let BoundStarterKind::Prime(prime_kind) = kind {
            let mut primes: Vec<(PrimeDirection, usize)> =
                vec![(prime_kind.direction(), prime_kind.count())];

            let followed_by_circumflex = loop {
                // We use `peek_any_token` here because primes can't be separated by whitespace
                // from each other or from a `^` that follows
                let next_tok = self.tokens.peek_any_token().token().unwrap_math();

                match next_tok {
                    Token::Prime(new_kind) => {
                        let last = primes.last_mut().unwrap();
                        if last.0 == new_kind.direction() {
                            last.1 += new_kind.count();
                        } else {
                            primes.push((new_kind.direction(), new_kind.count()));
                        }
                    }
                    Token::Circumflex => break true,
                    _ => break false,
                }

                self.tokens.next_any_token()?;
            };

            for (direction, count) in primes {
                let primes_arr: &[OrdLike] = match direction {
                    PrimeDirection::Forward => &[
                        symbol::PRIME,
                        symbol::DOUBLE_PRIME,
                        symbol::TRIPLE_PRIME,
                        symbol::QUADRUPLE_PRIME,
                    ],
                    PrimeDirection::Reversed => &[
                        symbol::REVERSED_PRIME,
                        symbol::REVERSED_DOUBLE_PRIME,
                        symbol::REVERSED_TRIPLE_PRIME,
                    ],
                };

                // If we have between 1 and 4 primes, we can use the predefined prime operators.
                if let Some(op) = primes_arr.get(count - 1) {
                    nodes.push(self.commit(Node::Operator {
                        op: op.as_op(),
                        attrs: OpAttrs::empty(),
                        left: None,
                        right: None,
                        size: None,
                    }));
                } else {
                    nodes.push(self.commit(Node::Operator {
                        op: primes_arr[0].as_op(),
                        attrs: OpAttrs::empty(),
                        left: None,
                        right: None,
                        size: None,
                    }));
                    for _ in 1..count {
                        nodes.push(&const { Node::Space(Length::new(-0.0833, LengthUnit::Em)) });
                        nodes.push(self.commit(Node::Operator {
                            op: primes_arr[0].as_op(),
                            attrs: OpAttrs::empty(),
                            left: None,
                            right: None,
                            size: None,
                        }));
                    }
                }
            }

            if followed_by_circumflex {
                // continue below
                self.tokens.next_any_token()?;
                kind = BoundStarterKind::Circumflex;
            }
        }

        if !matches!(kind, BoundStarterKind::Prime(_)) {
            let next = self.tokens.next()?;
            match next.token() {
                Token::Underscore | Token::Circumflex | Token::Prime(_) => {
                    return Err(Box::new(LatexError(
                        next.span().into(),
                        LatexErrKind::BoundFollowedByBound,
                    )));
                }
                Token::Eoi => {
                    return Err(Box::new(LatexError(
                        next.span().into(),
                        LatexErrKind::ExpectedArgumentGotEOI,
                    )));
                }
                _ => (),
            }
            let old_style = self.state.style;
            self.state.style = old_style.scriptify();
            let node = self.parse_token(Ok(next), ParseAs::Arg, Class::Default)?.1;
            self.state.style = old_style;
            nodes.push(node);
        }

        let ret_node = if nodes.len() == 1 {
            nodes[0]
        } else {
            self.arena.push(Node::Row {
                nodes: self.arena.push_slice(&nodes),
                attrs: RowAttrs::DEFAULT,
            })
        };

        Ok(ret_node)
    }

    fn mathop_spacing(
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
            ) || (matches!(self.state.style, Style::Script | Style::ScriptScript)
                && matches!(next_class, Class::Inner))
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
        // one character, we need it as a `SuperChar` and not as a `String`.
        let mut first_char: Option<SuperChar> = None;

        // Loop until we find a non-letter token.
        while let tok = self.tokens.peek().token()
            && matches!(
                tok,
                Token::Letter(_, _) | Token::UprightLetter(_) | Token::Digit(_)
            )
        {
            if matches!(tok, Token::Digit(_)) && matches!(tf, MathVariant::Normal) {
                // Don't collect digits in normal math variant.
                break;
            }
            let ch: SuperChar = match *tok {
                Token::Letter(ch, _) | Token::UprightLetter(ch) => ch,
                Token::Digit(ch) => ch.into(),
                _ => unreachable!(),
            };
            let is_upright = matches!(tok, Token::UprightLetter(_));
            let c = if let MathVariant::Transform(tf) = tf {
                tf.transform(ch, is_upright)
            } else {
                ch
            };
            builder.push_superchar(c);
            if first_char.is_none() {
                first_char = Some(c);
            }
            num_chars += 1;
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

    /// Parse the bare dimension argument of `\kern`, `\mkern`, `\hskip`, or `\mskip`,
    /// e.g. `1.5em` in `\kern1.5em`.
    ///
    /// The argument consists of an optional sign, digits with at most one decimal
    /// separator (`.` or `,`), and a two-letter unit. The unit need not be followed
    /// by whitespace: `x\kern1emx` is valid. Whitespace is allowed before the number
    /// and before the unit, but not within them: `\mkern 1 mu` is valid,
    /// but `\kern1 1em` is not.
    fn parse_kern_or_skip(&mut self, kind: UnitKind, span_end: usize) -> ParseResult<Node<'arena>> {
        let mut arg_start: Option<usize> = None;
        let mut arg_end = span_end;

        // An optional sign.
        let mut is_negative = false;
        if let &Token::MathOrTextMode(_, sign @ ('+' | '-')) = self.tokens.peek().token() {
            is_negative = sign == '-';
            let span = self.tokens.next()?.span();
            arg_start.get_or_insert(span.start());
            arg_end = span.end();
        }

        // The value: digits with at most one decimal separator (`.` or `,`).
        let mut buf = String::new();
        let mut in_number = false;
        loop {
            // Whitespace may precede the number, but it also ends the number,
            // so we only skip whitespace before the first digit.
            let tok = if in_number {
                self.tokens.peek_any_token().token()
            } else {
                self.tokens.peek().token()
            };
            let ch = match tok {
                Token::Digit(digit) => *digit,
                &FULL_STOP_TOKEN | &Token::MathOrTextMode(_, ',') => '.',
                _ => break,
            };
            buf.push(ch);
            let span = self.tokens.next()?.span();
            in_number = true;
            arg_start.get_or_insert(span.start());
            arg_end = span.end();
        }
        let value: &str = &buf;
        if value.is_empty() {
            return Err(Box::new(LatexError(
                arg_start.unwrap_or(span_end)..arg_end,
                LatexErrKind::ExpectedLength("".into()),
            )));
        }

        // The unit: exactly two letters.
        let mut unit = [0u8; 2];
        let mut unit_start: Option<usize> = None;
        for i in 0..unit.len() {
            // Whitespace may precede the unit, but the two letters of the unit
            // must not be separated by whitespace.
            let tokloc = if i == 0 {
                self.tokens.peek()
            } else {
                self.tokens.peek_any_token()
            };
            let peek_span = tokloc.span();
            let unit_char = if let Token::Letter(ch, _) = tokloc.token()
                && let Some(c) = ch.try_as_char()
                && c.is_ascii_alphabetic()
            {
                Some(c)
            } else {
                None
            };
            let Some(unit_char) = unit_char else {
                let start = unit_start.unwrap_or(peek_span.start());
                let unit_str = std::str::from_utf8(&unit[..i]).unwrap_or("");
                return Err(Box::new(LatexError(
                    start..peek_span.start(),
                    LatexErrKind::InvalidUnit(unit_str.into()),
                )));
            };
            unit[i] = unit_char as u8;
            let span = self.tokens.next()?.span();
            unit_start.get_or_insert(span.start());
            arg_end = span.end();
        }
        let unit = std::str::from_utf8(&unit).unwrap_or("");

        let unit_span = unit_start.unwrap_or(arg_end)..arg_end;
        let Ok(latex_unit) = LatexUnit::try_from(unit) else {
            return Err(Box::new(LatexError(
                unit_span,
                LatexErrKind::InvalidUnit(unit.into()),
            )));
        };
        let math_unit_expected = matches!(kind, UnitKind::MathUnits);
        if latex_unit.is_math_unit() != math_unit_expected {
            return Err(Box::new(LatexError(
                unit_span,
                LatexErrKind::IllegalUnit {
                    unit: unit.into(),
                    math_unit_expected,
                },
            )));
        }
        let arg_span = arg_start.unwrap_or(span_end)..arg_end;
        let Some(mut value) = limited_float_parse(value) else {
            buf.push_str(unit);
            return Err(Box::new(LatexError(
                arg_span,
                LatexErrKind::ExpectedLength(buf.into()),
            )));
        };
        if is_negative {
            value = -value;
        }
        Ok(Node::Space(latex_unit.length_with_unit(value)))
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
                if let Some(&tokloc) = iter.next() {
                    tokloc
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
                    LatexErrKind::ExpectedAscii,
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
        force: bool,
    ) -> (Option<MathSpacing>, Option<MathSpacing>) {
        (
            if matches!(
                prev_class,
                Class::Relation | Class::Open | Class::Punctuation
            ) || matches!(self.style, Style::Script | Style::ScriptScript)
            {
                Some(MathSpacing::Zero)
            } else if force {
                Some(MathSpacing::FiveMu) // force relation spacing
            } else {
                None
            },
            if matches!(
                next_class,
                Class::Relation | Class::Punctuation | Class::Close | Class::End
            ) || matches!(self.style, Style::Script | Style::ScriptScript)
            {
                Some(MathSpacing::Zero)
            } else if force {
                Some(MathSpacing::FiveMu) // force relation spacing
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
        ) || matches!(self.style, Style::Script | Style::ScriptScript)
        {
            Some(MathSpacing::Zero)
        } else if force {
            Some(MathSpacing::FourMu) // force binary op spacing
        } else {
            None
        }
    }

    fn punctuation_spacing(
        &self,
        next_class: Class,
        force: bool,
    ) -> (Option<MathSpacing>, Option<MathSpacing>) {
        let left = force.then_some(MathSpacing::Zero);

        let right = if matches!(next_class, Class::End)
            || matches!(self.style, Style::Script | Style::ScriptScript)
        {
            Some(MathSpacing::Zero)
        } else if force {
            Some(MathSpacing::ThreeMu)
        } else {
            None
        };

        (left, right)
    }

    fn mathinner_spacing(
        &self,
        prev_class: Class,
        next_class: Class,
        force: bool,
    ) -> (Option<MathSpacing>, Option<MathSpacing>) {
        let left = if matches!(
            prev_class,
            Class::Relation | Class::Punctuation | Class::Operator | Class::BinaryOp | Class::Open
        ) || matches!(self.style, Style::Script | Style::ScriptScript)
        {
            Some(MathSpacing::Zero)
        } else if force {
            Some(MathSpacing::ThreeMu)
        } else {
            None
        };
        let right = if matches!(
            next_class,
            Class::Relation | Class::BinaryOp | Class::Close | Class::End
        ) || (matches!(self.style, Style::Script | Style::ScriptScript)
            && !matches!(next_class, Class::Operator))
        {
            Some(MathSpacing::Zero)
        } else if force {
            Some(MathSpacing::ThreeMu)
        } else {
            None
        };
        (left, right)
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
            if let Node::Operator { op, attrs, .. } = **single {
                arena.push(Node::Operator {
                    op,
                    attrs,
                    left: None,
                    right: None,
                    size: None,
                })
            } else {
                single
            }
        } else {
            single
        }
    } else {
        let nodes = arena.push_slice(nodes);
        arena.push(Node::Row {
            nodes,
            attrs: RowAttrs::DEFAULT,
        })
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
    const SQ_L_BRACKET: StretchableOp =
        StretchableOp::from_ord(symbol::LEFT_SQUARE_BRACKET).unwrap();
    const SQ_R_BRACKET: StretchableOp =
        StretchableOp::from_ord(symbol::RIGHT_SQUARE_BRACKET).unwrap();
    let delim = match tok {
        Token::Ord(op) | Token::Open(op) | Token::Close(op) => StretchableOp::from_ord(op),
        Token::Relation(rel) => StretchableOp::from_rel(rel),
        Token::ForceOpen(op, stretch) | Token::ForceClose(op, stretch) => {
            StretchableOp::from_force_stretchy(op, stretch)
        }
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

fn relation_attrs(rel_category: symbol::RelCategory) -> OpAttrs {
    match rel_category {
        // Category A relations are stretchy by default; we explicitly
        // disable stretching for them.
        RelCategory::A => OpAttrs::STRETCHY_FALSE,
        RelCategory::Default => OpAttrs::empty(),
        // To get the right spacing on `DandForceDefault` relations, we have to
        // explicitly set the form to "infix".
        RelCategory::DandForceDefault => OpAttrs::FORM_INFIX,
    }
}

/// sub, sup
#[derive(Clone, Copy, Debug, Default)]
struct Bounds<'arena>(Option<&'arena Node<'arena>>, Option<&'arena Node<'arena>>);

impl<'arena> Bounds<'arena> {
    const fn is_trivial(self) -> bool {
        matches!(self, Self(None, None))
    }

    /// Returns `None` if the bounds are trivial
    fn try_wrap_node_underover(&self, node: &'arena Node<'arena>) -> Option<Node<'arena>> {
        match self {
            Self(None, None) => None,
            Self(Some(under), Some(over)) => Some(Node::UnderOver {
                target: node,
                under,
                over,
            }),
            Self(Some(symbol), None) => Some(Node::Under {
                target: node,
                symbol,
            }),
            Self(None, Some(symbol)) => Some(Node::Over {
                target: node,
                symbol,
            }),
        }
    }

    /// Returns `None` if the bounds are trivial
    fn try_wrap_node_subsup(&self, node: &'arena Node<'arena>) -> Option<Node<'arena>> {
        match self {
            Self(None, None) => None,
            Self(Some(sub), Some(sup)) => Some(Node::SubSup {
                target: node,
                sub,
                sup,
            }),
            Self(Some(symbol), None) => Some(Node::Sub {
                target: node,
                symbol,
            }),
            Self(None, Some(symbol)) => Some(Node::Sup {
                target: node,
                symbol,
            }),
        }
    }
}

impl<'arena> From<Bounds<'arena>> for MultiscriptPair<'arena> {
    fn from(bounds: Bounds<'arena>) -> Self {
        Self {
            sub: bounds.0.unwrap_or(&Node::EMPTY_ROW),
            sup: bounds.1.unwrap_or(&Node::EMPTY_ROW),
        }
    }
}

/// Return type of [`Parser::get_bounds_arg`].
#[derive(Clone, Copy, Debug, Default)]
struct BoundsWithLimits<'arena> {
    /// The bounds at the start of the arg
    bounds: Bounds<'arena>,
    /// The span of the `Limits` token applying to the bounds,
    /// if it was present
    limits_span: Option<(LimitsKind, Span)>,
}

impl<'arena> BoundsWithLimits<'arena> {
    /// Returns the bounds in this [`BoundsWithLimits`]
    /// if it has no `\limits`/`\nolimits`/`\displaylimits`,
    /// or an error otherwise.
    fn ensure_no_explicit_limits(self) -> ParseResult<Bounds<'arena>> {
        if let Some((limits_kind, limits_span)) = self.limits_span {
            Err(Box::new(LatexError(
                limits_span.into(),
                LatexErrKind::CannotBeUsedHere {
                    got: limits_kind.into(),
                    correct_place: Place::AfterBigOp,
                },
            )))
        } else {
            Ok(self.bounds)
        }
    }

    fn limits(self) -> Option<LimitsKind> {
        self.limits_span.map(|ls| ls.0)
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
            ("cancel", r"\cancel{abc}"),
            ("sum_relation", r"{\sum = 4}"),
            ("int_relation", r"{\int = 4}"),
            ("int_bounds_relation", r"{\int_0^\infty = 4}"),
            ("phantom_full", r"\phantom{a}"),
            ("mathstrut", r"\mathstrut"),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let mut equation_counter = 0usize;
            let mut label_map = FxHashMap::default();
            let l = Lexer::new(problem, false, None);
            let mut p = Parser::new(
                l,
                &arena,
                &mut equation_counter,
                &mut label_map,
                Style::Text,
                crate::UnicodeSubstitution::Conventional,
            )
            .unwrap();
            let ast = p.parse().expect("Parsing failed");
            assert_ron_snapshot!(name, &ast, problem);
        }
    }

    #[test]
    fn ast_from_token_stream_test() {
        use crate::token::Token::{
            CustomSpace, GroupBegin, GroupEnd, InternalStringLiteral, Letter, Text,
        };
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
                    const { Letter(SuperChar::from_char('a'), Mode::MathOrText) },
                    InternalStringLiteral("hi"),
                    GroupEnd,
                ],
            ),
            (
                "space_internal_string_literal",
                &[
                    CustomSpace(UnitKind::TextUnits),
                    InternalStringLiteral("3em"),
                ],
            ),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let mut equation_counter = 0usize;
            let mut label_map = FxHashMap::default();
            let l = Lexer::new("", false, None);
            let mut p = Parser::new(
                l,
                &arena,
                &mut equation_counter,
                &mut label_map,
                Style::Text,
                crate::UnicodeSubstitution::Conventional,
            )
            .unwrap();
            p.tokens.queue_in_front(problem);
            let ast = p.parse().expect("Parsing failed");
            let problem = format!("{:?}", problem);
            assert_ron_snapshot!(name, &ast, &problem);
        }
    }

    #[test]
    fn string_literal_test() {
        // let literal = r#" !"'()*+,-./012:;<=>?@`ABCabc|"#;
        let literal = r#" !()*+,-./012:;<=>?@ABCabc|"#;
        let input = format!("{{{}}}", literal);
        let arena = Arena::new();
        let mut equation_counter = 0usize;
        let mut label_map = FxHashMap::default();
        let l = Lexer::new(&input, false, None);
        let mut p = Parser::new(
            l,
            &arena,
            &mut equation_counter,
            &mut label_map,
            Style::Text,
            crate::UnicodeSubstitution::Conventional,
        )
        .unwrap();
        let parsed = p
            .parse_string_literal()
            .unwrap_or_else(|e| panic!("failed with error '{}'", e));
        assert_eq!(parsed.0, literal);
    }
}
