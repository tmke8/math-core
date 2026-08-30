use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::{mem, ops::Range};

use kstring::KString;
use mathml_renderer::{
    arena::{Arena, Buffer},
    ast::{MultiscriptPair, Node},
    attribute::{FracAttr, LetterAttr, MathSpacing, OpAttrs, RowAttrs, Style},
    length::{Length, LengthSet, LengthUnit},
    super_char::SuperChar,
    symbol::{self, OpCategory, OrdCategory, OrdLike, RelCategory},
    table::{EquationTag, RowLabelInfo},
};

use crate::{
    ParserConfig,
    atof::limited_float_parse,
    character_class::{
        Class, DelimiterSpacing, MathVariant, ParenType, StretchableOp, Stretchy, fenced,
    },
    color_defs::get_color,
    custom_cmds::RecordedToken,
    environments::{
        CLOSE_BRACE, CLOSE_BRACKET, CLOSE_PAREN, Env, EnvState, OPEN_BRACE, OPEN_BRACKET,
        OPEN_PAREN,
    },
    error::{DelimiterModifier, LatexErrKind, LatexError, LimitedUsabilityToken, Place},
    global_state::GlobalState,
    lexer::{Lexer, recover_limited_ascii},
    predefined,
    specifications::{LatexUnit, parse_column_specification, parse_length_specification},
    split_on_ascii::split_on_ascii,
    text_parser::TextSnippet,
    token::{
        DefineMode, EndToken, InfixDelim, LimitsKind, MathClassKind, Mode, PhantomKind,
        PrimeDirection, PrimeKind, Span, TokSpan, Token, UnitKind, VerticalLineDef,
    },
    token_queue::{CmdArgs, MacroArgument, OneOrNone, TokenQueue},
};

const FULL_STOP_TOKEN: Token = Token::Letter(SuperChar::from_char('.'), Mode::MathOrText);

pub(crate) struct Parser<'state, 'arena> {
    pub(super) tokens: TokenQueue<'state, 'arena>,
    pub(super) buffer: Buffer,
    pub(super) arena: &'arena Arena,
    state: ParserState<'arena>,
}

#[derive(Debug)]
struct ParserState<'arena> {
    /// The arguments of the custom command which is being expanded right now. They are only
    /// needed until the body has been queued, so this is really just a reusable buffer.
    cmd_args: CmdArgs<'arena>,
    transform: Option<MathVariant>,
    /// `true` if the boundaries at the end of a sequence are not real boundaries;
    /// this is not the case for style-only rows.
    /// This is currently a hack, which should be replaced by a more robust solution later.
    right_boundary_hack: bool,
    env: EnvState<'arena>,
    /// The current style (display/text/script/scriptscript) for the surrounding group.
    style: Style,
    /// The current meaning of the character `|`, which `\set`, `\Set` and `\Braket` change.
    vertical_line_def: Option<VerticalLineDef>,
    /// How many more custom commands may be expanded before we give up.
    ///
    /// Names are resolved when a command is expanded, so a definition may refer to itself,
    /// directly or through other definitions, and expanding it would never end. Rather than
    /// detecting that, we simply stop after a while, as LaTeX and KaTeX do. How long that
    /// takes is configurable; see [`crate::MaxExpansions`].
    expansions_left: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SequenceEnd {
    EndToken(EndToken),
    AnyEndToken,
}

impl SequenceEnd {
    #[inline]
    fn matches(self, other: &Token) -> bool {
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

/// What one token turned into.
enum Parsed<'arena> {
    Node(Class, &'arena Node<'arena>),
    /// We performed an expansion and the next token is the first token of the expansion.
    Expansion,
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

impl<'state, 'arena> Parser<'state, 'arena> {
    pub(crate) fn new(
        lexer: Lexer<'arena>,
        arena: &'arena Arena,
        parser_cfg: &'arena ParserConfig,
        global_state: &'state mut GlobalState,
        style: Style,
    ) -> ParseResult<Self> {
        let input_length = lexer.input_length();
        let tokens = TokenQueue::new(lexer, parser_cfg, global_state)?;
        Ok(Parser {
            tokens,
            buffer: Buffer::new(input_length),
            arena,
            state: ParserState {
                cmd_args: CmdArgs::default(),
                transform: None,
                right_boundary_hack: false,
                env: EnvState::default(),
                style,
                vertical_line_def: None,
                expansions_left: parser_cfg.max_expansions.0,
            },
        })
    }

    #[inline(never)]
    fn next_token(&mut self) -> ParseResult<TokSpan> {
        self.tokens.next()
    }

    #[inline]
    pub(crate) fn parse(&mut self) -> ParseResult<Vec<&'arena Node<'arena>>> {
        self.parse_sequence(SequenceEnd::EndToken(EndToken::Eoi), Class::Open, true)
    }

    /// Parse a sequence of tokens, if the parser is not in an argument.
    ///
    /// Arguments bind tighter than some kinds of grouping, so, if in an argument,
    /// the sequence is empty.
    fn parse_sequence_if_in_sequence(
        &mut self,
        parse_as: ParseAs,
        span: Span,
        sequence_end: SequenceEnd,
        prev_class: Class,
        keep_end_token: bool,
    ) -> ParseResult<Vec<&'arena Node<'arena>>> {
        if !parse_as.in_sequence() {
            return Err(LatexError(span.into(), LatexErrKind::CannotBeUsedAsArgument).into());
        }
        self.parse_sequence(sequence_end, prev_class, keep_end_token)
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
        tokspan: &TokSpan,
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
            // In a sequence, `\relax` really does produce nothing at all. In an argument it
            // has to produce an empty group instead; see `parse_token`.
            Token::Relax => Ok(()),
            Token::TransformSwitch(tf) => {
                self.state.transform = Some(tf);
                Ok(())
            }
            Token::NewCommand(mode) => {
                self.define_command(mode)?;
                Ok(())
            }
            Token::Let => {
                self.let_command(false)?;
                Ok(())
            }
            Token::Def(is_global) => {
                self.def_command(is_global)?;
                Ok(())
            }
            Token::Global => {
                self.global_prefix()?;
                Ok(())
            }
            Token::VerticalLineDef(def) => {
                self.state.vertical_line_def = def;
                Ok(())
            }
            Token::NoNumber => {
                if let Some(numbered_state) = &mut self.state.env.numbered {
                    numbered_state.suppress_next_number = true;
                }
                Ok(())
            }
            Token::Tag { parenthesized } => {
                let (tag_name, _) = self.parse_string_literal()?;
                if let Some(numbered_state) = &mut self.state.env.numbered {
                    numbered_state.custom_next_tag = Some(EquationTag {
                        text: tag_name,
                        parenthesized,
                    });
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
                if let Some(numbered_state) = &mut self.state.env.numbered {
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
    ///
    /// A token which expands to other tokens is not a node of its own, so we go around again
    /// with the first token of the expansion. This is a loop rather than a tail call, because
    /// one expansion can lead to the next without limit.
    fn parse_token(
        &mut self,
        cur_tokloc: ParseResult<TokSpan>,
        parse_as: ParseAs,
        prev_class: Class,
    ) -> ParseResult<(Class, &'arena Node<'arena>)> {
        let mut cur_tokloc = cur_tokloc;
        loop {
            match self.parse_one_token(cur_tokloc, parse_as, prev_class)? {
                Parsed::Node(class, node) => return Ok((class, node)),
                Parsed::Expansion => cur_tokloc = self.next_token(),
            }
        }
    }

    /// Parse one token into a node, or into the first token of its expansion.
    fn parse_one_token(
        &mut self,
        cur_tokloc: ParseResult<TokSpan>,
        parse_as: ParseAs,
        prev_class: Class,
    ) -> ParseResult<Parsed<'arena>> {
        let (cur_token, span) = cur_tokloc?.into_parts();
        let mut class = Class::default();
        let next_class = self.peek_class_token(parse_as.in_sequence())?;
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
                        return Ok(Parsed::Node(class, target));
                    }
                } else if let Some(node) = bounds.try_wrap_node_subsup(target) {
                    Ok(node)
                } else {
                    return Ok(Parsed::Node(class, target));
                }
            }
            Token::Ord(ord) => 'ord: {
                if matches!(ord, symbol::VERTICAL_LINE)
                    && let Some(vld) = self.state.vertical_line_def
                {
                    let mut attrs = OpAttrs::FORM_INFIX;
                    // For vertical line, `form="infix"` implies relation spacing, so we can set
                    // spacing to `None` in order to get relation spacing.
                    let (stretchy, spacing) = match vld {
                        VerticalLineDef::OpSpacingStretchy => (true, Some(MathSpacing::ThreeMu)),
                        VerticalLineDef::RelSpacingStretchy => (true, None),
                        VerticalLineDef::RelSpacing => (false, None),
                    };
                    // `form="infix"` also implies `stretchy="false"`, so we have to explicitly set
                    // `stretchy="true"` if the vertical line is stretchy.
                    if stretchy {
                        attrs |= OpAttrs::STRETCHY_TRUE;
                    }
                    break 'ord Ok(Node::Operator {
                        op: ord.as_op(),
                        attrs,
                        left: spacing,
                        right: spacing,
                        size: None,
                    });
                }
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
            Token::ForceOrd(op) => {
                class = Class::Default;
                Ok(Node::Operator {
                    op,
                    attrs: OpAttrs::empty(),
                    left: Some(MathSpacing::Zero),
                    right: Some(MathSpacing::Zero),
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
                let next_class = self.peek_class_token(parse_as.in_sequence())?;
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
                        voffset: None,
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
            Token::ForceMathInner(op) => {
                class = Class::Inner;
                let (left, right) = self.state.mathinner_spacing(prev_class, next_class, true);
                Ok(Node::Operator {
                    op,
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
                                    unit: KString::from_ref(unit),
                                    math_unit_expected,
                                },
                            ))
                        }
                    }
                    None => Err(LatexError(
                        span,
                        LatexErrKind::ExpectedLength(KString::from_ref(length)),
                    )),
                }
            }
            Token::KernOrSkip(kind) => {
                // Spaces pass through the symbol class.
                class = prev_class;
                Ok(self.parse_kern_or_skip(kind, span.end())?)
            }
            Token::NonBreakingSpace => Ok(Node::Text {
                text_style: None,
                text_size: None,
                text: "\u{A0}",
            }),
            Token::Sqrt => {
                let next = self.next_token();
                if let Ok(tokloc) = &next
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
                fn get_delimiter(
                    parser: &mut Parser<'_, '_>,
                ) -> Result<Option<StretchableOp>, Box<LatexError>> {
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
                                    LatexErrKind::ExpectedLength(KString::from_ref(decimal)),
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
                    Ok(Node::OverAccent(op.as_op(), attr, target))
                } else {
                    Ok(Node::UnderAccent(op.as_op(), attr, target))
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
                let base = if is_over {
                    Node::OverAccent(x.as_op(), OpAttrs::empty(), target)
                } else {
                    Node::UnderAccent(x.as_op(), OpAttrs::empty(), target)
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
                        None => return Ok(Parsed::Node(class, target)),
                    }
                } else {
                    match bounds.try_wrap_node_subsup(target) {
                        Some(node) => Ok(node),
                        None => return Ok(Parsed::Node(class, target)),
                    }
                }
            }
            ref tok @ (Token::PseudoOperator(name) | Token::PseudoOperatorLimits(name)) => {
                class = Class::Operator;
                let bounds_limits = self.get_bounds(None)?;
                let bounds = bounds_limits.bounds;

                let limits_by_default = matches!(tok, Token::PseudoOperatorLimits(_));
                let (use_underover, force_movable_limits) =
                    match (bounds_limits.limits(), limits_by_default) {
                        _ if bounds.is_trivial() => (false, false),
                        (None, true) | (Some(LimitsKind::Display), _) => (true, true),
                        (None, false) | (Some(LimitsKind::Never), _) => (false, false),
                        (Some(LimitsKind::Always), _) => (true, false),
                    };

                // Compute spacing after getting the bounds, so that we don't
                // consider tokens that are part of the bounds for spacing calculations.
                let (left, right) = self.mathop_spacing(parse_as, prev_class, true)?;
                let target = self.commit(Node::PseudoOp {
                    force_movable_limits,
                    left,
                    right,
                    name,
                });

                if use_underover {
                    match bounds.try_wrap_node_underover(target) {
                        Some(node) => Ok(node),
                        None => return Ok(Parsed::Node(class, target)),
                    }
                } else {
                    match bounds.try_wrap_node_subsup(target) {
                        Some(node) => Ok(node),
                        None => return Ok(Parsed::Node(class, target)),
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
                        force_movable_limits,
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
                            force_movable_limits,
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
                return Ok(Parsed::Node(Class::Close, content));
            }
            Token::TransformSwitch(_)
            | Token::NoNumber
            | Token::Tag { .. }
            | Token::Label
            | Token::NewCommand(_)
            | Token::Let
            | Token::Def(_)
            | Token::Global
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
                return Ok(Parsed::Node(
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
                class = Class::Open;
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
                    let next_class = self.peek_class_token(parse_as.in_sequence())?;
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
                let spec = if matches!(env, Env::Array | Env::DArray | Env::Subarray) {
                    // Parse the array options.
                    let (options, span) = self.parse_string_literal()?;
                    let Some(mut spec) = parse_column_specification(options, self.arena) else {
                        break 'begin_env Err(LatexError(
                            span,
                            LatexErrKind::ExpectedColSpec(KString::from_ref(options)),
                        ));
                    };
                    if matches!(env, Env::Subarray) {
                        spec.is_sub = true;
                    }
                    Some(spec)
                } else {
                    None
                };

                // A `\hline`/`\hdashline` directly at the start of the environment becomes the top
                // border of the whole table.
                let border_top = if env.allows_hlines()
                    && let Token::HLine(line_type) = *self.tokens.peek().token()
                {
                    self.tokens.next()?;
                    Some(line_type)
                } else {
                    None
                };
                let initial_shove = if env.allows_shove()
                    && let Token::Shove(shove) = *self.tokens.peek().token()
                {
                    self.tokens.next()?;
                    Some(shove)
                } else {
                    None
                };

                // For arrays, the top border is stored in the array spec, which already carries
                // the borders coming from the column specification.
                let array_spec = spec.map(|mut spec| {
                    spec.border_top = border_top;
                    self.arena.alloc_array_spec(spec)
                });

                let old_style = mem::replace(&mut self.state.style, env.style());
                let old_env_state = mem::replace(&mut self.state.env, env.new_state());

                let content = self.arena.push_slice(&self.parse_sequence_if_in_sequence(
                    parse_as,
                    span,
                    SequenceEnd::EndToken(EndToken::End),
                    Class::Open,
                    true, // keep_end_token
                )?);

                self.state.style = old_style;
                let env_state = mem::replace(&mut self.state.env, old_env_state);
                let numbered_state = env_state.numbered;

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
                    match n.next_equation_tag(
                        &mut self.tokens.stores.global_state.equation_count,
                        true,
                        self.arena,
                    ) {
                        Ok(tag) => {
                            let link_target = n.label.take();
                            let info = if let Some(tag) = tag {
                                if let Some(label) = link_target {
                                    self.tokens.stores.global_state.label_map.insert(
                                        KString::from_ref(label),
                                        KString::from_ref(tag.text),
                                    );
                                }
                                Some(
                                    self.arena
                                        .alloc_row_label_info(RowLabelInfo { tag, link_target }),
                                )
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

                Ok(env.construct_node(
                    content,
                    array_spec,
                    self.arena,
                    last_row_info,
                    num_rows,
                    border_top,
                    initial_shove,
                ))
            }
            Token::OperatorName { with_limits } => {
                class = Class::Operator;
                let snippets = self.extract_text(None, None, false)?;
                let mut builder = self.buffer.get_builder();
                for TextSnippet(_style, _size, _voffset, text) in snippets {
                    builder.push_str(text);
                }
                let letters = builder.finish(self.arena);
                let bounds_limits = self.get_bounds(None)?;
                let bounds = bounds_limits.bounds;

                let (use_underover, force_movable_limits) =
                    match (bounds_limits.limits(), with_limits) {
                        _ if bounds.is_trivial() => (false, false),
                        (None, true) | (Some(LimitsKind::Display), _) => (true, true),
                        (None, false) | (Some(LimitsKind::Never), _) => (false, false),
                        (Some(LimitsKind::Always), _) => (true, false),
                    };

                // Compute spacing after getting the bounds, so that we don't
                // consider tokens that are part of the bounds for spacing calculations.
                let (left, right) = self.mathop_spacing(parse_as, prev_class, true)?;
                let op = self.commit(Node::PseudoOp {
                    force_movable_limits,
                    left,
                    right,
                    name: letters,
                });

                if use_underover {
                    match bounds.try_wrap_node_underover(op) {
                        Some(node) => Ok(node),
                        None => return Ok(Parsed::Node(class, op)),
                    }
                } else {
                    match bounds.try_wrap_node_subsup(op) {
                        Some(node) => Ok(node),
                        None => return Ok(Parsed::Node(class, op)),
                    }
                }
            }
            Token::Text(transform) => {
                let snippets = self.extract_text(transform, None, true)?;
                let nodes = snippets
                    .into_iter()
                    .map(|TextSnippet(text_style, text_size, text_voffset, text)| {
                        let text_node = Node::Text {
                            text_style,
                            text_size,
                            text,
                        };
                        let voffset = text_voffset
                            .as_ref()
                            .filter(|voffset| **voffset != LengthSet::zero())
                            .map(|tv| self.arena.alloc_length_set(*tv));
                        self.commit(if voffset.is_some() {
                            Node::Padded {
                                node: self.commit(text_node),
                                width_0: false,
                                height_0: false,
                                left: None,
                                right: None,
                                voffset,
                            }
                        } else {
                            text_node
                        })
                    })
                    .collect::<Vec<_>>();
                return Ok(Parsed::Node(
                    Class::Close,
                    node_vec_to_node(self.arena, &nodes, false),
                ));
            }
            Token::RaiseBox => {
                let (initial_voffset_str, span) = self.parse_string_literal()?;
                let initial_voffset = match parse_length_specification(initial_voffset_str) {
                    Some((voffset, unit, is_math_unit)) => {
                        if is_math_unit {
                            return Err(Box::new(LatexError(
                                span,
                                LatexErrKind::IllegalUnit {
                                    unit: KString::from_ref(unit),
                                    math_unit_expected: false,
                                },
                            )));
                        }
                        LengthSet::from(voffset)
                    }
                    None => {
                        return Err(Box::new(LatexError(
                            span,
                            LatexErrKind::ExpectedLength(KString::from_ref(initial_voffset_str)),
                        )));
                    }
                };
                let snippets = self.extract_text(None, Some(initial_voffset), true)?;
                let nodes = snippets
                    .into_iter()
                    .map(|TextSnippet(text_style, text_size, text_voffset, text)| {
                        let voffset = text_voffset
                            .filter(|voffset| *voffset != LengthSet::zero())
                            .as_ref()
                            .map(|tv| self.arena.alloc_length_set(*tv));
                        let text_node = Node::Text {
                            text_style,
                            text_size,
                            text,
                        };
                        self.commit(if voffset.is_some() {
                            Node::Padded {
                                node: self.commit(text_node),
                                width_0: false,
                                height_0: false,
                                left: None,
                                right: None,
                                voffset,
                            }
                        } else {
                            text_node
                        })
                    })
                    .collect::<Vec<_>>();
                return Ok(Parsed::Node(
                    Class::Close,
                    node_vec_to_node(self.arena, &nodes, false),
                ));
            }
            Token::NewColumn => {
                if self.state.env.allow_columns {
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
            tok @ (Token::HLine(_) | Token::Shove(_) | Token::Limits(_)) => {
                let (got, correct_place) = match tok {
                    Token::HLine(_) => (LimitedUsabilityToken::HLine, Place::ArrayRowStart),
                    Token::Shove(_) => (LimitedUsabilityToken::Shove, Place::MultlineRowStart),
                    Token::Limits(kind) => (kind.into(), Place::AfterBigOp),
                    _ => unreachable!(),
                };
                Err(LatexError(
                    span.into(),
                    LatexErrKind::CannotBeUsedHere { got, correct_place },
                ))
            }
            Token::NewLine => 'new_line: {
                class = Class::Open;
                if !self.state.env.meaningful_newlines {
                    // FIXME: Return something other than a row here, so that we can avoid creating
                    //       empty rows in places where they are not needed.
                    break 'new_line Ok(Node::EMPTY_ROW);
                }
                // A `\hline`/`\hdashline` directly after the `\\` (whitespace is skipped by `peek`)
                // becomes the top border of the row that follows. Only legal inside an array or
                // matrix; elsewhere it falls through to the `Token::HLine` error arm.
                let border_top = if self.state.env.allow_hlines
                    && let Token::HLine(line_type) = *self.tokens.peek().token()
                {
                    self.tokens.next()?;
                    Some(line_type)
                } else {
                    None
                };
                // A `\shoveleft` or `\shoveright` after a `\\` becomes the "shove" of the row that
                // follows.
                let shove = if self.state.env.allow_shove
                    && let Token::Shove(shove) = *self.tokens.peek().token()
                {
                    self.tokens.next()?;
                    Some(shove)
                } else {
                    None
                };
                if let Some(numbered_state) = &mut self.state.env.numbered {
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
                    match numbered_state.next_equation_tag(
                        &mut self.tokens.stores.global_state.equation_count,
                        false,
                        self.arena,
                    ) {
                        Ok(tag) => {
                            let link_target = numbered_state.label.take();
                            let label_info = if let Some(tag) = tag {
                                if let Some(label) = link_target {
                                    self.tokens.stores.global_state.label_map.insert(
                                        KString::from_ref(label),
                                        KString::from_ref(tag.text),
                                    );
                                }
                                Some(
                                    self.arena
                                        .alloc_row_label_info(RowLabelInfo { tag, link_target }),
                                )
                            } else {
                                // If we don't have a tag, we're not setting a link target either
                                None
                            };
                            Ok(Node::RowSeparator {
                                label_info,
                                border_top,
                                shove,
                            })
                        }
                        Err(()) => Err(LatexError(span.into(), LatexErrKind::HardLimitExceeded)),
                    }
                } else {
                    Ok(Node::RowSeparator {
                        label_info: None,
                        border_top,
                        shove,
                    })
                }
            }
            Token::EqRef => {
                let (label_name, _) = self.parse_string_literal()?;
                Ok(Node::EqRef(label_name))
            }
            Token::Cramped => {
                // Optional style argument in square brackets (e.g. `\cramped[\scriptstyle]{b}`),
                // handled in the same style as `\sqrt`'s optional degree argument.
                let next = self.next_token();
                let (style, inner) = if let Ok(tokloc) = &next
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
                                    LatexErrKind::UnknownColor(KString::from_ref(
                                        type_name_builder.finish(self.arena),
                                    )),
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
                                    LatexErrKind::UnknownColor(KString::from_ref(
                                        color_description,
                                    )),
                                ));
                            };
                            let (Some(r), Some(g), Some(b)) = (
                                limited_float_parse(r.trim()),
                                limited_float_parse(g.trim()),
                                limited_float_parse(b.trim()),
                            ) else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(KString::from_ref(
                                        color_description,
                                    )),
                                ));
                            };
                            (
                                (r * 255.0 + 0.5) as u8,
                                (g * 255.0 + 0.5) as u8,
                                (b * 255.0 + 0.5) as u8,
                            )
                        }
                        "RGB" => {
                            let mut parts = split_on_ascii(color_description, b',');
                            let (Some(r), Some(g), Some(b)) =
                                (parts.next(), parts.next(), parts.next())
                            else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(KString::from_ref(
                                        color_description,
                                    )),
                                ));
                            };
                            let (Some(r), Some(g), Some(b)) = (
                                limited_float_parse(r.trim()),
                                limited_float_parse(g.trim()),
                                limited_float_parse(b.trim()),
                            ) else {
                                break 'color Err(LatexError(
                                    span,
                                    LatexErrKind::UnknownColor(KString::from_ref(
                                        color_description,
                                    )),
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
                                        LatexErrKind::UnknownColor(KString::from_ref(
                                            color_description,
                                        )),
                                    ));
                                }
                            }
                        }
                        unexpected => {
                            break 'color Err(LatexError(
                                span,
                                LatexErrKind::UnknownColor(KString::from_ref(unexpected)),
                            ));
                        }
                    }
                } else {
                    let (color_name, span) = self.parse_string_literal()?;
                    let Some(color) = get_color(color_name) else {
                        break 'color Err(LatexError(
                            span,
                            LatexErrKind::UnknownColor(KString::from_ref(color_name)),
                        ));
                    };
                    color
                };
                let content = self.parse_sequence_if_in_sequence(
                    parse_as,
                    span,
                    SequenceEnd::AnyEndToken,
                    prev_class,
                    true,
                )?;
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
                        voffset: None,
                    }),
                    PhantomKind::V => Ok(Node::Padded {
                        node: self.arena.push(Node::Phantom { node: inner }),
                        width_0: true,
                        height_0: false,
                        left: None,
                        right: None,
                        voffset: None,
                    }),
                }
            }
            Token::Style(style) => {
                let old_style = mem::replace(&mut self.state.style, style);
                let content = self.parse_sequence_if_in_sequence(
                    parse_as,
                    span,
                    SequenceEnd::AnyEndToken,
                    prev_class,
                    true,
                )?;
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
            Token::Dots => {
                let next_token_is_mathbin_or_mathrel =
                    matches!(next_class, Class::BinaryOp | Class::Relation);
                self.tokens
                    .queue_in_front(if next_token_is_mathbin_or_mathrel {
                        &predefined::CDOTS
                    } else {
                        &predefined::DOTS
                    });
                return Ok(Parsed::Expansion);
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
            Token::Whitespace
            | Token::MathOrTextMode(_, _)
            | Token::VerticalLineDef(_)
            | Token::CustomCmdArg(_) => {
                // These tokens should have been skipped.
                // We report an internal error here.
                Err(LatexError(span.into(), LatexErrKind::Internal))
            }
            Token::TextMode(_) => Err(LatexError(span.into(), LatexErrKind::NotValidInMathMode)),
            Token::UnsupportedUnicodeMath(c) => Err(LatexError(
                span.into(),
                LatexErrKind::UnsupportedUnicodeMath(c),
            )),
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
                let (under_arg, over_arg) = if let Ok(tokloc) = &next
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
                let next_class = self.peek_class_token(parse_as.in_sequence())?;

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
            Token::CustomCmd(num_args, token_stream) => {
                self.count_expansion(span)?;
                self.read_cmd_args(num_args)?;
                self.tokens
                    .queue_body_substituting(token_stream, &self.state.cmd_args, span);
                return Ok(Parsed::Expansion);
            }
            Token::CustomCmdRef(source, num_args, _, start, end) => {
                self.count_expansion(span)?;
                self.read_cmd_args(num_args)?;
                if !self.tokens.queue_stored_body_substituting(
                    source,
                    start,
                    end,
                    &self.state.cmd_args,
                    span,
                ) {
                    return Err(Box::new(LatexError(span.into(), LatexErrKind::Internal)));
                }
                return Ok(Parsed::Expansion);
            }
            Token::CustomCmdArgInput(_) => Err(LatexError(
                span.into(),
                LatexErrKind::MacroParameterOutsideCustomCommand,
            )),
            Token::Dollar => Err(LatexError(span.into(), LatexErrKind::UnexpectedDollar)),
            // `\relax` produces no output, but it still has to produce a *node*, because it
            // may stand where a construct needs an argument: `x^\relax`, or `x^\nop` for a
            // command with an empty body. An empty row is what an empty argument produces
            // too, and MathML elements like `msup` need their children either way.
            // A `\relax` which is in a sequence rather than in an argument never gets here;
            // `handle_tokens_without_output` drops it before that.
            Token::Relax => Ok(Node::Row {
                nodes: &[],
                attrs: RowAttrs::DEFAULT,
            }),
            Token::UnresolvedCommand(name) => {
                // The name lives in the string pool, so we have to copy it out.
                let name = self.tokens.cmd_name(name);
                Ok(Node::UnknownCommand(self.arena.alloc_str(name)))
            }
            Token::MathChoice => {
                let chosen = match self.state.style {
                    Style::Display => 0,
                    Style::Text => 1,
                    Style::Script => 2,
                    Style::ScriptScript => 3,
                };
                // All four alternatives are read, but only the chosen one is queued; the
                // others are not typeset at all, unlike in LaTeX.
                self.read_cmd_args(4)?;
                self.tokens
                    .queue_arg_in_front(self.state.cmd_args.get(chosen), span);
                return Ok(Parsed::Expansion);
            }
            Token::MathChoiceInternal(_, choice) => {
                let token = choice.select(self.state.style);
                self.tokens.queue_in_front(&[TokSpan::new(token, span)]);
                return Ok(Parsed::Expansion);
            }
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
            Ok(n) => Ok(Parsed::Node(class, self.commit(n))),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Parse a `\newcommand` definition and register the new command.
    ///
    /// How far the definition reaches depends on
    /// [`global_group`](crate::MathCoreConfig::global_group): in the global group, it is
    /// available for the rest of the document, including the math snippets which come after
    /// this one; otherwise it is forgotten at the end of this snippet.
    ///
    /// The `mode` decides what happens when the name is (not) already defined:
    /// `\providecommand` parses the definition as usual and then throws it away if the name is
    /// taken, and `\renewcommand` requires the name to be taken and overwrites the definition.
    ///
    /// The body is recorded by name: a command in it means whatever its name means when the
    /// command being defined here is expanded, not what it meant while the body was being read.
    /// A body which mentions the command it is the body of therefore recurses, as it does in
    /// LaTeX, and is stopped by the expansion limit rather than by the old definition.
    fn define_command(&mut self, mode: DefineMode) -> ParseResult<()> {
        // The name of the new command, optionally wrapped in braces.
        let braced = matches!(self.tokens.peek().token(), Token::GroupBegin);
        if braced {
            self.next_token()?;
        }
        // We have to bypass the usual rejection of unknown commands here, because the name of
        // a command which doesn't exist yet is exactly what we are looking for.
        let name_tokspan = self.tokens.next_allowing_unresolved_command()?;
        // The name to register the definition under, and whether it replaces an existing
        // definition. `None` means throwing the definition away, which is what
        // `\providecommand` does when the name is already taken.
        let target = match *name_tokspan.token() {
            Token::UnresolvedCommand(name) => {
                if matches!(mode, DefineMode::Renew) {
                    return Err(Box::new(LatexError(
                        name_tokspan.span().into(),
                        LatexErrKind::CommandNotDefined,
                    )));
                }
                // The name lives in a string pool which doesn't outlive this snippet, so we
                // have to copy it out.
                let name = self.arena.alloc_str(self.tokens.cmd_name(name));
                Some((name, false))
            }
            // Any other token means that the name was already defined.
            _ => {
                // The name the token came from, which is `None` if it didn't come from a
                // command at all.
                let name = name_tokspan.name();
                let span: Range<usize> = name_tokspan.span().into();
                let Some(name) = name else {
                    return Err(Box::new(LatexError(
                        span,
                        LatexErrKind::ExpectedCommandName,
                    )));
                };
                match mode {
                    DefineMode::New => {
                        return Err(Box::new(LatexError(
                            span,
                            LatexErrKind::CommandAlreadyDefined,
                        )));
                    }
                    DefineMode::Provide => None,
                    // The name may belong to a builtin command or to a macro from the
                    // configuration; in both cases, the new definition shadows the old one,
                    // because resolution looks in the document's definitions first.
                    DefineMode::Renew => Some((name, true)),
                }
            }
        };
        if braced {
            let tokspan = self.next_token()?;
            if !matches!(tokspan.token(), Token::GroupEnd) {
                return Err(Box::new(LatexError(
                    tokspan.span().into(),
                    LatexErrKind::ExpectedExactlyOneToken,
                )));
            }
        }

        // The number of arguments, e.g. `[1]`. (Optional arguments are not supported.)
        let num_args = if matches!(self.tokens.peek().token(), Token::SquareBracketOpen) {
            self.next_token()?;
            let tokspan = self.next_token()?;
            let num_args = match *tokspan.token() {
                Token::Digit(digit) => digit.to_digit(10).unwrap_or(0) as u8,
                _ => {
                    return Err(Box::new(LatexError(
                        tokspan.span().into(),
                        LatexErrKind::InvalidParameterNumber,
                    )));
                }
            };
            let tokspan = self.next_token()?;
            if !matches!(tokspan.token(), Token::SquareBracketClose) {
                return Err(Box::new(LatexError(
                    tokspan.span().into(),
                    LatexErrKind::InvalidParameterNumber,
                )));
            }
            num_args
        } else {
            0
        };

        // The body. We must not consume the opening `{` here, because that would make the
        // token queue load the first token of the body while macro parameters are still
        // disallowed; `record_macro_body` takes care of the braces instead.
        let mut body_tokspans = Vec::new();
        if matches!(self.tokens.peek().token(), Token::GroupBegin) {
            self.tokens.record_macro_body(&mut body_tokspans)?;
            // `record_macro_body` leaves the closing `}` for us.
            self.next_token()?;
        } else {
            let queued = self.tokens.next_keeping_name()?;
            if matches!(queued.token(), Token::Eoi) {
                return Err(Box::new(LatexError(
                    queued.span().into(),
                    LatexErrKind::ExpectedArgumentGotEOI,
                )));
            }
            // A body which isn't a group consists of a single token.
            body_tokspans.push(queued);
        }

        let (body, first_class) = self.tokens.map_recorded_tokens(body_tokspans, num_args)?;

        let Some((name, replace)) = target else {
            // `\providecommand` for a command which already exists: the definition we just
            // parsed is discarded.
            return Ok(());
        };
        if !self
            .tokens
            .define(name, num_args, &body, first_class, replace, false)
        {
            // The name came from an unresolved command, so it cannot be in the store already:
            // the names which are defined are resolved when they are read, and the buffered
            // tokens are re-resolved whenever something is defined.
            return Err(Box::new(LatexError(
                name_tokspan.span().into(),
                LatexErrKind::Internal,
            )));
        }
        Ok(())
    }

    /// Parse `\def\a#1#2{...}`, TeX's way of defining a command.
    ///
    /// The body means the same thing as the body of a `\newcommand`: it is recorded by name,
    /// so it follows later redefinitions of the names in it. What differs is the syntax and
    /// that an existing meaning of the name is replaced without complaint, as it is in TeX.
    ///
    /// Delimited parameters are not supported, so the parameter text may only be a run of
    /// `#1`, `#2`, ... followed by the braced body. Whitespace within the parameter text is a
    /// delimiter in TeX, so it is rejected here rather than skipped.
    ///
    /// With `is_global` (`\gdef` or `\global\def`), the definition goes into the store which
    /// outlives the snippet, and whatever it refers to in the local store is copied there
    /// along with it.
    fn def_command(&mut self, is_global: bool) -> ParseResult<()> {
        let name = self.tokens.read_definition_name(self.arena)?;

        // Whitespace between the name and the parameter text is skipped.
        // Buf after this point, whitespace is significant. (It can also be used as delimiter).
        while matches!(self.tokens.peek_any_token().token(), Token::Whitespace) {
            self.tokens.next_any_token()?;
        }

        // The parameter text, which ends where the body begins.
        let mut num_args = 0u8;
        loop {
            let tokspan = self.tokens.peek_any_token();
            match *tokspan.token() {
                // The `{` must be left for `record_macro_body` below.
                Token::GroupBegin => break,
                Token::CustomCmdArgInput(arg_num) => {
                    if arg_num != num_args {
                        return Err(Box::new(LatexError(
                            tokspan.span().into(),
                            LatexErrKind::UnexpectedParameterNumber {
                                expected: num_args + 1,
                                actual: arg_num + 1,
                            },
                        )));
                    }
                    num_args += 1;
                    self.tokens.next_any_token()?;
                }
                Token::Eoi => {
                    return Err(Box::new(LatexError(
                        tokspan.span().into(),
                        LatexErrKind::ExpectedArgumentGotEOI,
                    )));
                }
                // Anything else would be a delimiter, whitespace included. We must not consume
                // it, because an unknown command here would be reported as such instead.
                _ => {
                    return Err(Box::new(LatexError(
                        tokspan.span().into(),
                        LatexErrKind::DelimitedParameters,
                    )));
                }
            }
        }

        let mut body_tokspans = Vec::new();
        self.tokens.record_macro_body(&mut body_tokspans)?;
        // `record_macro_body` leaves the closing `}` for us.
        self.next_token()?;
        let (body, first_class) = self.tokens.map_recorded_tokens(body_tokspans, num_args)?;

        // `\def` never complains about an existing meaning, so it always replaces; `define`
        // then returns `true` unconditionally.
        self.tokens
            .define(name, num_args, &body, first_class, true, is_global);
        Ok(())
    }

    /// Parse the `\global` prefix, which only `\let` and `\def` may follow.
    ///
    /// `\global` makes the definition after it outlive the snippet even when the conversion
    /// doesn't run in the global group; in the global group it changes nothing, because every
    /// definition outlives the snippet there anyway.
    fn global_prefix(&mut self) -> ParseResult<()> {
        loop {
            let tokspan = self.tokens.next()?;
            match tokspan.token() {
                Token::Let => return self.let_command(true),
                // `\global\gdef` is allowed, and means the same as `\gdef`.
                Token::Def(_) => return self.def_command(true),
                // `\global\global\let` is allowed, as it is in LaTeX.
                Token::Global => {}
                Token::Eoi => {
                    return Err(Box::new(LatexError(
                        tokspan.span().into(),
                        LatexErrKind::ExpectedArgumentGotEOI,
                    )));
                }
                _ => {
                    return Err(Box::new(LatexError(
                        tokspan.span().into(),
                        LatexErrKind::CannotBeUsedHere {
                            got: LimitedUsabilityToken::Global,
                            correct_place: Place::BeforeDefinition,
                        },
                    )));
                }
            }
        }
    }

    /// Parse `\let\a=\b`, which gives `\a` the meaning `\b` has right now.
    ///
    /// Unlike the body of a `\newcommand`, the meaning is *not* recorded by name: `\let`
    /// binds what the source means at this point and keeps it, which is what makes it useful
    /// for holding on to a definition one is about to replace.
    ///
    /// With `global`, the definition goes into the store which outlives the snippet, and
    /// whatever it refers to in the local store is copied there along with it.
    fn let_command(&mut self, is_global: bool) -> ParseResult<()> {
        let name = self.tokens.read_definition_name(self.arena)?;

        // The `=` between the two names is optional. We compare instead of matching, because
        // `peek` doesn't unwrap `Token::MathOrTextMode`, which is how `=` arrives from the
        // lexer, whereas it arrives bare from the body of a custom command.
        if matches!(
            self.tokens.peek().token().unwrap_math_ref(),
            Token::Relation(symbol::EQUALS_SIGN)
        ) {
            self.next_token()?;
        }

        let src = self.tokens.next()?;
        let (token, span) = src.into_parts();
        match token {
            Token::Eoi => {
                return Err(Box::new(LatexError(
                    span.into(),
                    LatexErrKind::ExpectedArgumentGotEOI,
                )));
            }
            // We require resolved commands here for `\let`.
            Token::UnresolvedCommand(name) => {
                let name = self.tokens.cmd_name(name);
                return Err(Box::new(LatexError(
                    span.into(),
                    LatexErrKind::UnknownCommand(KString::from_ref(name)),
                )));
            }
            _ => {}
        }
        let first_class = token.class();
        let body = [RecordedToken::Token(token)];
        // `\let` never complains about an existing meaning, so it always replaces; `define`
        // then returns `true` unconditionally.
        self.tokens
            .define(name, 0, &body, first_class, true, is_global);
        Ok(())
    }

    /// Account for one expansion of a custom command, and give up if there were too many.
    ///
    /// A command may expand to itself, directly or through other commands, in which case
    /// expanding it would never end. We don't try to recognize that; we just stop when a
    /// snippet has had more expansions than any reasonable one needs.
    fn count_expansion(&mut self, span: Span) -> ParseResult<()> {
        let Some(left) = self.state.expansions_left.checked_sub(1) else {
            return Err(Box::new(LatexError(
                span.into(),
                LatexErrKind::TooManyExpansions,
            )));
        };
        self.state.expansions_left = left;
        Ok(())
    }

    /// Read the arguments of a custom command, ready to be substituted into its body.
    ///
    /// The arguments are read as raw token streams; nothing in them is expanded here. They
    /// are only needed until the body has been queued, so they go straight into the one
    /// buffer, overwriting the arguments of whichever command was expanded before.
    fn read_cmd_args(&mut self, num_args: u8) -> ParseResult<()> {
        self.state.cmd_args.clear();
        for arg_num in 0..num_args {
            // The tokens keep the names they came from, so that an argument which is
            // substituted into a `\newcommand` can be recorded by name like any other token.
            let queued = self.tokens.next_keeping_name()?;
            match queued.token() {
                Token::GroupBegin => {
                    self.tokens
                        .record_group(self.state.cmd_args.buffer(), true)?;
                }
                Token::Eoi => {
                    return Err(Box::new(LatexError(
                        queued.span().into(),
                        LatexErrKind::ExpectedArgumentGotEOI,
                    )));
                }
                // An argument which isn't a group consists of a single token.
                _ => self.state.cmd_args.push(queued),
            }
            self.state.cmd_args.finish_arg(arg_num);
        }
        Ok(())
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
        core::debug_assert_matches!(
            first,
            Some((Token::Underscore | Token::Circumflex | Token::Prime(_), _)) | None
        );

        let mut ret: BoundsWithLimits = BoundsWithLimits::default();

        loop {
            // retrieve and consume next token
            let (next_token, next_span) = first
                .as_ref()
                .map(|(tok, span)| (tok, *span))
                .unwrap_or_else(|| {
                    let tok = self.tokens.peek();
                    (tok.token(), tok.span())
                });
            let next_token = next_token.unwrap_math_ref();
            if matches!(
                next_token,
                Token::Underscore | Token::Circumflex | Token::Prime(_) | Token::Limits(_)
            ) {
                let token = *next_token;
                // consume the token we're currently looking at
                if first.take().is_none() {
                    self.tokens.next()?;
                }
                // parse it into a bound
                let (bound_starter_kind, bound_to_replace) = match token {
                    Token::Circumflex => (BoundStarterKind::Circumflex, &mut ret.bounds.1),
                    Token::Prime(kind) => (BoundStarterKind::Prime(kind), &mut ret.bounds.1),
                    Token::Underscore => (BoundStarterKind::Underscore, &mut ret.bounds.0),
                    Token::Limits(kind) => {
                        ret.limits_span = Some((kind, next_span));
                        continue;
                    }
                    _ => unreachable!(),
                };

                let node = self.get_sub_or_sup(bound_starter_kind)?;
                if bound_to_replace.replace(node).is_some() {
                    return Err(Box::new(LatexError(
                        next_span.into(),
                        LatexErrKind::DuplicateSubOrSup,
                    )));
                }
            } else {
                // we are done!
                break Ok(ret);
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
        let first = self.tokens.peek();
        let first_span = first.span();
        match *first.token().unwrap_math_ref() {
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
                let next_tok = self.tokens.peek_any_token().token().unwrap_math_ref();

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
        let next_class = self.peek_class_token(parse_as.in_sequence())?;
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
                let unit_str = core::str::from_utf8(&unit[..i]).unwrap_or("");
                return Err(Box::new(LatexError(
                    start..peek_span.start(),
                    LatexErrKind::InvalidUnit(KString::from_ref(unit_str)),
                )));
            };
            unit[i] = unit_char as u8;
            let span = self.tokens.next()?.span();
            unit_start.get_or_insert(span.start());
            arg_end = span.end();
        }
        let unit = core::str::from_utf8(&unit).unwrap_or("");

        let unit_span = unit_start.unwrap_or(arg_end)..arg_end;
        let Ok(latex_unit) = LatexUnit::try_from(unit) else {
            return Err(Box::new(LatexError(
                unit_span,
                LatexErrKind::InvalidUnit(KString::from_ref(unit)),
            )));
        };
        let math_unit_expected = matches!(kind, UnitKind::MathUnits);
        if latex_unit.is_math_unit() != math_unit_expected {
            return Err(Box::new(LatexError(
                unit_span,
                LatexErrKind::IllegalUnit {
                    unit: KString::from_ref(unit),
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
        // The tokens come out of the queue, so any custom command among them has already
        // been expanded, arguments and all.
        for tokspan in tokens {
            let (tok, span) = tokspan.into_parts();
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

    fn peek_class_token(&mut self, in_sequence: bool) -> ParseResult<Class> {
        if !in_sequence {
            return Ok(Class::Default);
        }
        let (token_idx, class) = self.tokens.peek_class_token()?;
        if let Some(vld) = self.state.vertical_line_def
            && matches!(
                self.tokens
                    .get_token_by_index(token_idx)
                    .map(TokSpan::token),
                Some(Token::MathOrTextMode(
                    Token::Ord(symbol::VERTICAL_LINE),
                    '|'
                ))
            )
        {
            Ok(match vld {
                VerticalLineDef::RelSpacing => Class::Default,
                VerticalLineDef::OpSpacingStretchy | VerticalLineDef::RelSpacingStretchy => {
                    Class::Close
                }
            })
        } else {
            Ok(class)
        }
    }
}

impl ParserState<'_> {
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

fn extract_delimiter(tok: TokSpan, location: DelimiterModifier) -> ParseResult<StretchableOp> {
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

    use crate::global_state::GlobalState;

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
        let parser_cfg = crate::ParserConfig::default();
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let mut state = GlobalState::default();
            let l = Lexer::new(problem);
            let mut p = Parser::new(l, &arena, &parser_cfg, &mut state, Style::Text).unwrap();
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
        let parser_cfg = crate::ParserConfig::default();
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let mut state = GlobalState::default();
            let l = Lexer::new("");
            let mut p = Parser::new(l, &arena, &parser_cfg, &mut state, Style::Text).unwrap();
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
        let parser_cfg = crate::ParserConfig::default();
        let mut state = GlobalState::default();
        let l = Lexer::new(&input);
        let mut p = Parser::new(l, &arena, &parser_cfg, &mut state, Style::Text).unwrap();
        let parsed = p
            .parse_string_literal()
            .unwrap_or_else(|e| panic!("failed with error '{}'", e));
        assert_eq!(parsed.0, literal);
    }
}
