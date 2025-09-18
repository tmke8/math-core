use std::mem;

use crate::mathml_renderer::{
    arena::{Arena, Buffer, StringBuilder},
    ast::Node,
    attribute::{
        FracAttr, LetterAttr, MathSpacing, MathVariant, OpAttr, RowAttr, StretchMode, Style,
        TextTransform,
    },
    length::{Length, LengthUnit},
    symbol::{self, StretchableOp},
    table::Alignment,
};

use super::{
    character_class::Class,
    color_defs::get_color,
    commands::get_negated_op,
    environments::Env,
    error::{LatexErrKind, LatexError, Place},
    lexer::Lexer,
    specifications::{LatexUnit, parse_column_specification, parse_length_specification},
    token::{ErrorTup, TokResult, Token},
};

pub(crate) struct Parser<'arena, 'source> {
    tokens: TokenProducer<'arena, 'source>,
    cmd_args: Vec<Vec<TokResult<'arena, 'source>>>,
    buffer: Buffer,
    arena: &'arena Arena,
    collector: LetterCollector<'arena>,
    tf_differs_on_upright_letters: bool,
}

struct TokenProducer<'arena, 'source> {
    lexer: Lexer<'source, 'source>,
    peek: TokResult<'arena, 'source>,
    stack: Vec<TokResult<'arena, 'source>>,
}

impl<'arena, 'source> TokenProducer<'arena, 'source> {
    /// Get the next token from the lexer, replacing the current peek token.
    ///
    /// If there are tokens on the stack, pop the top token from the stack instead.
    fn next(&mut self, arena: &'arena Arena) -> TokResult<'arena, 'source> {
        let peek_token = if let Some(tok) = self.stack.pop() {
            tok
        } else {
            match self.lexer.next_token() {
                Ok((loc, tok)) => TokResult(loc, Ok(tok)),
                Err(e) => {
                    let err = arena.alloc(e.1);
                    TokResult(e.0, Err(err))
                }
            }
        };
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek, peek_token)
    }

    fn add_to_stack(&mut self, tokens: &[impl Into<TokResult<'arena, 'source>> + Copy]) {
        // Only do something if the token slice is non-empty.
        if let [head, tail @ ..] = tokens {
            // Replace the peek token with the first token of the token stream.
            let old_peek = mem::replace(&mut self.peek, (*head).into());
            // Put the old peek token onto the token stack.
            self.stack.push(old_peek);
            // Put the rest of the token stream onto the token stack in reverse order.
            for tok in tail.iter().rev() {
                self.stack.push((*tok).into());
            }
        }
    }

    #[inline]
    fn is_empty_stack(&self) -> bool {
        self.stack.is_empty()
    }
}

/// A struct for managing the state of the sequence parser.
#[derive(Debug, Default)]
struct SequenceState {
    class: Class,
    /// `true` if the boundaries of the sequence are real boundaries;
    /// this is not the case for style-only rows.
    real_boundaries: bool,
    /// `true` if we are inside an environment that allows columns (`&`).
    allow_columns: bool,
    /// `true` if we are within a group where the style is `\scriptstyle` or smaller
    script_style: bool,
}

#[derive(Debug)]
enum SequenceEnd {
    Token(Token<'static>),
    AnyEndToken,
}

impl SequenceEnd {
    #[inline]
    fn matches(&self, other: &Result<Token<'_>, &LatexErrKind<'_>>) -> bool {
        match self {
            SequenceEnd::Token(token) => other.as_ref().is_ok_and(|tok| token.is_same_kind_as(tok)),
            SequenceEnd::AnyEndToken => matches!(
                other,
                Ok(Token::Eof | Token::GroupEnd | Token::End(_) | Token::Right)
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

type ASTResult<'arena, 'source> = Result<&'arena Node<'arena>, ErrorTup<'arena, 'source>>;

impl<'arena, 'source> Parser<'arena, 'source>
where
    'source: 'arena, // The reference to the source string will live as long as the arena.
{
    pub(crate) fn new(lexer: Lexer<'source, 'source>, arena: &'arena Arena) -> Self {
        let input_length = lexer.input_length();
        let mut p = Parser {
            tokens: TokenProducer {
                lexer,
                peek: TokResult(0, Ok(Token::Eof)),
                stack: Vec::new(),
            },
            cmd_args: Vec::new(),
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

    fn collect_letters(&mut self) -> Option<TokResult<'arena, 'source>> {
        let first_loc = self.tokens.peek.location();
        let mut builder = self.buffer.get_builder();
        let mut num_chars = 0usize;
        // We store the first character separately, because if we only collect
        // one character, we need it as a `char` and not as a `String`.
        let mut first_char: Option<char> = None;

        // Loop until we find a non-letter token.
        while let Ok(tok @ (Token::Letter(ch) | Token::UprightLetter(ch))) =
            self.tokens.peek.token()
        {
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
            self.tokens.next(self.arena);
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
            return Some(TokResult(first_loc, Ok(Token::GetCollectedLetters)));
        }
        None
    }

    #[inline(never)]
    fn next_token(&mut self) -> TokResult<'arena, 'source> {
        self.tokens.next(self.arena)
    }

    #[inline]
    pub(crate) fn parse(&mut self) -> Result<Vec<&'arena Node<'arena>>, ErrorTup<'arena, 'source>> {
        let nodes = self.parse_sequence(SequenceEnd::Token(Token::Eof), None)?;
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
    ) -> Result<Vec<&'arena Node<'arena>>, ErrorTup<'arena, 'source>> {
        let mut nodes = Vec::new();
        let sequence_state = if let Some(seq_state) = sequence_state {
            seq_state
        } else {
            &mut SequenceState {
                class: Class::Open,
                real_boundaries: true,
                allow_columns: false,
                script_style: false,
            }
        };

        // Because we don't want to consume the end token, we just peek here.
        while !sequence_end.matches(self.tokens.peek.token()) {
            let cur_tokloc = if matches!(self.collector, LetterCollector::Collecting) {
                self.collect_letters()
            } else {
                None
            };
            let cur_tokloc = cur_tokloc.unwrap_or_else(|| self.next_token());
            if matches!(cur_tokloc.token(), Ok(Token::Eof)) {
                // When the input ends without the closing token.
                if let SequenceEnd::Token(end_token) = sequence_end {
                    return Err((
                        cur_tokloc.location(),
                        self.arena.alloc(LatexErrKind::UnclosedGroup(end_token)),
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
        cur_tokloc: TokResult<'arena, 'source>,
        parse_as: ParseAs,
        sequence_state: Option<&mut SequenceState>,
    ) -> ASTResult<'arena, 'source> {
        let (loc, cur_token) = cur_tokloc.with_error()?;
        let sequence_state = if let Some(seq_state) = sequence_state {
            seq_state
        } else {
            &mut Default::default()
        };
        let mut new_class: Class = Default::default();
        let next_class = self.next_class(parse_as, sequence_state);
        let node: Result<Node, LatexError> = match cur_token {
            Token::Number(number) => {
                let mut builder = self.buffer.get_builder();
                builder.push_char(number as u8 as char);
                if matches!(parse_as, ParseAs::Sequence) {
                    // Consume tokens as long as they are `Token::Number` or
                    // `Token::Letter('.')`,
                    // but the latter only if the token *after that* is a digit.
                    loop {
                        let ch = if let Ok(Token::Number(number)) = self.tokens.peek.token() {
                            *number as u8 as char
                        } else {
                            let ch = if matches!(self.tokens.peek.token(), Ok(Token::Letter('.'))) {
                                Some('.')
                            } else {
                                None
                            };
                            if let Some(ch) = ch {
                                if self.tokens.lexer.is_next_digit() {
                                    ch
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        };
                        builder.push_char(ch);
                        self.tokens.next(self.arena);
                    }
                }
                Ok(Node::Number(builder.finish(self.arena)))
            }
            Token::Letter(x) => Ok(Node::IdentifierChar(x, LetterAttr::Default)),
            Token::UprightLetter(x) => Ok(Node::IdentifierChar(x, LetterAttr::Upright)),
            Token::Relation(relation) => {
                new_class = Class::Relation;
                if let Some(op) = relation.as_stretchable_op() {
                    Ok(Node::StretchableOp(op, StretchMode::NoStretch))
                } else {
                    let (left, right) = relation_spacing(next_class, sequence_state);
                    Ok(Node::Operator {
                        op: relation.as_op(),
                        attr: None,
                        left,
                        right,
                    })
                }
            }
            Token::Punctuation(punc) => {
                new_class = Class::Punctuation;
                let right = if matches!(next_class, Class::Close) || sequence_state.script_style {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                Ok(Node::Operator {
                    op: punc.as_op(),
                    attr: None,
                    left: None,
                    right,
                })
            }
            Token::Ord(ord) => {
                if let Some(op) = ord.as_stretchable_op() {
                    // If the operator can stretch, we prevent that by rendering it
                    // as a normal identifier.
                    Ok(Node::IdentifierChar(op.into(), LetterAttr::Default))
                } else {
                    Ok(Node::Operator {
                        op: ord.as_op(),
                        attr: None,
                        left: None,
                        right: None,
                    })
                }
            }
            Token::BinaryOp(binary_op) => {
                new_class = Class::BinaryOp;
                let spacing = if matches!(
                    sequence_state.class,
                    Class::Relation
                        | Class::Punctuation
                        | Class::BinaryOp
                        | Class::Open
                        | Class::Operator
                ) || matches!(
                    next_class,
                    Class::Relation | Class::Punctuation | Class::Close
                ) || sequence_state.script_style
                {
                    Some(MathSpacing::Zero)
                } else {
                    None
                };
                Ok(Node::Operator {
                    op: binary_op.into(),
                    attr: None,
                    left: spacing,
                    right: spacing,
                })
            }
            Token::OpGreaterThan => {
                let (left, right) = relation_spacing(next_class, sequence_state);
                Ok(Node::PseudoOp {
                    name: "&gt;",
                    attr: None,
                    left,
                    right,
                })
            }
            Token::OpLessThan => {
                let (left, right) = relation_spacing(next_class, sequence_state);
                Ok(Node::PseudoOp {
                    name: "&lt;",
                    attr: None,
                    left,
                    right,
                })
            }
            Token::OpAmpersand => Ok(Node::PseudoOp {
                name: "&amp;",
                attr: None,
                left: None,
                right: None,
            }),
            Token::PseudoOperator(name) => {
                let (left, right) = self.big_operator_spacing(parse_as, sequence_state, true);
                new_class = Class::Operator;
                Ok(Node::PseudoOp {
                    attr: None,
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
                // Spaces pass through the sequence state.
                new_class = sequence_state.class;
                Ok(Node::Space(space))
            }
            Token::CustomSpace => {
                let (loc, length) = self.parse_ascii_text_group()?;
                match parse_length_specification(length.trim()) {
                    Some(space) => Ok(Node::Space(space)),
                    None => Err(LatexError(loc, LatexErrKind::ExpectedLength(length))),
                }
            }
            Token::NonBreakingSpace => Ok(Node::Text("\u{A0}")),
            Token::Sqrt => {
                let next = self.next_token();
                if matches!(next.token(), Ok(Token::SquareBracketOpen)) {
                    let degree =
                        self.parse_sequence(SequenceEnd::Token(Token::SquareBracketClose), None)?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_next(ParseAs::Arg)?;
                    Ok(Node::Root(
                        node_vec_to_node(self.arena, degree, true),
                        content,
                    ))
                } else {
                    Ok(Node::Sqrt(self.parse_token(next, ParseAs::Arg, None)?))
                }
            }
            Token::Frac(attr) | Token::Binom(attr) => {
                let num = self.parse_next(ParseAs::Arg)?;
                let denom = self.parse_next(ParseAs::Arg)?;
                if matches!(cur_token, Token::Binom(_)) {
                    let (lt_value, lt_unit) = Length::zero().into_parts();
                    Ok(Node::Fenced {
                        open: Some(symbol::LEFT_PARENTHESIS.as_op()),
                        close: Some(symbol::RIGHT_PARENTHESIS.as_op()),
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
                // TODO: This should not just blindly try to parse a node.
                // Rather, we should explicitly attempt to parse a group (aka Row),
                // and if that doesn't work, we try to parse it as an Operator,
                // and if that still doesn't work, we return an error.
                let open = match self.parse_next(ParseAs::Arg)? {
                    Node::StretchableOp(op, _) => Some(*op),
                    Node::Row { nodes: [], .. } => None,
                    _ => break 'genfrac Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let close = match self.parse_next(ParseAs::Arg)? {
                    Node::StretchableOp(op, _) => Some(*op),
                    Node::Row { nodes: [], .. } => None,
                    _ => break 'genfrac Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let (loc, length) = self.parse_ascii_text_group()?;
                let lt = match length.trim() {
                    "" => Length::none(),
                    decimal => parse_length_specification(decimal)
                        .ok_or((loc, self.arena.alloc(LatexErrKind::ExpectedLength(decimal))))?,
                };
                let style = match self.parse_next(ParseAs::Arg)? {
                    Node::Number(num) => match num.as_bytes() {
                        b"0" => Some(Style::Display),
                        b"1" => Some(Style::Text),
                        b"2" => Some(Style::Script),
                        b"3" => Some(Style::ScriptScript),
                        _ => {
                            break 'genfrac Err(LatexError(0, LatexErrKind::UnexpectedEOF));
                        }
                    },
                    Node::Row { nodes: [], .. } => None,
                    _ => break 'genfrac Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
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
            Token::OverUnder(op, is_over, attr) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                if is_over {
                    Ok(Node::OverOp(op.as_op(), attr, target))
                } else {
                    Ok(Node::UnderOp(op.as_op(), target))
                }
            }
            Token::Overset | Token::Underset => {
                let symbol = self.parse_next(ParseAs::Arg)?;
                let token = self.next_token();
                let target =
                    self.parse_token(token, ParseAs::ContinueSequence, Some(sequence_state))?;
                if matches!(cur_token, Token::Overset) {
                    Ok(Node::Overset { symbol, target })
                } else {
                    Ok(Node::Underset { symbol, target })
                }
            }
            Token::OverUnderBrace(x, is_over) => {
                let target = self.parse_next(ParseAs::ArgWithSpace)?;
                let symbol = self.commit(Node::Operator {
                    op: x.as_op(),
                    attr: None,
                    left: None,
                    right: None,
                });
                let base = if is_over {
                    Node::Overset { symbol, target }
                } else {
                    Node::Underset { symbol, target }
                };
                if (is_over && matches!(self.tokens.peek.token(), Ok(Token::Circumflex)))
                    || (!is_over && matches!(self.tokens.peek.token(), Ok(Token::Underscore)))
                {
                    let target = self.commit(base);
                    self.next_token(); // Discard the circumflex or underscore token.
                    let expl = self.parse_next(ParseAs::Arg)?;
                    if is_over {
                        Ok(Node::Overset {
                            symbol: expl,
                            target,
                        })
                    } else {
                        Ok(Node::Underset {
                            symbol: expl,
                            target,
                        })
                    }
                } else {
                    Ok(base)
                }
            }
            Token::BigOp(op) => {
                new_class = Class::Operator;
                let limits = matches!(self.tokens.peek.token(), Ok(Token::Limits));
                if limits {
                    self.next_token(); // Discard the limits token.
                };
                let (left, right) = self.big_operator_spacing(parse_as, sequence_state, false);
                let attr = if limits {
                    Some(OpAttr::NoMovableLimits)
                } else {
                    None
                };
                let target = self.commit(Node::Operator {
                    op: op.as_op(),
                    attr,
                    left,
                    right,
                });
                match self.get_bounds()? {
                    Bounds(Some(under), Some(over)) => Ok(Node::UnderOver {
                        target,
                        under,
                        over,
                    }),
                    Bounds(Some(symbol), None) => Ok(Node::Underset { target, symbol }),
                    Bounds(None, Some(symbol)) => Ok(Node::Overset { target, symbol }),
                    Bounds(None, None) => {
                        sequence_state.class = new_class;
                        return Ok(target);
                    }
                }
            }
            Token::PseudoOperatorLimits(name) => {
                let movablelimits = if matches!(self.tokens.peek.token(), Ok(Token::Limits)) {
                    self.next_token(); // Discard the limits token.
                    Some(OpAttr::NoMovableLimits)
                } else {
                    Some(OpAttr::ForceMovableLimits)
                };
                if matches!(self.tokens.peek.token(), Ok(Token::Underscore)) {
                    let target = self.commit(Node::PseudoOp {
                        name,
                        attr: movablelimits,
                        left: Some(MathSpacing::ThreeMu),
                        right: Some(MathSpacing::ThreeMu),
                    });
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_next(ParseAs::Arg)?;
                    Ok(Node::Underset {
                        target,
                        symbol: under,
                    })
                } else {
                    let (left, right) = self.big_operator_spacing(parse_as, sequence_state, true);
                    new_class = Class::Operator;
                    Ok(Node::PseudoOp {
                        attr: None,
                        left,
                        right,
                        name,
                    })
                }
            }
            Token::Slashed => {
                let node = self.parse_next(ParseAs::Arg)?;
                Ok(Node::Slashed(node))
            }
            Token::Not => {
                // `\not` has to be followed by something:
                match self.next_token().into_token() {
                    Ok(Token::Relation(op)) => {
                        if let Some(negated) = get_negated_op(op) {
                            Ok(Node::Operator {
                                op: negated.as_op(),
                                attr: None,
                                left: None,
                                right: None,
                            })
                        } else {
                            Ok(Node::Operator {
                                op: op.as_op(),
                                attr: None,
                                left: None,
                                right: None,
                            })
                        }
                    }
                    Ok(tok @ (Token::OpLessThan | Token::OpGreaterThan)) => Ok(Node::Operator {
                        op: if matches!(tok, Token::OpLessThan) {
                            symbol::NOT_LESS_THAN.as_op()
                        } else {
                            symbol::NOT_GREATER_THAN.as_op()
                        },
                        attr: None,
                        left: None,
                        right: None,
                    }),
                    Ok(Token::Letter(char) | Token::UprightLetter(char)) => {
                        let mut builder = self.buffer.get_builder();
                        builder.push_char(char);
                        builder.push_char('\u{338}');
                        Ok(Node::IdentifierStr(builder.finish(self.arena)))
                    }
                    _ => Err(LatexError(
                        loc,
                        LatexErrKind::CannotBeUsedHere {
                            got: cur_token,
                            correct_place: Place::BeforeSomeOps,
                        },
                    )),
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
                Ok(Node::TextTransform { content, tf })
            }
            Token::Integral(int) => {
                new_class = Class::Operator;
                let limits = matches!(self.tokens.peek.token(), Ok(Token::Limits));
                if limits {
                    self.next_token(); // Discard the limits token.
                };
                let bounds = self.get_bounds()?;
                let (left, right) = self.big_operator_spacing(parse_as, sequence_state, false);
                let target = self.commit(Node::Operator {
                    op: int.as_op(),
                    attr: None,
                    left,
                    right,
                });
                if limits {
                    match bounds {
                        Bounds(Some(under), Some(over)) => Ok(Node::UnderOver {
                            target,
                            under,
                            over,
                        }),
                        Bounds(Some(symbol), None) => Ok(Node::Underset { target, symbol }),
                        Bounds(None, Some(symbol)) => Ok(Node::Overset { target, symbol }),
                        Bounds(None, None) => {
                            sequence_state.class = new_class;
                            return Ok(target);
                        }
                    }
                } else {
                    match bounds {
                        Bounds(Some(sub), Some(sup)) => Ok(Node::SubSup { target, sub, sup }),
                        Bounds(Some(symbol), None) => Ok(Node::Subscript { target, symbol }),
                        Bounds(None, Some(symbol)) => Ok(Node::Superscript { target, symbol }),
                        Bounds(None, None) => {
                            sequence_state.class = new_class;
                            return Ok(target);
                        }
                    }
                }
            }
            Token::ForceRelation(op) => {
                // A colon is actually just a relation, but by default, MathML Core gives it
                // punctuation spacing (left: 0, right: 3mu), so we have to explicitly make it have
                // relation spacing (left: 5mu, right: 5mu).
                new_class = Class::Relation;
                let left = if !matches!(parse_as, ParseAs::Sequence) {
                    None // Don't add spacing if we are in an argument.
                } else if matches!(
                    sequence_state.class,
                    Class::Relation | Class::Open | Class::Punctuation
                ) {
                    Some(MathSpacing::Zero)
                } else {
                    Some(MathSpacing::FiveMu)
                };
                let right = if !matches!(parse_as, ParseAs::Sequence) {
                    None // Don't add spacing if we are in an argument.
                } else if matches!(
                    next_class,
                    Class::Relation | Class::Close | Class::Punctuation
                ) {
                    Some(MathSpacing::Zero)
                } else {
                    Some(MathSpacing::FiveMu)
                };
                Ok(Node::Operator {
                    op,
                    attr: None,
                    left,
                    right,
                })
            }
            Token::GroupBegin => {
                let content = if matches!(parse_as, ParseAs::ContinueSequence) {
                    self.parse_sequence(SequenceEnd::Token(Token::GroupEnd), Some(sequence_state))?
                } else {
                    let mut s = SequenceState {
                        class: Class::Open,
                        real_boundaries: true,
                        script_style: sequence_state.script_style,
                        ..Default::default()
                    };
                    self.parse_sequence(SequenceEnd::Token(Token::GroupEnd), Some(&mut s))?
                };
                self.next_token(); // Discard the closing token.
                sequence_state.class = Class::Default;
                return Ok(node_vec_to_node(
                    self.arena,
                    content,
                    matches!(parse_as, ParseAs::Arg),
                ));
            }
            ref tok @ (Token::Open(paren) | Token::Close(paren)) => {
                if matches!(tok, Token::Open(_)) {
                    new_class = Class::Open;
                }
                Ok(Node::StretchableOp(paren.as_op(), StretchMode::NoStretch))
            }
            Token::SquareBracketOpen => {
                new_class = Class::Open;
                Ok(Node::StretchableOp(
                    symbol::LEFT_SQUARE_BRACKET.as_op(),
                    StretchMode::NoStretch,
                ))
            }
            Token::SquareBracketClose => Ok(Node::StretchableOp(
                symbol::RIGHT_SQUARE_BRACKET.as_op(),
                StretchMode::NoStretch,
            )),
            Token::Left => {
                let tok_loc = self.next_token();
                let open_paren = if matches!(tok_loc.token(), Ok(Token::Letter('.'))) {
                    None
                } else {
                    Some(self.extract_delimiter(tok_loc)?.0)
                };
                let content = self.parse_sequence(SequenceEnd::Token(Token::Right), None)?;
                self.next_token(); // Discard the closing token.
                let tok_loc = self.next_token();
                let close_paren = if matches!(tok_loc.token(), Ok(Token::Letter('.'))) {
                    None
                } else {
                    Some(self.extract_delimiter(tok_loc)?.0)
                };
                Ok(Node::Fenced {
                    open: open_paren,
                    close: close_paren,
                    content: node_vec_to_node(self.arena, content, false),
                    style: None,
                })
            }
            Token::Middle => {
                let tok_loc = self.next_token();
                let op = self.extract_delimiter(tok_loc)?.0;
                Ok(Node::StretchableOp(op, StretchMode::Middle))
            }
            Token::Big(size, class) => {
                let tok_loc = self.next_token();
                let (paren, symbol_class) = self.extract_delimiter(tok_loc)?;
                new_class = class.unwrap_or(symbol_class);
                Ok(Node::SizedParen(size, paren))
            }
            Token::Begin(env) => 'begin_env: {
                let array_spec = if matches!(env, Env::Array | Env::Subarray) {
                    // Parse the array options.
                    let (loc, options) = self.parse_ascii_text_group()?;
                    let Some(spec) = parse_column_specification(options, self.arena) else {
                        break 'begin_env Err(LatexError(
                            loc,
                            LatexErrKind::ExpectedColSpec(options.trim()),
                        ));
                    };
                    Some(spec)
                } else {
                    None
                };
                let mut state = SequenceState {
                    class: Class::Open,
                    real_boundaries: true,
                    allow_columns: true,
                    script_style: matches!(env, Env::Subarray),
                };
                let content =
                    self.parse_sequence(SequenceEnd::Token(Token::End(env)), Some(&mut state))?;

                // TODO: Find a better way to extract the `env` of the `End` token.
                let (end_token_loc, end_token) = self.next_token().with_error()?;
                let Token::End(end_env) = end_token else {
                    // This should never happen because we specified the end token above.
                    break 'begin_env Err(LatexError(
                        end_token_loc,
                        LatexErrKind::UnexpectedToken {
                            expected: &Token::End(Env::Align),
                            got: end_token,
                        },
                    ));
                };

                let content = self.arena.push_slice(&content);
                let node = match env {
                    Env::Align | Env::AlignStar | Env::Aligned => Node::Table {
                        content,
                        align: Alignment::Alternating,
                        attr: Some(FracAttr::DisplayStyleTrue),
                        with_numbering: matches!(env, Env::Align),
                    },
                    Env::Cases => {
                        let align = Alignment::Cases;
                        let content = self.commit(Node::Table {
                            content,
                            align,
                            attr: None,
                            with_numbering: false,
                        });
                        Node::Fenced {
                            open: Some(symbol::LEFT_CURLY_BRACKET.as_op()),
                            close: None,
                            content,
                            style: None,
                        }
                    }
                    Env::Matrix => Node::Table {
                        content,
                        align: Alignment::Centered,
                        attr: None,
                        with_numbering: false,
                    },
                    array_variant @ (Env::Array | Env::Subarray) => {
                        // SAFETY: `array_spec` is guaranteed to be Some because we checked for
                        // "array" and "subarray" above.
                        // TODO: Refactor this to avoid using `unsafe`.
                        let mut spec = unsafe { array_spec.unwrap_unchecked() };
                        let style = if matches!(array_variant, Env::Subarray) {
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
                    matrix_variant @ (Env::PMatrix
                    | Env::BMatrix
                    | Env::Bmatrix
                    | Env::VMatrix
                    | Env::Vmatrix) => {
                        let align = Alignment::Centered;
                        let (open, close) = match matrix_variant {
                            Env::PMatrix => (
                                symbol::LEFT_PARENTHESIS.as_op(),
                                symbol::RIGHT_PARENTHESIS.as_op(),
                            ),
                            Env::BMatrix => (
                                symbol::LEFT_SQUARE_BRACKET.as_op(),
                                symbol::RIGHT_SQUARE_BRACKET.as_op(),
                            ),
                            Env::Bmatrix => (
                                symbol::LEFT_CURLY_BRACKET.as_op(),
                                symbol::RIGHT_CURLY_BRACKET.as_op(),
                            ),
                            Env::VMatrix => {
                                const LINE: StretchableOp =
                                    symbol::VERTICAL_LINE.as_stretchable_op().unwrap();
                                (LINE, LINE)
                            }
                            Env::Vmatrix => {
                                const DOUBLE_LINE: StretchableOp =
                                    symbol::DOUBLE_VERTICAL_LINE.as_stretchable_op().unwrap();
                                (DOUBLE_LINE, DOUBLE_LINE)
                            }
                            // SAFETY: `matrix_variant` is one of the strings above.
                            _ => unsafe { std::hint::unreachable_unchecked() },
                        };
                        let attr = None;
                        Node::Fenced {
                            open: Some(open),
                            close: Some(close),
                            content: self.commit(Node::Table {
                                content,
                                align,
                                attr,
                                with_numbering: false,
                            }),
                            style: None,
                        }
                    }
                };
                if end_env != env {
                    Err(LatexError(
                        end_token_loc,
                        LatexErrKind::MismatchedEnvironment {
                            expected: env,
                            got: end_env,
                        },
                    ))
                } else {
                    Ok(node)
                }
            }
            Token::OperatorName => {
                let tokloc = self.tokens.peek;
                let mut builder = self.buffer.get_builder();
                let mut text_parser =
                    TextModeParser::new(&mut builder, &mut self.tokens, self.arena);
                text_parser.parse_token_in_text_mode(tokloc)?;
                let letters = builder.finish(self.arena);
                // Discard the last token.
                self.next_token();
                if let Some(ch) = get_single_char(letters) {
                    Ok(Node::IdentifierChar(ch, LetterAttr::Upright))
                } else {
                    let (left, right) = self.big_operator_spacing(parse_as, sequence_state, true);
                    new_class = Class::Operator;
                    Ok(Node::PseudoOp {
                        attr: None,
                        left,
                        right,
                        name: letters,
                    })
                }
            }
            Token::Text(transform) => {
                // Discard any whitespace that immediately follows the `Text` token.
                if matches!(self.tokens.peek.token(), Ok(Token::Whitespace)) {
                    self.next_token();
                }
                // Copy the token out of the peek variable.
                // We do this because we need to turn off text mode while there is still a peek
                // token that is consumed by the `Text` command.
                let tokloc = self.tokens.peek;
                let mut builder = self.buffer.get_builder();
                let mut text_parser =
                    TextModeParser::new(&mut builder, &mut self.tokens, self.arena);
                text_parser.parse_token_in_text_mode(tokloc)?;
                let text = builder.finish(self.arena);
                // Now turn off text mode.
                self.tokens.lexer.turn_off_text_mode();
                // Discard the last token that we already processed but we kept it in `peek`,
                // so that we can turn off text mode before new tokens are read.
                self.next_token();
                // Discard any whitespace tokens that are still stored in self.tokens.peek.
                if matches!(self.tokens.peek.token(), Ok(Token::Whitespace)) {
                    self.next_token();
                }
                if let Some(transform) = transform {
                    Ok(Node::TextTransform {
                        content: self.commit(Node::Text(text)),
                        tf: MathVariant::Transform(transform),
                    })
                } else {
                    Ok(Node::Text(text))
                }
            }
            Token::Ampersand => {
                if sequence_state.allow_columns {
                    new_class = Class::Close;
                    Ok(Node::ColumnSeparator)
                } else {
                    Err(LatexError(
                        loc,
                        LatexErrKind::CannotBeUsedHere {
                            got: cur_token,
                            correct_place: Place::TableEnv,
                        },
                    ))
                }
            }
            Token::NewLine => Ok(Node::RowSeparator),
            Token::Color => {
                let (loc, color_name) = self.parse_ascii_text_group()?;
                if let Some(color) = get_color(color_name) {
                    let content =
                        self.parse_sequence(SequenceEnd::AnyEndToken, Some(sequence_state))?;
                    Ok(Node::Row {
                        nodes: self.arena.push_slice(&content),
                        attr: color,
                    })
                } else {
                    Err(LatexError(loc, LatexErrKind::UnknownColor(color_name)))
                }
            }
            Token::Style(style) => {
                let old_style = mem::replace(
                    &mut sequence_state.script_style,
                    matches!(style, Style::Script | Style::ScriptScript),
                );
                let content =
                    self.parse_sequence(SequenceEnd::AnyEndToken, Some(sequence_state))?;
                sequence_state.script_style = old_style;
                Ok(Node::Row {
                    nodes: self.arena.push_slice(&content),
                    attr: RowAttr::Style(style),
                })
            }
            Token::Prime => {
                let target = self.commit(Node::Row {
                    nodes: &[],
                    attr: RowAttr::None,
                });
                let symbol = self.commit(Node::Operator {
                    op: symbol::PRIME.as_op(),
                    attr: None,
                    left: None,
                    right: None,
                });
                Ok(Node::Superscript { target, symbol })
            }
            tok @ (Token::Underscore | Token::Circumflex) => {
                let symbol = self.parse_next(ParseAs::Arg)?;
                if !matches!(
                    self.tokens.peek.token(),
                    Ok(Token::Eof | Token::GroupEnd | Token::End(_))
                ) {
                    let base = self.parse_next(ParseAs::Sequence)?;
                    let (sub, sup) = if matches!(tok, Token::Underscore) {
                        (Some(symbol), None)
                    } else {
                        (None, Some(symbol))
                    };
                    Ok(Node::Multiscript { base, sub, sup })
                } else {
                    let empty_row = self.commit(Node::Row {
                        nodes: &[],
                        attr: RowAttr::None,
                    });
                    if matches!(tok, Token::Underscore) {
                        Ok(Node::Subscript {
                            target: empty_row,
                            symbol,
                        })
                    } else {
                        Ok(Node::Superscript {
                            target: empty_row,
                            symbol,
                        })
                    }
                }
            }
            Token::Limits => Err(LatexError(
                loc,
                LatexErrKind::CannotBeUsedHere {
                    got: cur_token,
                    correct_place: Place::AfterBigOp,
                },
            )),
            Token::Eof => Err(LatexError(loc, LatexErrKind::UnexpectedEOF)),
            Token::End(_) | Token::Right | Token::GroupEnd => {
                Err(LatexError(loc, LatexErrKind::UnexpectedClose(cur_token)))
            }
            Token::CustomCmd(num_args, token_stream) => {
                if num_args > 0 {
                    // The fact that we only clear for `num_args > 0` is a hack to
                    // allow zero-argument token streams to be used within
                    // non-zero-argument token streams.
                    self.cmd_args.clear();
                }
                for _ in 0..num_args {
                    let tokens = if matches!(self.tokens.peek.token(), Ok(Token::GroupBegin)) {
                        let tokens = self
                            .tokens
                            .lexer
                            .read_group()
                            .map_err(|e| (e.0, self.arena.alloc(e.1)))?;
                        self.next_token(); // Discard the opening `{` token.
                        tokens
                    } else {
                        vec![self.next_token()]
                    };
                    self.cmd_args.push(tokens);
                }
                self.tokens.add_to_stack(token_stream);
                let token = self.next_token();
                return self.parse_token(token, parse_as, Some(sequence_state));
            }
            Token::CustomCmdArg(arg_num) => {
                if let Some(arg) = self.cmd_args.get(arg_num as usize) {
                    self.tokens.add_to_stack(&arg[..]);
                    let token = self.next_token();
                    return self.parse_token(token, parse_as, Some(sequence_state));
                } else {
                    Err(LatexError(loc, LatexErrKind::RenderError))
                }
            }
            Token::GetCollectedLetters => match self.collector {
                LetterCollector::FinishedOneLetter { collected_letter } => {
                    self.collector = LetterCollector::Collecting;
                    Ok(Node::IdentifierChar(collected_letter, LetterAttr::Default))
                }
                LetterCollector::FinishedManyLetters { collected_letters } => {
                    self.collector = LetterCollector::Collecting;
                    Ok(Node::IdentifierStr(collected_letters))
                }
                _ => Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::AfterOpOrIdent,
                    },
                )),
            },
            Token::HardcodedMathML(mathml) => Ok(Node::HardcodedMathML(mathml)),
            // The following are text-mode-only tokens.
            Token::Whitespace | Token::TextModeAccent(_) => {
                Err(LatexError(
                    loc,
                    // TODO: Find a better error.
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::BeforeSomeOps,
                    },
                ))
            }
        };
        sequence_state.class = new_class;
        match node {
            Ok(n) => Ok(self.commit(n)),
            Err(LatexError(loc, e)) => Err((loc, self.arena.alloc(e))),
        }
    }

    /// Same as `parse_token`, but also gets the next token.
    #[inline]
    fn parse_next(&mut self, parse_as: ParseAs) -> ASTResult<'arena, 'source> {
        let token = self.next_token();
        self.parse_token(token, parse_as, None)
    }

    /// Parse the contents of a group, `{...}`, which may only contain ASCII text.
    fn parse_ascii_text_group(
        &mut self,
    ) -> Result<(usize, &'source str), ErrorTup<'arena, 'source>> {
        if !self.tokens.is_empty_stack() {
            // This function doesn't work if we are processing tokens from the token stack.
            return Err((0, self.arena.alloc(LatexErrKind::NotSupportedInCustomCmd)));
        }
        // First check whether there is an opening `{` token.
        if !matches!(self.tokens.peek.token(), Ok(Token::GroupBegin)) {
            let (loc, token) = self.next_token().with_error()?;
            return Err((
                loc,
                self.arena.alloc(LatexErrKind::UnexpectedToken {
                    expected: &Token::GroupBegin,
                    got: token,
                }),
            ));
        }
        // Read the text.
        let result = self.tokens.lexer.read_ascii_text_group();
        // Discard the opening `{` token (which is still stored as `peek`).
        let opening_loc = self.next_token().location();
        result
            .map(|r| (opening_loc, r))
            .ok_or((opening_loc, self.arena.alloc(LatexErrKind::DisallowedChars)))
    }

    /// Parse the bounds of an integral, sum, or product.
    /// These bounds are preceeded by `_` or `^`.
    fn get_bounds(&mut self) -> Result<Bounds<'arena>, ErrorTup<'arena, 'source>> {
        let mut primes = self.prime_check();
        // Check whether the first bound is specified and is a lower bound.
        let first_underscore = matches!(self.tokens.peek.token(), Ok(Token::Underscore));
        let first_circumflex = matches!(self.tokens.peek.token(), Ok(Token::Circumflex));

        let (sub, sup) = if first_underscore || first_circumflex {
            let first_bound = Some(self.get_sub_or_sup(first_circumflex)?);

            // If the first bound was a subscript *and* we didn't encounter primes yet,
            // we check once more for primes.
            if first_underscore && primes.is_empty() {
                primes = self.prime_check();
            }

            // Check whether both an upper and a lower bound were specified.
            let second_underscore = matches!(self.tokens.peek.token(), Ok(Token::Underscore));
            let second_circumflex = matches!(self.tokens.peek.token(), Ok(Token::Circumflex));

            if (first_circumflex && second_circumflex) || (first_underscore && second_underscore) {
                let (loc, token) = self.next_token().with_error()?;
                return Err((
                    loc,
                    self.arena.alloc(LatexErrKind::CannotBeUsedHere {
                        got: token,
                        correct_place: Place::AfterOpOrIdent,
                    }),
                ));
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
        while matches!(self.tokens.peek.token(), Ok(Token::Prime)) {
            self.next_token(); // Discard the prime token.
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
                    attr: None,
                    left: None,
                    right: None,
                }));
            } else {
                for _ in 0..prime_count {
                    primes.push(self.commit(Node::Operator {
                        op: symbol::PRIME.as_op(),
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
    fn get_sub_or_sup(
        &mut self,
        is_sup: bool,
    ) -> Result<&'arena Node<'arena>, ErrorTup<'arena, 'source>> {
        self.next_token(); // Discard the underscore or circumflex token.
        let next = self.next_token();
        if matches!(
            next.token(),
            Ok(Token::Underscore | Token::Circumflex | Token::Prime)
        ) {
            return Err((
                next.location(),
                self.arena.alloc(LatexErrKind::CannotBeUsedHere {
                    got: next.with_error()?.1,
                    correct_place: Place::AfterOpOrIdent,
                }),
            ));
        }
        let mut sequence_state = SequenceState {
            script_style: true,
            ..Default::default()
        };
        let node = self.parse_token(next, ParseAs::Arg, Some(&mut sequence_state));

        // If the bound was a superscript, it may *not* be followed by a prime.
        if is_sup && matches!(self.tokens.peek.token(), Ok(Token::Prime)) {
            return Err((
                self.tokens.peek.location(),
                self.arena.alloc(LatexErrKind::CannotBeUsedHere {
                    got: Token::Prime,
                    correct_place: Place::AfterOpOrIdent,
                }),
            ));
        }

        node
    }

    fn big_operator_spacing(
        &self,
        parse_as: ParseAs,
        sequence_state: &SequenceState,
        explicit: bool,
    ) -> (Option<MathSpacing>, Option<MathSpacing>) {
        // We re-determine the next class here, because the next token may have changed
        // because we discarded bounds or limits tokens.
        let next_class = self.next_class(parse_as, sequence_state);
        (
            if matches!(
                sequence_state.class,
                Class::Relation | Class::Punctuation | Class::Operator | Class::Open
            ) {
                Some(MathSpacing::Zero)
            } else if explicit {
                Some(MathSpacing::ThreeMu)
            } else {
                None
            },
            if matches!(
                next_class,
                Class::Punctuation | Class::Relation | Class::Open | Class::Close
            ) {
                Some(MathSpacing::Zero)
            } else if explicit {
                Some(MathSpacing::ThreeMu)
            } else {
                None
            },
        )
    }

    fn next_class(&self, parse_as: ParseAs, sequence_state: &SequenceState) -> Class {
        if !matches!(parse_as, ParseAs::Sequence | ParseAs::ContinueSequence) {
            return Class::Default;
        }
        let Ok(tok) = self.tokens.peek.token() else {
            return Class::Default;
        };
        match tok {
            Token::Relation(_) | Token::ForceRelation(_) => Class::Relation,
            Token::Punctuation(_) => Class::Punctuation,
            Token::Open(_) | Token::Left | Token::SquareBracketOpen => Class::Open,
            Token::Close(_) | Token::SquareBracketClose | Token::Ampersand => Class::Close,
            Token::BinaryOp(_) => Class::BinaryOp,
            Token::BigOp(_) | Token::Integral(_) => Class::Operator,
            Token::End(_) | Token::Right | Token::GroupEnd | Token::Eof
                if sequence_state.real_boundaries =>
            {
                Class::Close
            }
            Token::Big(_, None) => Class::Default,
            Token::Big(_, Some(class)) => *class,
            _ => Class::Default,
        }
    }

    fn extract_delimiter(
        &self,
        tok: TokResult<'arena, 'source>,
    ) -> Result<(StretchableOp, Class), ErrorTup<'arena, 'source>> {
        let (loc, tok) = tok.with_error()?;
        let (delim, class) = match tok {
            Token::Open(paren) => (Some(paren.as_op()), Class::Open),
            Token::Close(paren) => (Some(paren.as_op()), Class::Close),
            Token::Ord(ord) => (ord.as_stretchable_op(), Class::Default),
            Token::Relation(rel) => (rel.as_stretchable_op(), Class::Relation),
            Token::SquareBracketOpen => (Some(symbol::LEFT_SQUARE_BRACKET.as_op()), Class::Open),
            Token::SquareBracketClose => (Some(symbol::RIGHT_SQUARE_BRACKET.as_op()), Class::Close),
            _ => (None, Class::Default),
        };
        let Some(delim) = delim else {
            return Err((
                loc,
                self.arena.alloc(LatexErrKind::UnexpectedToken {
                    expected: &Token::Open(symbol::LEFT_PARENTHESIS),
                    got: tok,
                }),
            ));
        };
        Ok((delim, class))
    }
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
    if let [single] = &nodes[..] {
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

fn relation_spacing(
    next_class: Class,
    sequence_state: &SequenceState,
) -> (Option<MathSpacing>, Option<MathSpacing>) {
    (
        if matches!(
            sequence_state.class,
            Class::Relation | Class::Open | Class::Punctuation
        ) || sequence_state.script_style
        {
            Some(MathSpacing::Zero)
        } else {
            None
        },
        if matches!(
            next_class,
            Class::Relation | Class::Punctuation | Class::Close
        ) || sequence_state.script_style
        {
            Some(MathSpacing::Zero)
        } else {
            None
        },
    )
}

struct Bounds<'arena>(Option<&'arena Node<'arena>>, Option<&'arena Node<'arena>>);

enum LetterCollector<'arena> {
    Inactive,
    Collecting,
    FinishedOneLetter { collected_letter: char },
    FinishedManyLetters { collected_letters: &'arena str },
}

struct TextModeParser<'arena, 'builder, 'source, 'parser> {
    builder: &'builder mut StringBuilder<'parser>,
    tokens: &'parser mut TokenProducer<'arena, 'source>,
    arena: &'arena Arena,
    tf: Option<TextTransform>,
}

impl<'arena, 'builder, 'source, 'parser> TextModeParser<'arena, 'builder, 'source, 'parser> {
    fn new(
        builder: &'builder mut StringBuilder<'parser>,
        tokens: &'parser mut TokenProducer<'arena, 'source>,
        arena: &'arena Arena,
    ) -> Self {
        Self {
            builder,
            tokens,
            arena,
            tf: None,
        }
    }

    fn next_token(&mut self) -> TokResult<'arena, 'source> {
        self.tokens.next(self.arena)
    }

    /// Parse the given token in text mode.
    ///
    /// This function may read in more tokens from the lexer, but it will always leave the last
    /// processed token in `peek`. This is important for turning of the text mode in the lexer at
    /// the right time.
    fn parse_token_in_text_mode(
        &mut self,
        tokloc: TokResult<'arena, 'source>,
    ) -> Result<(), ErrorTup<'arena, 'source>> {
        let (loc, token) = tokloc.with_error()?;
        let c: Result<char, LatexErrKind> = match token {
            Token::Letter(c) | Token::UprightLetter(c) => Ok(c),
            Token::Whitespace | Token::NonBreakingSpace => Ok('\u{A0}'),
            Token::Open(op) | Token::Close(op) => Ok(op.as_op().into()),
            Token::BinaryOp(op) => Ok(op.as_op().into()),
            Token::Relation(op) => Ok(op.as_op().into()),
            Token::SquareBracketOpen => Ok(symbol::LEFT_SQUARE_BRACKET.as_op().into()),
            Token::SquareBracketClose => Ok(symbol::RIGHT_SQUARE_BRACKET.as_op().into()),
            Token::Number(digit) => Ok(digit as u8 as char),
            Token::Prime => Ok('’'),
            Token::ForceRelation(op) => Ok(op.as_char()),
            Token::Punctuation(op) => Ok(op.as_op().into()),
            Token::PseudoOperator(name) | Token::PseudoOperatorLimits(name) => {
                // We don't transform these strings.
                self.builder.push_str(name);
                return Ok(());
            }
            Token::Space(length) => {
                if length == Length::new(1.0, LengthUnit::Em) {
                    Ok('\u{2003}')
                } else if length == LatexUnit::Mu.length_with_unit(5.0) {
                    Ok('\u{2004}')
                } else if length == LatexUnit::Mu.length_with_unit(4.0) {
                    Ok('\u{205F}')
                } else if length == LatexUnit::Mu.length_with_unit(3.0) {
                    Ok('\u{2009}')
                } else {
                    return Ok(());
                }
            }
            Token::TextModeAccent(accent) => {
                // Discard `TextModeAccent` token.
                self.next_token();
                let tokloc = self.tokens.peek;
                self.parse_token_in_text_mode(tokloc)?;
                self.builder.push_char(accent);
                return Ok(());
            }
            Token::Text(tf) => {
                // Discard `Text` token.
                self.next_token();
                let old_tf = mem::replace(&mut self.tf, tf);
                let tokloc = self.tokens.peek;
                self.parse_token_in_text_mode(tokloc)?;
                self.tf = old_tf;
                return Ok(());
            }
            Token::GroupBegin => {
                // Discard opening token.
                self.next_token();
                while !matches!(self.tokens.peek.token(), Ok(Token::GroupEnd)) {
                    let tokloc = self.tokens.peek;
                    self.parse_token_in_text_mode(tokloc)?;
                    // Discard the last token.
                    // We must do this here, because `parse_token_in_text_mode` always leaves the
                    // last token in `peek`, but we want to continue here, so we need to discard it.
                    self.next_token();
                }
                return Ok(());
            }
            Token::Eof => Err(LatexErrKind::UnclosedGroup(Token::GroupEnd)),
            Token::End(_) | Token::Right | Token::GroupEnd => {
                Err(LatexErrKind::UnexpectedClose(token))
            }
            _ => Err(LatexErrKind::NotValidInTextMode(token)),
        };
        match c {
            Err(e) => Err((loc, self.arena.alloc(e))),
            Ok(c) => {
                self.builder
                    .push_char(self.tf.map(|tf| tf.transform(c, false)).unwrap_or(c));
                Ok(())
            }
        }
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
            ("sout", r"\sout{abc}"),
            ("sum_relation", r"{\sum = 4}"),
            ("int_relation", r"{\int = 4}"),
            ("int_bounds_relation", r"{\int_0^\infty = 4}"),
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
