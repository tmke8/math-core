use std::mem;

use mathml_renderer::{
    arena::{Arena, Buffer, StringBuilder},
    ast::Node,
    attribute::{
        Align, FracAttr, MathSpacing, MathVariant, OpAttr, StretchMode, Style, TextTransform,
    },
    ops,
};

use crate::{
    commands::get_negated_op,
    error::{LatexErrKind, LatexError, Place},
    lexer::Lexer,
    token::{TokLoc, Token},
};

pub(crate) struct Parser<'arena, 'source> {
    l: Lexer<'source>,
    peek: TokLoc<'source>,
    buffer: Buffer,
    arena: &'arena Arena,
    collector: LetterCollector<'arena>,
    is_bold_italic: bool,
    is_after_colon: bool,
    is_after_relation: bool,
}
impl<'arena, 'source> Parser<'arena, 'source>
where
    'source: 'arena, // The reference to the source string will live as long as the arena.
{
    pub(crate) fn new(l: Lexer<'source>, arena: &'arena Arena) -> Self {
        let input_length = l.input_length;
        let mut p = Parser {
            l,
            peek: TokLoc(0, Token::EOF),
            buffer: Buffer::new(input_length),
            arena,
            collector: LetterCollector::Inactive,
            is_bold_italic: false,
            is_after_colon: false,
            is_after_relation: false,
        };
        // Discard the EOF token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    fn next_token(&mut self) -> TokLoc<'source> {
        if matches!(self.collector, LetterCollector::Collecting) {
            let first_loc = self.peek.location();
            let mut builder = self.buffer.get_builder();
            let mut num_chars = 0usize;
            let mut first_char: Option<char> = None;

            // Loop until we find a non-letter token.
            while let tok @ (Token::Letter(ch) | Token::UprightLetter(ch)) = self.peek.token() {
                // We stop collecting if we encounter an upright letter while in bold italic mode.
                // This is because the bold-italic transformation has to handle upright letters
                // differently, and this wouldn't be possible anymore if we merged these letters
                // here together with the non-upright letters.
                if matches!(tok, Token::UprightLetter(_)) && self.is_bold_italic {
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
        next_token(&mut self.peek, &mut self.l)
    }

    pub(crate) fn parse(&mut self) -> Result<&'arena [&'arena Node<'arena>], LatexError<'source>> {
        let nodes = self.parse_sequence(Token::EOF, true)?;
        Ok(self.arena.push_slice(&nodes))
    }

    /// Parse a sequence of tokens until the given end token is encountered.
    ///
    /// If `eof_as_end_token` is `false`, an error is returned if the input ends before the end
    /// token.
    ///
    /// Note that this function does not consume the end token. That's because the end token might
    /// be used by the calling function to emit another node.
    fn parse_sequence(
        &mut self,
        end_token: Token<'static>,
        eof_as_end_token: bool,
    ) -> Result<Vec<&'arena Node<'arena>>, LatexError<'source>> {
        let mut nodes = Vec::new();

        // Because we don't want to consume the end token, we just peek here.
        while !self.peek.token().is_same_kind_as(&end_token) {
            let cur_tokloc = self.next_token();
            if matches!(cur_tokloc.token(), Token::EOF) {
                if eof_as_end_token {
                    break;
                } else {
                    // When the input ends without the closing token.
                    return Err(LatexError(
                        cur_tokloc.location(),
                        LatexErrKind::UnclosedGroup(end_token),
                    ));
                }
            }
            // Parse the token.
            let target = self.parse_token(cur_tokloc, false)?;

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
    ///
    /// If `wants_arg` is `true`, the parser will not collect digits into a number node,
    /// and will not apply any fusing of nodes.
    fn parse_token(
        &mut self,
        cur_tokloc: TokLoc<'source>,
        wants_arg: bool,
    ) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        let TokLoc(loc, cur_token) = cur_tokloc;
        let is_after_colon = self.is_after_colon;
        self.is_after_colon = false;
        let is_after_relation = self.is_after_relation;
        self.is_after_relation = false;
        let node = match cur_token {
            Token::Number(number) => {
                let mut builder = self.buffer.get_builder();
                builder.push_char(number as u8 as char);
                if !wants_arg {
                    // Consume tokens as long as they are `Token::Number` or
                    // `Token::Letter(ops::FULL_STOP)` or `Token::Operator(ops::COMMA)`,
                    // but only if the token *after that* is a digit.
                    loop {
                        let ch = if let Token::Number(number) = self.peek.token() {
                            *number as u8 as char
                        } else {
                            let ch = match self.peek.token() {
                                Token::Letter(ops::FULL_STOP) => Some('.'),
                                Token::Relation(ops::COMMA) => Some(','),
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
            Token::Letter(x) => Node::SingleLetterIdent(x, false),
            Token::UprightLetter(x) => Node::SingleLetterIdent(x, true),
            Token::Relation(relation) => {
                self.is_after_relation = true;
                if is_after_colon && matches!(relation, ops::IDENTICAL_TO) {
                    Node::OperatorWithSpacing {
                        op: relation.into(),
                        left: Some(MathSpacing::Zero),
                        right: None,
                    }
                } else {
                    Node::Operator(relation.into(), None)
                }
            }
            Token::BinaryOp(binary_op) => Node::Operator(
                binary_op.into(),
                if is_after_relation {
                    Some(OpAttr::FormPrefix)
                } else {
                    None
                },
            ),
            Token::OpGreaterThan => Node::OpGreaterThan,
            Token::OpLessThan => Node::OpLessThan,
            Token::OpAmpersand => Node::OpAmpersand,
            Token::Function(fun) => Node::MultiLetterIdent(fun),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace | Token::Whitespace => Node::Text("\u{A0}"),
            Token::Sqrt => {
                let next = self.next_token();
                if matches!(next.token(), Token::SquareBracketOpen) {
                    let degree = self.parse_sequence(Token::SquareBracketClose, false)?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_next(true)?;
                    Node::Root(self.node_vec_to_node(degree, None), content)
                } else {
                    Node::Sqrt(self.parse_token(next, true)?)
                }
            }
            Token::Frac(attr) | Token::Binom(attr) => {
                let num = self.parse_next(true)?;
                let den = self.parse_next(true)?;
                if matches!(cur_token, Token::Binom(_)) {
                    let lt = Some('0');
                    Node::Fenced {
                        open: ops::LEFT_PARENTHESIS,
                        close: ops::RIGHT_PARENTHESIS,
                        content: self.commit(Node::Frac { num, den, lt, attr }),
                        style: None,
                    }
                } else {
                    let lt = None;
                    Node::Frac { num, den, lt, attr }
                }
            }
            Token::Genfrac => {
                // TODO: This should not just blindly try to parse a node.
                // Rather, we should explicitly attempt to parse a group (aka Row),
                // and if that doesn't work, we try to parse it as an Operator,
                // and if that still doesn't work, we return an error.
                let open = match self.parse_next(true)? {
                    Node::StretchableOp(op, _) => *op,
                    Node::Row { nodes: [], .. } => ops::NULL,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let close = match self.parse_next(true)? {
                    Node::StretchableOp(op, _) => *op,
                    Node::Row { nodes: [], .. } => ops::NULL,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                self.check_lbrace()?;
                // The default line thickness in LaTeX is 0.4pt.
                // TODO: Support other line thicknesses.
                // We could maybe store them as multiples of 0.4pt,
                // so that we can render them as percentages.
                let lt = match self.parse_text_group()?.trim() {
                    "" => None,
                    "0pt" => Some('0'),
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let style = match self.parse_next(true)? {
                    Node::Number(num) => match num.as_bytes() {
                        b"0" => Some(Style::DisplayStyle),
                        b"1" => Some(Style::TextStyle),
                        b"2" => Some(Style::ScriptStyle),
                        b"3" => Some(Style::ScriptScriptStyle),
                        _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                    },
                    Node::Row { nodes: [], .. } => None,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let num = self.parse_next(true)?;
                let den = self.parse_next(true)?;
                let attr = None;
                Node::Fenced {
                    open,
                    close,
                    content: self.commit(Node::Frac { num, den, lt, attr }),
                    style,
                }
            }
            Token::OverUnder(op, is_over, attr) => {
                let target = self.parse_next(true)?;
                if is_over {
                    Node::OverOp(op.into(), attr, target)
                } else {
                    Node::UnderOp(op.into(), target)
                }
            }
            Token::Overset | Token::Underset => {
                let symbol = self.parse_next(true)?;
                let target = self.parse_next(true)?;
                if matches!(cur_token, Token::Overset) {
                    Node::Overset { symbol, target }
                } else {
                    Node::Underset { symbol, target }
                }
            }
            Token::OverUnderBrace(x, is_over) => {
                let target = self.parse_next(true)?;
                let symbol = self.commit(Node::Operator(x.into(), None));
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
                    let expl = self.parse_next(true)?;
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
                    self.commit(Node::Operator(op.into(), Some(OpAttr::NoMovableLimits)))
                } else {
                    self.commit(Node::Operator(op.into(), None))
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
            Token::Lim(lim) => {
                if matches!(self.peek.token(), Token::Underscore) {
                    let target = self.commit(Node::MultiLetterIdent(lim));
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_next(true)?;
                    Node::Underset {
                        target,
                        symbol: under,
                    }
                } else {
                    Node::MultiLetterIdent(lim)
                }
            }
            Token::Slashed => {
                let node = self.parse_next(true)?;
                Node::Slashed(node)
            }
            Token::Not => {
                // `\not` has to be followed by something:
                match self.next_token().into_token() {
                    Token::Relation(op) => {
                        if let Some(negated) = get_negated_op(op) {
                            Node::Operator(negated.into(), None)
                        } else {
                            Node::Operator(op.into(), None)
                        }
                    }
                    Token::OpLessThan => Node::Operator(ops::NOT_LESS_THAN.into(), None),
                    Token::OpGreaterThan => Node::Operator(ops::NOT_GREATER_THAN.into(), None),
                    Token::Letter(char) | Token::UprightLetter(char) => {
                        let mut builder = self.buffer.get_builder();
                        builder.push_char(char);
                        builder.push_char('\u{338}');
                        Node::MultiLetterIdent(builder.finish(self.arena))
                    }
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::CannotBeUsedHere {
                                got: cur_token,
                                correct_place: Place::BeforeSomeOps,
                            },
                        ))
                    }
                }
            }
            Token::Transform(tf) => {
                let old_collector = mem::replace(&mut self.collector, LetterCollector::Collecting);
                let old_is_bold_italic = mem::replace(
                    &mut self.is_bold_italic,
                    matches!(tf, MathVariant::Transform(TextTransform::BoldItalic)),
                );
                let content = self.parse_next(true)?;
                self.collector = old_collector;
                self.is_bold_italic = old_is_bold_italic;
                Node::TextTransform { content, tf }
            }
            Token::Integral(int) => {
                if matches!(self.peek.token(), Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    let target = self.commit(Node::Operator(int.into(), None));
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
                    let target = self.commit(Node::Operator(int.into(), None));
                    match self.get_bounds()? {
                        Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                        Bounds(Some(symbol), None) => Node::Subscript { target, symbol },
                        Bounds(None, Some(symbol)) => Node::Superscript { target, symbol },
                        Bounds(None, None) => return Ok(target),
                    }
                }
            }
            Token::Colon => match &self.peek.token() {
                Token::Relation(ops::EQUALS_SIGN) if !wants_arg => {
                    self.next_token(); // Discard the equals_sign token.
                    Node::Operator(ops::COLON_EQUALS.into(), None)
                }
                Token::Relation(ops::IDENTICAL_TO) if !wants_arg => {
                    self.is_after_colon = true;
                    Node::OperatorWithSpacing {
                        op: ops::COLON.into(),
                        left: Some(MathSpacing::FourMu),
                        right: Some(MathSpacing::Zero),
                    }
                }
                _ => Node::OperatorWithSpacing {
                    op: ops::COLON.into(),
                    left: Some(MathSpacing::FourMu),
                    right: Some(MathSpacing::FourMu),
                },
            },
            Token::GroupBegin => {
                let content = self.parse_sequence(Token::GroupEnd, false)?;
                self.next_token(); // Discard the closing token.
                return Ok(self.node_vec_to_node(content, None));
            }
            Token::Delimiter(paren) => Node::StretchableOp(paren, StretchMode::NoStretch),
            Token::SquareBracketOpen => {
                Node::StretchableOp(ops::LEFT_SQUARE_BRACKET, StretchMode::NoStretch)
            }
            Token::SquareBracketClose => {
                Node::StretchableOp(ops::RIGHT_SQUARE_BRACKET, StretchMode::NoStretch)
            }
            Token::Left => {
                let TokLoc(loc, next_token) = self.next_token();
                let open_paren = match next_token {
                    Token::Delimiter(open) => open,
                    Token::SquareBracketOpen => ops::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => ops::RIGHT_SQUARE_BRACKET,
                    Token::Letter(ops::FULL_STOP) => ops::NULL,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::MissingParenthesis {
                                location: &Token::Left,
                                got: next_token,
                            },
                        ))
                    }
                };
                let content = self.parse_sequence(Token::Right, false)?;
                self.next_token(); // Discard the closing token.
                let TokLoc(loc, next_token) = self.next_token();
                let close_paren = match next_token {
                    Token::Delimiter(close) => close,
                    Token::SquareBracketOpen => ops::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => ops::RIGHT_SQUARE_BRACKET,
                    Token::Letter(ops::FULL_STOP) => ops::NULL,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::MissingParenthesis {
                                location: &Token::Right,
                                got: next_token,
                            },
                        ))
                    }
                };
                Node::Fenced {
                    open: open_paren,
                    close: close_paren,
                    content: self.node_vec_to_node(content, None),
                    style: None,
                }
            }
            Token::Middle => {
                let TokLoc(loc, next_token) = self.next_token();
                let op = match next_token {
                    Token::Delimiter(op) => op,
                    Token::SquareBracketOpen => ops::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => ops::RIGHT_SQUARE_BRACKET,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::UnexpectedToken {
                                expected: &Token::Delimiter(ops::NULL),
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
                    Token::SquareBracketOpen => ops::LEFT_SQUARE_BRACKET,
                    Token::SquareBracketClose => ops::RIGHT_SQUARE_BRACKET,
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::UnexpectedToken {
                                expected: &Token::Delimiter(ops::NULL),
                                got: next_token,
                            },
                        ));
                    }
                };
                Node::SizedParen(size, paren)
            }
            Token::Begin => {
                self.check_lbrace()?;
                // Read the environment name.
                let env_name = self.parse_text_group()?;
                let content = self.parse_sequence(Token::End, false)?;
                let content = self.arena.push_slice(&content);
                let end_token_loc = self.next_token().location();
                let node = match env_name {
                    "align" | "align*" | "aligned" => Node::Table {
                        content,
                        align: Align::Alternating,
                        attr: Some(FracAttr::DisplayStyleTrue),
                    },
                    "cases" => {
                        let align = Align::Left;
                        let content = self.commit(Node::Table {
                            content,
                            align,
                            attr: None,
                        });
                        Node::Fenced {
                            open: ops::LEFT_CURLY_BRACKET,
                            close: ops::NULL,
                            content,
                            style: None,
                        }
                    }
                    "matrix" => Node::Table {
                        content,
                        align: Align::Center,
                        attr: None,
                    },
                    matrix_variant
                    @ ("pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix") => {
                        let align = Align::Center;
                        let (open, close) = match matrix_variant {
                            "pmatrix" => (ops::LEFT_PARENTHESIS, ops::RIGHT_PARENTHESIS),
                            "bmatrix" => (ops::LEFT_SQUARE_BRACKET, ops::RIGHT_SQUARE_BRACKET),
                            "Bmatrix" => (ops::LEFT_CURLY_BRACKET, ops::RIGHT_CURLY_BRACKET),
                            "vmatrix" => (ops::VERTICAL_LINE, ops::VERTICAL_LINE),
                            "Vmatrix" => (ops::DOUBLE_VERTICAL_LINE, ops::DOUBLE_VERTICAL_LINE),
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
                            }),
                            style: None,
                        }
                    }
                    _ => {
                        return Err(LatexError(loc, LatexErrKind::UnknownEnvironment(env_name)));
                    }
                };
                self.check_lbrace()?;
                let end_name = self.parse_text_group()?;
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
                // TODO: Don't parse a node just to immediately destructure it.

                // Turn off collection mode.
                let old_collector = mem::replace(&mut self.collector, LetterCollector::Inactive);
                let node = self.parse_next(true)?;
                self.collector = old_collector;
                let mut builder = self.buffer.get_builder();
                if !extract_letters(&mut builder, node) {
                    return Err(LatexError(
                        loc,
                        LatexErrKind::ExpectedText("\\operatorname"),
                    ));
                }
                let letters = builder.finish(self.arena);
                if let Some(ch) = get_single_char(letters) {
                    Node::SingleLetterIdent(ch, true)
                } else {
                    Node::MultiLetterIdent(letters)
                }
            }
            Token::Text(transform) => {
                self.l.text_mode = true;
                let node = self.parse_next(true)?;
                let mut builder = self.buffer.get_builder();
                if !extract_letters(&mut builder, node) {
                    return Err(LatexError(loc, LatexErrKind::ExpectedText("\\text")));
                }
                let text = builder.finish(self.arena);
                self.l.text_mode = false;
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
            Token::Style(style) => {
                let content = self.parse_sequence(Token::GroupEnd, true)?;
                Node::Row {
                    nodes: self.arena.push_slice(&content),
                    style: Some(style),
                }
            }
            Token::UnknownCommand(name) => {
                return Err(LatexError(loc, LatexErrKind::UnknownCommand(name)));
            }
            // Token::Underscore | Token::Circumflex => {
            Token::Circumflex => {
                return Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::AfterOpOrIdent,
                    },
                ));
            }
            Token::Prime => {
                let target = self.commit(Node::Row {
                    nodes: &[],
                    style: None,
                });
                let symbol = self.commit(Node::Operator(ops::PRIME.into(), None));
                Node::Superscript { target, symbol }
            }
            Token::Underscore => {
                let sub = self.parse_next(true)?;
                let base = self.parse_next(false)?;
                Node::Multiscript { base, sub }
            }
            Token::Limits => {
                return Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::AfterBigOp,
                    },
                ))
            }
            Token::EOF => return Err(LatexError(loc, LatexErrKind::UnexpectedEOF)),
            Token::End | Token::Right | Token::GroupEnd => {
                return Err(LatexError(loc, LatexErrKind::UnexpectedClose(cur_token)))
            }
            Token::CustomCmd(num_args, predefined) => {
                let mut nodes = Vec::with_capacity(num_args);
                for _ in 0..num_args {
                    let token = self.next_token();
                    let node = self.parse_token(token, true)?;
                    nodes.push(node);
                }
                let args = self.arena.push_slice(&nodes);
                Node::CustomCmd { predefined, args }
            }
            Token::GetCollectedLetters => match self.collector {
                LetterCollector::FinishedOneLetter { collected_letter } => {
                    self.collector = LetterCollector::Collecting;
                    Node::SingleLetterIdent(collected_letter, false)
                }
                LetterCollector::FinishedManyLetters { collected_letters } => {
                    self.collector = LetterCollector::Collecting;
                    Node::CollectedLetters(collected_letters)
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
        };
        Ok(self.commit(node))
    }

    /// Same as `parse_token`, but also gets the next token.
    #[inline]
    fn parse_next(&mut self, wants_arg: bool) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        let token = self.next_token();
        self.parse_token(token, wants_arg)
    }

    /// Parse the contents of a group which can only contain text.
    fn parse_text_group(&mut self) -> Result<&'source str, LatexError<'source>> {
        let result = self.l.read_environment_name();
        // Discard the opening token (which is still stored as `peek`).
        let opening_loc = self.next_token().location();
        result.ok_or(LatexError(opening_loc, LatexErrKind::UnparsableEnvName))
    }

    fn check_lbrace(&mut self) -> Result<(), LatexError<'source>> {
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
        Ok(())
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
            Some(self.node_vec_to_node(primes, None))
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
        static PRIME_SELECTION: [ops::Rel; 4] = [
            ops::PRIME,
            ops::DOUBLE_PRIME,
            ops::TRIPLE_PRIME,
            ops::QUADRUPLE_PRIME,
        ];
        if prime_count > 0 {
            // If we have between 1 and 4 primes, we can use the predefined prime operators.
            if let Some(op) = PRIME_SELECTION.get(prime_count - 1) {
                primes.push(self.commit(Node::Operator(op.into(), None)));
            } else {
                for _ in 0..prime_count {
                    primes.push(self.commit(Node::Operator(ops::PRIME.into(), None)));
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
        let node = self.parse_token(next, true);

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

    // Turn a vector of nodes into a single node.
    //
    // This is done either by returning the single node if there is only one,
    // or by creating a row node if there are multiple nodes.
    fn node_vec_to_node(
        &self,
        list_builder: Vec<&'arena Node<'arena>>,
        style: Option<Style>,
    ) -> &'arena Node<'arena> {
        if list_builder.len() == 1 {
            // Safety: We checked that there is an element.
            unsafe { list_builder.into_iter().next().unwrap_unchecked() }
        } else {
            let nodes = self.arena.push_slice(&list_builder);
            self.commit(Node::Row { nodes, style })
        }
    }
}

#[inline]
fn next_token<'source>(peek: &mut TokLoc<'source>, lexer: &mut Lexer<'source>) -> TokLoc<'source> {
    let peek_token = lexer.next_token();
    // Return the previous peek token and store the new peek token.
    mem::replace(peek, peek_token)
}

struct Bounds<'arena>(Option<&'arena Node<'arena>>, Option<&'arena Node<'arena>>);

enum LetterCollector<'arena> {
    Inactive,
    Collecting,
    FinishedOneLetter { collected_letter: char },
    FinishedManyLetters { collected_letters: &'arena str },
}

/// Extract the text of all single-letter identifiers and operators in `node`.
/// This function cannot be a method, because we need to borrow arena immutably
/// but buffer mutably. This is not possible with a mutable self reference.
///
/// Returns false if no letters could be extracted.
fn extract_letters<'arena>(buffer: &mut StringBuilder, node: &'arena Node<'arena>) -> bool {
    match node {
        Node::SingleLetterIdent(c, _) => buffer.push_char(*c),
        Node::Row { nodes, .. } => {
            for node in nodes.iter() {
                if !extract_letters(buffer, node) {
                    return false;
                }
            }
        }
        Node::Number(n) => buffer.push_str(n),
        Node::StretchableOp(op, _) => {
            buffer.push_char((*op).into());
        }
        Node::Operator(op, _) | Node::OperatorWithSpacing { op, .. } => {
            buffer.push_char(op.into());
        }
        Node::Text(str_ref) => {
            buffer.push_str(str_ref);
        }
        _ => return false,
    }
    true
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
            ("overset_digits", r"\overset12"),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let l = Lexer::new(problem);
            let mut p = Parser::new(l, &arena);
            let ast = p.parse().expect("Parsing failed");
            assert_ron_snapshot!(name, &ast, problem);
        }
    }
}
