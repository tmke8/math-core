use std::mem;

use crate::{
    arena::{Arena, Buffer, NodeList, NodeListBuilder, NodeRef, SingletonOrList, StringBuilder},
    ast::Node,
    attribute::{
        Accent, Align, FracAttr, MathSpacing, MathVariant, OpAttr, ParenAttr, Style, TextTransform,
    },
    commands::get_negated_op,
    error::{LatexErrKind, LatexError, Place},
    lexer::Lexer,
    ops,
    token::{TokLoc, Token},
};

pub(crate) struct Parser<'arena, 'source> {
    l: Lexer<'source>,
    peek: TokLoc<'source>,
    buffer: Buffer,
    arena: &'arena Arena,
    tf: Option<TextTransform>,
    var: Option<MathVariant>,
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
            tf: None,
            var: None,
        };
        // Discard the EOF token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    fn next_token(&mut self) -> TokLoc<'source> {
        let peek_token = self.l.next_token(self.peek.token().acts_on_a_digit());
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek, peek_token)
    }

    pub(crate) fn parse(&mut self) -> Result<Node<'arena>, LatexError<'source>> {
        let mut list_builder = NodeListBuilder::new();

        loop {
            let cur_tokloc = self.next_token();
            if matches!(cur_tokloc.token(), Token::EOF) {
                break;
            }
            let node = self.parse_node(cur_tokloc)?;
            list_builder.push(node);
        }

        Ok(Node::PseudoRow(list_builder.finish()))
    }

    fn parse_node(
        &mut self,
        cur_tokloc: TokLoc<'source>,
    ) -> Result<NodeRef<'arena>, LatexError<'source>> {
        let target = self.parse_single_node(cur_tokloc)?;

        match self.get_bounds()? {
            Bounds(Some(sub), Some(sup)) => Ok(self.commit(Node::SubSup {
                target: target.node(),
                sub,
                sup,
            })),
            Bounds(Some(symbol), None) => Ok(self.commit(Node::Subscript {
                target: target.node(),
                symbol,
            })),
            Bounds(None, Some(symbol)) => Ok(self.commit(Node::Superscript {
                target: target.node(),
                symbol,
            })),
            Bounds(None, None) => Ok(target),
        }
    }

    /// Put the node onto the heap in the arena and return a reference to it.
    ///
    /// The advantage over using `Box` is that we can store the nodes in a contiguous
    /// memory block, and release all of them at once when the arena is dropped.
    ///
    /// Ideally, the node is constructed directly on the heap, so try to avoid
    /// constructing it on the stack and then moving it to the heap.
    fn commit(&self, node: Node<'arena>) -> NodeRef<'arena> {
        self.arena.push(node)
    }

    /// Read the node immediately after without worrying about whether
    /// the infix operator `_`, `^`, `'` will continue
    ///
    /// Use this function only if you are sure that this is what you need.
    /// Otherwise, use `parse_node` instead.
    fn parse_single_node(
        &mut self,
        cur_tokloc: TokLoc<'source>,
    ) -> Result<NodeRef<'arena>, LatexError<'source>> {
        let TokLoc(loc, cur_token) = cur_tokloc;
        let node = match cur_token {
            Token::Number(number) => match self.tf {
                Some(tf) => {
                    let mut builder = self.buffer.get_builder();
                    builder.transform_and_push(number, tf);
                    Node::MultiLetterIdent(builder.finish(self.arena))
                }
                None => Node::Number(number),
            },
            ref tok @ (Token::NumberWithDot(number) | Token::NumberWithComma(number)) => {
                let num = match self.tf {
                    Some(tf) => {
                        let mut builder = self.buffer.get_builder();
                        builder.transform_and_push(number, tf);
                        Node::MultiLetterIdent(builder.finish(self.arena))
                    }
                    None => Node::Number(number),
                };
                let first = self.commit(num);
                let second = self.commit(match tok {
                    Token::NumberWithDot(_) => Node::SingleLetterIdent(ops::FULL_STOP, None),
                    Token::NumberWithComma(_) => Node::Operator(ops::COMMA, None),
                    _ => unreachable!(),
                });
                Node::PseudoRow(NodeList::from_two_nodes(first, second))
            }
            Token::Letter(x) => Node::SingleLetterIdent(
                self.tf.as_ref().map_or(x, |tf| tf.transform(x, false)),
                self.var,
            ),
            Token::UprightLetter(x) => match self.tf.as_ref() {
                Some(tf) => Node::SingleLetterIdent(tf.transform(x, true), None),
                None => Node::SingleLetterIdent(x, Some(MathVariant::Normal)),
            },
            Token::Operator(op) => Node::Operator(op, None),
            Token::OpGreaterThan => Node::OpGreaterThan,
            Token::OpLessThan => Node::OpLessThan,
            Token::OpAmpersand => Node::OpAmpersand,
            Token::Function(fun) => Node::MultiLetterIdent(fun),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace | Token::Whitespace => Node::Text("\u{A0}"),
            Token::Sqrt => {
                let next = self.next_token();
                if matches!(next.token(), Token::SquareBracketOpen) {
                    let degree = self.parse_group(Token::SquareBracketClose)?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_token()?;
                    Node::Root(self.squeeze(degree, None).node(), content)
                } else {
                    let content = self.parse_node(next)?;
                    Node::Sqrt(content.node())
                }
            }
            Token::Frac(attr) | Token::Binom(attr) => {
                let num = self.parse_token()?;
                let den = self.parse_token()?;
                if matches!(cur_token, Token::Binom(_)) {
                    let lt = Some('0');
                    Node::Fenced {
                        open: ops::LEFT_PARENTHESIS,
                        close: ops::RIGHT_PARENTHESIS,
                        content: self.commit(Node::Frac { num, den, lt, attr }).node(),
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
                let open = match self.parse_token()? {
                    Node::Operator(op, _) => *op,
                    Node::Row { nodes, .. } if nodes.is_empty() => ops::NULL,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let close = match self.parse_token()? {
                    Node::Operator(op, _) => *op,
                    Node::Row { nodes, .. } if nodes.is_empty() => ops::NULL,
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
                let style = match self.parse_token()? {
                    Node::Number(num) => match num.as_bytes() {
                        b"0" => Some(Style::DisplayStyle),
                        b"1" => Some(Style::TextStyle),
                        b"2" => Some(Style::ScriptStyle),
                        b"3" => Some(Style::ScriptScriptStyle),
                        _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                    },
                    Node::Row { nodes, .. } if nodes.is_empty() => None,
                    _ => return Err(LatexError(0, LatexErrKind::UnexpectedEOF)),
                };
                let num = self.parse_token()?;
                let den = self.parse_token()?;
                let attr = None;
                Node::Fenced {
                    open,
                    close,
                    content: self.commit(Node::Frac { num, den, lt, attr }).node(),
                    style,
                }
            }
            ref tok @ (Token::Over(op) | Token::Under(op)) => {
                let target = self.parse_token()?;
                if matches!(tok, Token::Over(_)) {
                    Node::OverOp(op, Accent::True, target)
                } else {
                    Node::UnderOp(op, Accent::True, target)
                }
            }
            Token::Overset | Token::Underset => {
                let symbol = self.parse_token()?;
                let target = self.parse_token()?;
                if matches!(cur_token, Token::Overset) {
                    Node::Overset { symbol, target }
                } else {
                    Node::Underset { symbol, target }
                }
            }
            ref tok @ (Token::Overbrace(x) | Token::Underbrace(x)) => {
                let is_over = matches!(tok, Token::Overbrace(_));
                let target = self.parse_single_token()?;
                let symbol = self.commit(Node::Operator(x, None)).node();
                let base = if is_over {
                    self.commit(Node::Overset { symbol, target })
                } else {
                    self.commit(Node::Underset { symbol, target })
                };
                if (is_over && matches!(self.peek.token(), Token::Circumflex))
                    || (!is_over && matches!(self.peek.token(), Token::Underscore))
                {
                    self.next_token(); // Discard the circumflex or underscore token.
                    let expl = self.parse_single_token()?;
                    if is_over {
                        Node::Overset {
                            symbol: expl,
                            target: base.node(),
                        }
                    } else {
                        Node::Underset {
                            symbol: expl,
                            target: base.node(),
                        }
                    }
                } else {
                    return Ok(base);
                }
            }
            Token::BigOp(op) => {
                let target = if matches!(self.peek.token(), Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    self.commit(Node::Operator(op, Some(OpAttr::NoMovableLimits)))
                } else {
                    self.commit(Node::Operator(op, None))
                };
                match self.get_bounds()? {
                    Bounds(Some(under), Some(over)) => Node::UnderOver {
                        target: target.node(),
                        under,
                        over,
                    },
                    Bounds(Some(symbol), None) => Node::Underset {
                        target: target.node(),
                        symbol,
                    },
                    Bounds(None, Some(symbol)) => Node::Overset {
                        target: target.node(),
                        symbol,
                    },
                    Bounds(None, None) => {
                        return Ok(target);
                    }
                }
            }
            Token::Lim(lim) => {
                let lim = self.commit(Node::MultiLetterIdent(lim));
                if matches!(self.peek.token(), Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    Node::Underset {
                        target: lim.node(),
                        symbol: under,
                    }
                } else {
                    return Ok(lim);
                }
            }
            Token::Slashed => {
                // TODO: Actually check the braces.
                self.next_token(); // Optimistically assume that the next token is `{`
                let node = self.parse_token()?;
                self.next_token(); // Optimistically assume that the next token is `}`
                Node::Slashed(node)
            }
            Token::Not => {
                // `\not` has to be followed by something:
                match self.next_token().into_token() {
                    Token::Operator(op) => {
                        if let Some(negated) = get_negated_op(op) {
                            Node::Operator(negated, None)
                        } else {
                            Node::Operator(op, None)
                        }
                    }
                    Token::OpLessThan => Node::Operator(ops::NOT_LESS_THAN, None),
                    Token::OpGreaterThan => Node::Operator(ops::NOT_GREATER_THAN, None),
                    Token::Letter(char) | Token::UprightLetter(char) => {
                        let negated_letter = [char, '\u{338}'];
                        let mut builder = self.buffer.get_builder();
                        builder.extend(negated_letter.into_iter());
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
            Token::Transform(tf, var) => {
                let old_var = mem::replace(&mut self.var, var);
                let old_tf = mem::replace(&mut self.tf, tf);
                let token = self.next_token();
                let node_ref = self.parse_single_node(token)?;
                self.var = old_var;
                self.tf = old_tf;
                if let Node::Row { nodes, style } = node_ref.mut_node() {
                    let nodes = mem::replace(nodes, NodeList::empty());
                    let style = *style;
                    return Ok(self.merge_single_letters(nodes, style));
                }
                return Ok(node_ref);
            }
            Token::Integral(int) => {
                if matches!(self.peek.token(), Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    let target = self.commit(Node::Operator(int, None));
                    match self.get_bounds()? {
                        Bounds(Some(under), Some(over)) => Node::UnderOver {
                            target: target.node(),
                            under,
                            over,
                        },
                        Bounds(Some(symbol), None) => Node::Underset {
                            target: target.node(),
                            symbol,
                        },
                        Bounds(None, Some(symbol)) => Node::Overset {
                            target: target.node(),
                            symbol,
                        },
                        Bounds(None, None) => {
                            return Ok(target);
                        }
                    }
                } else {
                    let target = self.commit(Node::Operator(int, None));
                    match self.get_bounds()? {
                        Bounds(Some(sub), Some(sup)) => Node::SubSup {
                            target: target.node(),
                            sub,
                            sup,
                        },
                        Bounds(Some(symbol), None) => Node::Subscript {
                            target: target.node(),
                            symbol,
                        },
                        Bounds(None, Some(symbol)) => Node::Superscript {
                            target: target.node(),
                            symbol,
                        },
                        Bounds(None, None) => {
                            return Ok(target);
                        }
                    }
                }
            }
            Token::Colon => match &self.peek.token() {
                Token::Operator(ops::EQUALS_SIGN) => {
                    self.next_token(); // Discard the equals_sign token.
                    Node::Operator(ops::COLON_EQUALS, None)
                }
                Token::Operator(op @ ops::IDENTICAL_TO) => {
                    let op = *op;
                    self.next_token(); // Discard the operator token.
                    let first = self.commit(Node::OperatorWithSpacing {
                        op: ops::COLON,
                        left: Some(MathSpacing::FourMu),
                        right: Some(MathSpacing::Zero),
                    });
                    let second = self.commit(Node::OperatorWithSpacing {
                        op,
                        left: Some(MathSpacing::Zero),
                        right: None,
                    });
                    Node::PseudoRow(NodeList::from_two_nodes(first, second))
                }
                _ => Node::OperatorWithSpacing {
                    op: ops::COLON,
                    left: Some(MathSpacing::FourMu),
                    right: Some(MathSpacing::FourMu),
                },
            },
            Token::GroupBegin => {
                let content = self.parse_group(Token::GroupEnd)?;
                self.next_token(); // Discard the closing token.
                return Ok(self.squeeze(content, None));
            }
            Token::Paren(paren, spacing) => match spacing {
                Some(ParenAttr::Ordinary) => Node::SingleLetterIdent(paren.into(), None),
                None => Node::Operator(paren, Some(OpAttr::StretchyFalse)),
            },
            Token::SquareBracketOpen => {
                Node::Operator(ops::LEFT_SQUARE_BRACKET, Some(OpAttr::StretchyFalse))
            }
            Token::SquareBracketClose => {
                Node::Operator(ops::RIGHT_SQUARE_BRACKET, Some(OpAttr::StretchyFalse))
            }
            Token::Left => {
                let TokLoc(loc, next_token) = self.next_token();
                let open_paren = match next_token {
                    Token::Paren(open, _) => open,
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
                let content = self.parse_group(Token::Right)?;
                self.next_token(); // Discard the closing token.
                let TokLoc(loc, next_token) = self.next_token();
                let close_paren = match next_token {
                    Token::Paren(close, _) => close,
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
                    content: self.squeeze(content, None).node(),
                    style: None,
                }
            }
            Token::Middle => {
                let TokLoc(loc, next_token) = self.next_token();
                match next_token {
                    Token::Operator(op) | Token::Paren(op, _) => {
                        Node::Operator(op, Some(OpAttr::StretchyTrue))
                    }
                    Token::SquareBracketOpen => {
                        Node::Operator(ops::LEFT_SQUARE_BRACKET, Some(OpAttr::StretchyTrue))
                    }
                    Token::SquareBracketClose => {
                        Node::Operator(ops::RIGHT_SQUARE_BRACKET, Some(OpAttr::StretchyTrue))
                    }
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::UnexpectedToken {
                                expected: &Token::Operator(ops::NULL),
                                got: next_token,
                            },
                        ))
                    }
                }
            }
            Token::Big(size) => {
                let TokLoc(loc, next_token) = self.next_token();
                match next_token {
                    Token::Paren(paren, _) => Node::SizedParen { size, paren },
                    Token::SquareBracketOpen => Node::SizedParen {
                        size,
                        paren: ops::LEFT_SQUARE_BRACKET,
                    },
                    Token::SquareBracketClose => Node::SizedParen {
                        size,
                        paren: ops::RIGHT_SQUARE_BRACKET,
                    },
                    _ => {
                        return Err(LatexError(
                            loc,
                            LatexErrKind::UnexpectedToken {
                                expected: &Token::Paren(ops::NULL, None),
                                got: next_token,
                            },
                        ));
                    }
                }
            }
            Token::Begin => {
                self.check_lbrace()?;
                // Read the environment name.
                let env_name = self.parse_text_group()?;
                let content = self.parse_group(Token::End)?.finish();
                let end_token_loc = self.next_token().location();
                let node = match env_name {
                    "align" | "align*" | "aligned" => Node::Table {
                        content,
                        align: Align::Alternating,
                        attr: Some(FracAttr::DisplayStyleTrue),
                    },
                    "cases" => {
                        let align = Align::Left;
                        let content = self
                            .commit(Node::Table {
                                content,
                                align,
                                attr: None,
                            })
                            .node();
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
                            content: self
                                .commit(Node::Table {
                                    content,
                                    align,
                                    attr,
                                })
                                .node(),
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
                let node = self.parse_single_token()?;
                let mut builder = self.buffer.get_builder();
                if !extract_letters(&mut builder, node, None) {
                    return Err(LatexError(
                        loc,
                        LatexErrKind::ExpectedText("\\operatorname"),
                    ));
                }
                Node::MultiLetterIdent(builder.finish(self.arena))
            }
            Token::Text(transform) => {
                self.l.text_mode = true;
                let node = self.parse_single_token()?;
                let mut builder = self.buffer.get_builder();
                if !extract_letters(&mut builder, node, transform) {
                    return Err(LatexError(loc, LatexErrKind::ExpectedText("\\text")));
                }
                let text = builder.finish(self.arena);
                self.l.text_mode = false;
                // Discard any whitespace tokens that are still stored in self.peek_token.
                if matches!(self.peek.token(), Token::Whitespace) {
                    self.next_token();
                }
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Mathstrut => Node::Mathstrut,
            Token::Style(style) => {
                let content = self.parse_group(Token::GroupEnd)?;
                Node::Row {
                    nodes: content.finish(),
                    style: Some(style),
                }
            }
            Token::UnknownCommand(name) => {
                return Err(LatexError(loc, LatexErrKind::UnknownCommand(name)));
            }
            // Token::Underscore | Token::Circumflex => {
            Token::Circumflex | Token::Prime => {
                return Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: cur_token,
                        correct_place: Place::AfterOpOrIdent,
                    },
                ));
            }
            Token::Underscore => {
                let sub = self.parse_single_token()?;
                let base = self.parse_single_token()?;
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
        };
        Ok(self.commit(node))
    }

    #[inline]
    fn parse_token(&mut self) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        let token = self.next_token();
        self.parse_node(token).map(|n| n.node())
    }

    #[inline]
    fn parse_single_token(&mut self) -> Result<&'arena Node<'arena>, LatexError<'source>> {
        let token = self.next_token();
        self.parse_single_node(token).map(|n| n.node())
    }

    /// Parse the contents of a group which can contain any expression.
    fn parse_group(
        &mut self,
        end_token: Token<'source>,
    ) -> Result<NodeListBuilder<'arena>, LatexError<'source>> {
        let mut nodes = NodeListBuilder::new();

        while !self.peek.token().is_same_kind(&end_token) {
            let next = self.next_token();
            if matches!(next.token(), Token::EOF) {
                // When the input ends without the closing token.
                return Err(LatexError(
                    next.location(),
                    LatexErrKind::UnclosedGroup(end_token),
                ));
            }
            let node = self.parse_node(next)?;
            nodes.push(node);
        }
        Ok(nodes)
    }

    /// Parse the contents of a group which can only contain text.
    fn parse_text_group(&mut self) -> Result<&'source str, LatexError<'source>> {
        let result = self.l.read_text_content();
        // Discard the opening token (which is still stored as `peek`).
        let opening_loc = self.next_token().location();
        result.ok_or(LatexError(
            opening_loc,
            LatexErrKind::UnclosedGroup(Token::GroupEnd),
        ))
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
        let mut primes = NodeListBuilder::new();
        let mut prime_count = 0usize;
        while matches!(self.peek.token(), Token::Prime) {
            self.next_token(); // Discard the prime token.
            prime_count += 1;
        }
        match prime_count {
            0 => (),
            1 => primes.push(self.commit(Node::Operator(ops::PRIME, None))),
            2 => primes.push(self.commit(Node::Operator(ops::DOUBLE_PRIME, None))),
            3 => primes.push(self.commit(Node::Operator(ops::TRIPLE_PRIME, None))),
            4 => primes.push(self.commit(Node::Operator(ops::QUADRUPLE_PRIME, None))),
            _ => {
                for _ in 0..prime_count {
                    primes.push(self.commit(Node::Operator(ops::PRIME, None)));
                }
            }
        }

        // Check whether the first bound is specified and is a lower bound.
        let first_underscore = matches!(self.peek.token(), Token::Underscore);

        let (sub, sup) = if first_underscore || matches!(self.peek.token(), Token::Circumflex) {
            let first_bound = Some(self.get_sub_or_sub()?);

            // Check whether both an upper and a lower bound were specified.
            let second_underscore = matches!(self.peek.token(), Token::Underscore);
            let second_circumflex = matches!(self.peek.token(), Token::Circumflex);

            if (!first_underscore && second_circumflex) || (first_underscore && second_underscore) {
                let TokLoc(loc, token) = self.next_token();
                return Err(LatexError(
                    loc,
                    LatexErrKind::CannotBeUsedHere {
                        got: token,
                        correct_place: Place::AfterOpOrIdent,
                    },
                ));
            }

            if (first_underscore && second_circumflex) || (!first_underscore && second_underscore) {
                let second_bound = Some(self.get_sub_or_sub()?);
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
            Some(self.squeeze(primes, None))
        } else {
            sup
        };

        Ok(Bounds(sub.map(|s| s.node()), sup.map(|s| s.node())))
    }

    /// Parse the node after a `_` or `^` token.
    fn get_sub_or_sub(&mut self) -> Result<NodeRef<'arena>, LatexError<'source>> {
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
        self.parse_single_node(next)
    }

    fn squeeze(
        &self,
        list_builder: NodeListBuilder<'arena>,
        style: Option<Style>,
    ) -> NodeRef<'arena> {
        match list_builder.as_singleton_or_finish() {
            SingletonOrList::Singleton(value) => value,
            SingletonOrList::List(nodes) => self.commit(Node::Row { nodes, style }),
        }
    }

    fn merge_single_letters(
        &mut self,
        nodes: NodeList<'arena>,
        style: Option<Style>,
    ) -> NodeRef<'arena> {
        let mut list_builder = NodeListBuilder::new();
        let mut collector: Option<LetterCollector> = None;
        for node_ref in nodes {
            if let Node::SingleLetterIdent(c, _) = node_ref.node() {
                let c = *c;
                if let Some(LetterCollector {
                    ref mut only_one_char,
                    ref mut builder,
                    ..
                }) = collector
                {
                    *only_one_char = false;
                    builder.push_char(c);
                } else {
                    let mut builder = self.buffer.get_builder();
                    builder.push_char(c);
                    // We start collecting.
                    collector = Some(LetterCollector {
                        builder,
                        node_ref,
                        only_one_char: true,
                    });
                }
            } else {
                // Commit the collected letters.
                if let Some(collector) = collector.take() {
                    let node_ref = collector.finish(self.arena);
                    list_builder.push(node_ref);
                }
                list_builder.push(node_ref);
            }
        }
        if let Some(collector) = collector {
            let node_ref = collector.finish(self.arena);
            list_builder.push(node_ref);
        }
        self.squeeze(list_builder, style)
    }
}

struct Bounds<'arena>(Option<&'arena Node<'arena>>, Option<&'arena Node<'arena>>);

struct LetterCollector<'arena, 'buffer> {
    builder: StringBuilder<'buffer>,
    node_ref: NodeRef<'arena>,
    only_one_char: bool,
}

impl<'arena> LetterCollector<'arena, '_> {
    fn finish(self, arena: &'arena Arena) -> NodeRef<'arena> {
        let node = self.node_ref.mut_node();
        if !self.only_one_char {
            *node = Node::MultiLetterIdent(self.builder.finish(arena));
        }
        self.node_ref
    }
}

/// Extract the text of all single-letter identifiers and operators in `node`.
/// This function cannot be a method, because we need to borrow arena immutably
/// but buffer mutably. This is not possible with a mutable self reference.
///
/// Returns false if no letters could be extracted.
fn extract_letters<'arena>(
    buffer: &mut StringBuilder,
    node: &'arena Node<'arena>,
    transform: Option<TextTransform>,
) -> bool {
    match node {
        Node::SingleLetterIdent(c, var) => {
            let is_normal = var.is_some();
            buffer.push_char(
                transform
                    .as_ref()
                    .map_or(*c, |t| t.transform(*c, is_normal)),
            );
        }
        Node::Row { nodes, .. } | Node::PseudoRow(nodes) => {
            for node in nodes.iter() {
                if !extract_letters(buffer, node, transform) {
                    return false;
                }
            }
        }
        Node::Number(n) => {
            match transform {
                Some(tf) => buffer.transform_and_push(n, tf),
                None => buffer.push_str(n),
            };
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

#[cfg(test)]
mod tests {
    use insta::assert_ron_snapshot;

    use super::*;

    #[test]
    fn ast_test() {
        let problems = [
            ("slightly_more_complex_fraction", r"\frac{12}{5}"),
            ("complex_square_root", r"\sqrt{x+2}"),
            ("long_sub_super", r"x_{92}^{31415}"),
            ("integral_with_reversed_limits", r"\int\limits^1_0 dx"),
            ("matrix", r"\begin{pmatrix} x \\ y \end{pmatrix}"),
            ("number_with_dot", r"4.x"),
            ("double_prime", r"f''"),
        ];
        for (name, problem) in problems.into_iter() {
            let arena = Arena::new();
            let l = Lexer::new(problem);
            let mut p = Parser::new(l, &arena);
            let ast = p.parse().expect("Parsing failed");
            if let Node::PseudoRow(nodes) = ast {
                assert_ron_snapshot!(name, &nodes, problem);
            }
        }
    }
}
