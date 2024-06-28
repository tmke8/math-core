use std::mem;

use crate::{
    arena::{
        Arena, Buffer, NodeList, NodeListBuilder, NodeReference, SingletonOrList, StrBound,
        StrReference,
    },
    ast::Node,
    attribute::{Accent, Align, MathSpacing, MathVariant, OpAttr, Style, TextTransform},
    commands::get_negated_op,
    error::LatexError,
    lexer::Lexer,
    ops,
    token::Token,
};

#[derive(Debug)]
pub(crate) struct Parser<'source> {
    l: Lexer<'source>,
    peek_token: Token<'source>,
    buffer: Buffer,
    arena: Arena<'source>,
    tf: Option<TextTransform>,
    var: Option<MathVariant>,
}
impl<'source> Parser<'source> {
    pub(crate) fn new(l: Lexer<'source>, buffer: Buffer, arena: Arena<'source>) -> Self {
        let mut p = Parser {
            l,
            peek_token: Token::EOF,
            buffer,
            arena,
            tf: None,
            var: None,
        };
        // Discard the EOF token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    pub(crate) fn into_inner(self) -> (Buffer, Arena<'source>) {
        (self.buffer, self.arena)
    }

    fn next_token(&mut self) -> Token<'source> {
        let peek_token = self.l.next_token(self.peek_token.acts_on_a_digit());
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek_token, peek_token)
    }

    pub(crate) fn parse(&mut self) -> Result<Node<'source>, LatexError<'source>> {
        let mut list_builder = NodeListBuilder::new();
        let mut cur_token = self.next_token();

        while !matches!(cur_token, Token::EOF) {
            let node = self.parse_node(cur_token)?;
            list_builder.push(&mut self.arena, node);
            cur_token = self.next_token();
        }

        Ok(Node::PseudoRow(list_builder.finish()))
    }

    fn parse_node(
        &mut self,
        cur_token: Token<'source>,
    ) -> Result<NodeReference, LatexError<'source>> {
        let left = self.parse_single_node(cur_token)?;

        match self.get_bounds()? {
            Bounds(Some(sub), Some(sup)) => Ok(self.commit_node(Node::SubSup {
                target: left,
                sub,
                sup,
            })),
            Bounds(Some(symbol), None) => Ok(self.commit_node(Node::Subscript(left, symbol))),
            Bounds(None, Some(symbol)) => Ok(self.commit_node(Node::Superscript(left, symbol))),
            Bounds(None, None) => Ok(left),
        }
    }

    fn commit_node(&mut self, node: Node<'source>) -> NodeReference {
        self.arena.push(node)
    }

    // Read the node immediately after without worrying about whether
    // the infix operator `_`, `^`, `'` will continue
    //
    // Note: Use `parse_node()` when reading nodes correctly in
    // consideration of infix operators.
    fn parse_single_node(
        &mut self,
        cur_token: Token<'source>,
    ) -> Result<NodeReference, LatexError<'source>> {
        let node = match cur_token {
            Token::Number(number) => Node::Number(number),
            ref tok @ (Token::NumberWithDot(number) | Token::NumberWithComma(number)) => {
                let op = match tok {
                    Token::NumberWithDot(_) => ops::FULL_STOP,
                    Token::NumberWithComma(_) => ops::COMMA,
                    _ => unreachable!(),
                };
                let first = self.commit_node(Node::Number(number));
                let second = self.commit_node(Node::Operator(op, None));
                Node::PseudoRow(NodeList::from_two_nodes(&mut self.arena, first, second))
            }
            Token::Letter(x) => {
                Node::SingleLetterIdent(self.tf.as_ref().map_or(x, |tf| tf.transform(x)), self.var)
            }
            Token::NormalLetter(x) => Node::SingleLetterIdent(x, Some(MathVariant::Normal)),
            Token::Operator(op) => match self.tf.as_ref() {
                None => Node::Operator(op, None),
                Some(tf) => Node::SingleLetterIdent(tf.transform(op.into()), None),
            },
            Token::OpGreaterThan => Node::OpGreaterThan,
            Token::OpLessThan => Node::OpLessThan,
            Token::OpAmpersand => Node::OpAmpersand,
            Token::Function(fun) => Node::MultiLetterIdent(self.buffer.push_str(fun)),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace | Token::Whitespace => {
                Node::Text(self.buffer.push_str("\u{A0}"))
            }
            Token::Sqrt => {
                let next_token = self.next_token();
                if matches!(next_token, Token::Paren(ops::LEFT_SQUARE_BRACKET)) {
                    let degree = self.parse_group(Token::SquareBracketClose)?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_token()?;
                    Node::Root(self.squeeze(degree, None), content)
                } else {
                    let content = self.parse_node(next_token)?;
                    Node::Sqrt(content)
                }
            }
            Token::Frac(displaystyle) | Token::Binom(displaystyle) => {
                let numerator = self.parse_token()?;
                let denominator = self.parse_token()?;
                if matches!(cur_token, Token::Binom(_)) {
                    let inner = Node::Frac(numerator, denominator, Some('0'), displaystyle);
                    Node::Fenced {
                        open: ops::LEFT_PARENTHESIS,
                        close: ops::RIGHT_PARENTHESIS,
                        content: self.commit_node(inner),
                        style: None,
                    }
                } else {
                    Node::Frac(numerator, denominator, None, displaystyle)
                }
            }
            Token::Genfrac => {
                // TODO: This should not just blindly try to parse a node.
                // Rather, we should explicitly attempt to parse a group (aka Row),
                // and if that doesn't work, we try to parse it as an Operator,
                // and if that still doesn't work, we return an error.
                let open = match self.parse_token()?.as_node(&self.arena) {
                    Node::Operator(op, _) => *op,
                    Node::Row(elements, _) if elements.is_empty() => ops::NULL,
                    _ => return Err(LatexError::UnexpectedEOF),
                };
                let close = match self.parse_token()?.as_node(&self.arena) {
                    Node::Operator(op, _) => *op,
                    Node::Row(elements, _) if elements.is_empty() => ops::NULL,
                    _ => return Err(LatexError::UnexpectedEOF),
                };
                self.check_lbrace()?;
                // The default line thickness in LaTeX is 0.4pt.
                // TODO: Support other line thicknesses.
                // We could maybe store them as multiples of 0.4pt,
                // so that we can render them as percentages.
                let line_thickness = match self.parse_text_group()?.trim() {
                    "" => None,
                    "0pt" => Some('0'),
                    _ => return Err(LatexError::UnexpectedEOF),
                };
                let style = match self.parse_token()?.as_node(&self.arena) {
                    Node::Number(num) => match num.parse::<u8>() {
                        Ok(0) => Some(Style::DisplayStyle),
                        Ok(1) => Some(Style::TextStyle),
                        Ok(2) => Some(Style::ScriptStyle),
                        Ok(3) => Some(Style::ScriptScriptStyle),
                        Ok(_) | Err(_) => return Err(LatexError::UnexpectedEOF),
                    },
                    Node::Row(elements, _) if elements.is_empty() => None,
                    _ => return Err(LatexError::UnexpectedEOF),
                };
                let numerator = self.parse_token()?;
                let denominator = self.parse_token()?;
                let inner = Node::Frac(numerator, denominator, line_thickness, None);
                let content = self.commit_node(inner);
                Node::Fenced {
                    open,
                    close,
                    content,
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
                if (is_over && matches!(self.peek_token, Token::Circumflex))
                    || (!is_over && matches!(self.peek_token, Token::Underscore))
                {
                    self.next_token(); // Discard the circumflex or underscore token.
                    let expl = self.parse_single_token()?;
                    let op = self.commit_node(Node::Operator(x, None));
                    if is_over {
                        let symbol = self.commit_node(Node::Overset {
                            symbol: expl,
                            target: op,
                        });
                        Node::Overset { symbol, target }
                    } else {
                        let symbol = self.commit_node(Node::Underset {
                            symbol: expl,
                            target: op,
                        });
                        Node::Underset { symbol, target }
                    }
                } else {
                    let symbol = self.commit_node(Node::Operator(x, None));
                    if is_over {
                        Node::Overset { symbol, target }
                    } else {
                        Node::Underset { symbol, target }
                    }
                }
            }
            Token::BigOp(op) => {
                let target = if matches!(self.peek_token, Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    Node::Operator(op, Some(OpAttr::NoMovableLimits))
                } else {
                    Node::Operator(op, None)
                };
                let target = self.commit_node(target);
                match self.get_bounds()? {
                    Bounds(Some(under), Some(over)) => Node::UnderOver {
                        target,
                        under,
                        over,
                    },
                    Bounds(Some(symbol), None) => Node::Underset { target, symbol },
                    Bounds(None, Some(symbol)) => Node::Overset { target, symbol },
                    Bounds(None, None) => Node::Operator(op, None),
                }
            }
            Token::Lim(lim) => {
                let lim = Node::MultiLetterIdent(self.buffer.push_str(lim));
                if matches!(self.peek_token, Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    Node::Underset {
                        target: self.commit_node(lim),
                        symbol: under,
                    }
                } else {
                    lim
                }
            }
            Token::Slashed => {
                self.next_token(); // Optimistically skip the next token.
                let node = self.parse_token()?;
                self.next_token(); // Optimistically skip the next token.
                Node::Slashed(node)
            }
            Token::Not => {
                match self.peek_token {
                    Token::Operator(op) => {
                        self.next_token(); // Discard the operator token.
                        if let Some(negated) = get_negated_op(op) {
                            Node::Operator(negated, None)
                        } else {
                            Node::Operator(op, None)
                        }
                    }
                    Token::OpLessThan => {
                        self.next_token(); // Discard the less-than token.
                        Node::Operator(ops::NOT_LESS_THAN, None)
                    }
                    Token::OpGreaterThan => {
                        self.next_token(); // Discard the greater-than token.
                        Node::Operator(ops::NOT_GREATER_THAN, None)
                    }
                    Token::Letter(char) | Token::NormalLetter(char) => {
                        self.next_token(); // Discard the letter token.
                        let negated_letter = [char, '\u{338}'];
                        Node::MultiLetterIdent(self.buffer.extend(negated_letter))
                    }
                    _ => {
                        return Err(LatexError::CannotBeUsedHere {
                            got: cur_token,
                            correct_place: "before supported operators",
                        })
                    }
                }
            }
            Token::NormalVariant => {
                let old_var = mem::replace(&mut self.var, Some(MathVariant::Normal));
                let old_tf = self.tf.take();
                let node_ref = self.parse_single_token()?;
                self.var = old_var;
                self.tf = old_tf;
                if let Node::Row(nodes, style) = node_ref.as_node_mut(&mut self.arena) {
                    let nodes = mem::replace(nodes, NodeList::empty());
                    let style = style.clone();
                    return Ok(self.merge_single_letters(nodes, style));
                }
                return Ok(node_ref);
            }
            Token::Transform(tf) => {
                let old_tf = mem::replace(&mut self.tf, Some(tf));
                let node_ref = self.parse_single_token()?;
                self.tf = old_tf;
                if let Node::Row(nodes, style) = node_ref.as_node_mut(&mut self.arena) {
                    let nodes = mem::replace(nodes, NodeList::empty());
                    let style = style.clone();
                    return Ok(self.merge_single_letters(nodes, style));
                }
                return Ok(node_ref);
            }
            Token::Integral(int) => {
                if matches!(self.peek_token, Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    let target = self.commit_node(Node::Operator(int, None));
                    match self.get_bounds()? {
                        Bounds(Some(under), Some(over)) => Node::UnderOver {
                            target,
                            under,
                            over,
                        },
                        Bounds(Some(symbol), None) => Node::Underset { target, symbol },
                        Bounds(None, Some(symbol)) => Node::Overset { target, symbol },
                        Bounds(None, None) => Node::Operator(int, None),
                    }
                } else {
                    let target = self.commit_node(Node::Operator(int, None));
                    match self.get_bounds()? {
                        Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                        Bounds(Some(symbol), None) => Node::Subscript(target, symbol),
                        Bounds(None, Some(symbol)) => Node::Superscript(target, symbol),
                        Bounds(None, None) => Node::Operator(int, None),
                    }
                }
            }
            Token::Colon => match &self.peek_token {
                Token::Operator(op @ (ops::EQUALS_SIGN | ops::IDENTICAL_TO)) => {
                    let op = *op;
                    self.next_token(); // Discard the operator token.
                    let first = self.commit_node(Node::OperatorWithSpacing {
                        op: ops::COLON,
                        left: Some(MathSpacing::FourMu),
                        right: Some(MathSpacing::Zero),
                    });
                    let second = self.commit_node(Node::OperatorWithSpacing {
                        op,
                        left: Some(MathSpacing::Zero),
                        right: None,
                    });
                    Node::PseudoRow(NodeList::from_two_nodes(&mut self.arena, first, second))
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
            Token::Paren(paren) => Node::Operator(paren, Some(OpAttr::StretchyFalse)),
            Token::SquareBracketClose => {
                Node::Operator(ops::RIGHT_SQUARE_BRACKET, Some(OpAttr::StretchyFalse))
            }
            Token::Left => {
                let open = match self.next_token() {
                    Token::Paren(open) => open,
                    Token::SquareBracketClose => ops::RIGHT_SQUARE_BRACKET,
                    Token::Operator(ops::FULL_STOP) => ops::NULL,
                    token => {
                        return Err(LatexError::MissingParenthesis {
                            location: Token::Left,
                            got: token,
                        })
                    }
                };
                let content = self.parse_group(Token::Right)?;
                self.next_token(); // Discard the closing token.
                let close = match self.next_token() {
                    Token::Paren(close) => close,
                    Token::SquareBracketClose => ops::RIGHT_SQUARE_BRACKET,
                    Token::Operator(ops::FULL_STOP) => ops::NULL,
                    token => {
                        return Err(LatexError::MissingParenthesis {
                            location: Token::Right,
                            got: token,
                        })
                    }
                };
                Node::Fenced {
                    open,
                    close,
                    content: self.squeeze(content, None),
                    style: None,
                }
            }
            Token::Middle => match self.next_token() {
                Token::Operator(op) | Token::Paren(op) => {
                    Node::Operator(op, Some(OpAttr::StretchyTrue))
                }
                Token::SquareBracketClose => {
                    Node::Operator(ops::RIGHT_SQUARE_BRACKET, Some(OpAttr::StretchyTrue))
                }
                tok => {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::Operator(ops::NULL),
                        got: tok,
                    })
                }
            },
            Token::Big(size) => match self.next_token() {
                Token::Paren(paren) => Node::SizedParen { size, paren },
                Token::SquareBracketClose => Node::SizedParen {
                    size,
                    paren: ops::RIGHT_SQUARE_BRACKET,
                },
                tok => {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::Paren(ops::NULL),
                        got: tok,
                    });
                }
            },
            Token::Begin => {
                self.check_lbrace()?;
                // Read the environment name.
                let environment = self.parse_text_group()?;
                let node = match environment {
                    "align" | "align*" | "aligned" => self.parse_table(Align::Alternating)?,
                    "cases" => {
                        let content = self.parse_table(Align::Left)?;
                        Node::Fenced {
                            open: ops::LEFT_CURLY_BRACKET,
                            close: ops::NULL,
                            content: self.commit_node(content),
                            style: None,
                        }
                    }
                    "matrix" => self.parse_table(Align::Center)?,
                    matrix_variant @ ("pmatrix" | "bmatrix" | "vmatrix") => {
                        let content = self.parse_table(Align::Center)?;
                        let (open, close) = match matrix_variant {
                            "pmatrix" => (ops::LEFT_PARENTHESIS, ops::RIGHT_PARENTHESIS),
                            "bmatrix" => (ops::LEFT_SQUARE_BRACKET, ops::RIGHT_SQUARE_BRACKET),
                            "vmatrix" => (ops::VERTICAL_LINE, ops::VERTICAL_LINE),
                            // SAFETY: `matrix_variant` is one of the three strings above.
                            _ => unsafe { std::hint::unreachable_unchecked() },
                        };
                        Node::Fenced {
                            open,
                            close,
                            content: self.commit_node(content),
                            style: None,
                        }
                    }
                    _ => {
                        return Err(LatexError::UnknownEnvironment(environment));
                    }
                };
                self.check_lbrace()?;
                let end_name = self.parse_text_group()?;
                if end_name != environment {
                    return Err(LatexError::MismatchedEnvironment {
                        expected: environment,
                        got: end_name,
                    });
                }

                node
            }
            Token::OperatorName => {
                // TODO: Don't parse a node just to immediately destructure it.
                let node = self.parse_single_token()?.as_node(&self.arena);
                let start = self.buffer.end();
                extract_letters(&self.arena, &mut self.buffer, node)?;
                let end = self.buffer.end();
                Node::MultiLetterIdent(StrReference::new(start, end))
            }
            Token::Text => {
                self.l.text_mode = true;
                let node = self.parse_single_token()?.as_node(&self.arena);
                let start = self.buffer.end();
                extract_letters(&self.arena, &mut self.buffer, node)?;
                let end = self.buffer.end();
                self.l.text_mode = false;
                // Discard any whitespace tokens that are still stored in self.peek_token.
                if matches!(self.peek_token, Token::Whitespace) {
                    self.next_token();
                }
                Node::Text(StrReference::new(start, end))
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Mathstrut => Node::Mathstrut,
            Token::Style(style) => {
                let content = self.parse_group(Token::GroupEnd)?;
                Node::Row(content.finish(), Some(style))
            }
            Token::UnknownCommand(name) => {
                return Err(LatexError::UnknownCommand(name));
            }
            // Token::Underscore | Token::Circumflex => {
            Token::Circumflex | Token::Prime => {
                return Err(LatexError::CannotBeUsedHere {
                    got: cur_token,
                    correct_place: "after an identifier or operator",
                });
            }
            Token::Underscore => {
                let sub = self.parse_single_token()?;
                let base = self.parse_single_token()?;
                Node::Multiscript { base, sub }
            }
            Token::Limits => {
                return Err(LatexError::CannotBeUsedHere {
                    got: cur_token,
                    correct_place: r"after \int, \sum, ...",
                })
            }
            Token::EOF => return Err(LatexError::UnexpectedEOF),
            Token::End | Token::Right | Token::GroupEnd => {
                return Err(LatexError::UnexpectedClose(cur_token))
            }
        };
        Ok(self.commit_node(node))
    }

    #[inline]
    fn parse_token(&mut self) -> Result<NodeReference, LatexError<'source>> {
        let token = self.next_token();
        self.parse_node(token)
    }

    #[inline]
    fn parse_single_token(&mut self) -> Result<NodeReference, LatexError<'source>> {
        let token = self.next_token();
        self.parse_single_node(token)
    }

    /// Parse the contents of a group which can contain any expression.
    fn parse_group(
        &mut self,
        end_token: Token<'source>,
    ) -> Result<NodeListBuilder, LatexError<'source>> {
        let mut nodes = NodeListBuilder::new();

        while !self.peek_token.is_same_kind(&end_token) {
            let token = self.next_token();
            if matches!(token, Token::EOF) {
                // When the input ends without the closing token.
                return Err(LatexError::UnclosedGroup(end_token));
            }
            let node = self.parse_node(token)?;
            nodes.push(&mut self.arena, node);
        }
        Ok(nodes)
    }

    /// Parse the contents of a group which can only contain text.
    fn parse_text_group(&mut self) -> Result<&'source str, LatexError<'source>> {
        let result = self
            .l
            .read_text_content()
            .ok_or(LatexError::UnclosedGroup(Token::GroupEnd));
        self.next_token(); // Discard the opening token (which is still stored as `peek`).
        result
    }

    #[inline]
    fn parse_table(&mut self, align: Align) -> Result<Node<'source>, LatexError<'source>> {
        // Read the contents of \begin..\end.
        let content = self.parse_group(Token::End)?;
        self.next_token(); // Discard the closing token.
        Ok(Node::Table(content.finish(), align))
    }

    fn check_lbrace(&mut self) -> Result<(), LatexError<'source>> {
        if !matches!(self.peek_token, Token::GroupBegin) {
            return Err(LatexError::UnexpectedToken {
                expected: Token::GroupBegin,
                got: self.next_token(),
            });
        }
        Ok(())
    }

    /// Parse the bounds of an integral, sum, or product.
    /// These bounds are preceeded by `_` or `^`.
    fn get_bounds(&mut self) -> Result<Bounds, LatexError<'source>> {
        let mut primes = NodeListBuilder::new();
        while matches!(self.peek_token, Token::Prime) {
            self.next_token(); // Discard the prime token.
            let node_ref = self.commit_node(Node::Operator(ops::PRIME, None));
            primes.push(&mut self.arena, node_ref);
        }

        // Check whether the first bound is specified and is a lower bound.
        let first_underscore = matches!(self.peek_token, Token::Underscore);

        let (sub, sup) = if first_underscore || matches!(self.peek_token, Token::Circumflex) {
            let first_bound = Some(self.get_sub_or_sub()?);

            // Check whether both an upper and a lower bound were specified.
            let second_underscore = matches!(self.peek_token, Token::Underscore);
            let second_circumflex = matches!(self.peek_token, Token::Circumflex);

            if (!first_underscore && second_circumflex) || (first_underscore && second_underscore) {
                return Err(LatexError::CannotBeUsedHere {
                    got: self.next_token(),
                    correct_place: "after an identifier or operator",
                });
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
                primes.push(&mut self.arena, sup);
            }
            Some(self.squeeze(primes, None))
        } else {
            sup
        };

        Ok(Bounds(sub, sup))
    }

    /// Parse the node after a `_` or `^` token.
    fn get_sub_or_sub(&mut self) -> Result<NodeReference, LatexError<'source>> {
        self.next_token(); // Discard the underscore or circumflex token.
        let next_token = self.next_token();
        if matches!(
            next_token,
            Token::Underscore | Token::Circumflex | Token::Prime
        ) {
            return Err(LatexError::CannotBeUsedHere {
                got: next_token,
                correct_place: "after an identifier or operator",
            });
        }
        self.parse_single_node(next_token)
    }

    fn squeeze(&mut self, list_builder: NodeListBuilder, style: Option<Style>) -> NodeReference {
        match list_builder.as_singleton_or_finish() {
            SingletonOrList::Singleton(value) => value,
            SingletonOrList::List(list) => self.commit_node(Node::Row(list, style)),
        }
    }

    fn merge_single_letters(&mut self, nodes: NodeList, style: Option<Style>) -> NodeReference {
        let mut list_builder = NodeListBuilder::new();
        let mut collector: Option<LetterCollector> = None;
        let mut iter = nodes.into_man_iter();
        while let Some((node_ref, node)) = iter.next(&mut self.arena) {
            if let Node::SingleLetterIdent(c, _) = node {
                if let Some(LetterCollector {
                    ref mut only_one_char,
                    ..
                }) = collector
                {
                    *only_one_char = false;
                } else {
                    // We start collecting.
                    collector = Some(LetterCollector {
                        start: self.buffer.end(),
                        node_ref,
                        only_one_char: true,
                    });
                }
                self.buffer.push(*c);
            } else {
                // Commit the collected letters.
                if let Some(collector) = collector.take() {
                    let node_ref = collector.finish(&mut self.arena, self.buffer.end());
                    list_builder.push(&mut self.arena, node_ref);
                }
                list_builder.push(&mut self.arena, node_ref);
            }
        }
        if let Some(collector) = collector {
            let node_ref = collector.finish(&mut self.arena, self.buffer.end());
            list_builder.push(&mut self.arena, node_ref);
        }
        self.squeeze(list_builder, style)
    }
}

struct Bounds(Option<NodeReference>, Option<NodeReference>);

struct LetterCollector {
    start: StrBound,
    node_ref: NodeReference,
    only_one_char: bool,
}

impl LetterCollector {
    fn finish(self, arena: &mut Arena, end: StrBound) -> NodeReference {
        let node = self.node_ref.as_node_mut(arena);
        if !self.only_one_char {
            *node = Node::MultiLetterIdent(StrReference::new(self.start, end));
        }
        self.node_ref
    }
}

/// Extract the text of all single-letter identifiers and operators in `node`.
/// This function cannot be a method, because we need to borrow arena immutably
/// but buffer mutably. This is not possible with a mutable self reference.
fn extract_letters<'source>(
    arena: &Arena<'source>,
    buffer: &mut Buffer,
    node: &Node<'source>,
) -> Result<(), LatexError<'source>> {
    match node {
        Node::SingleLetterIdent(c, _) => {
            buffer.push(*c);
        }
        Node::Row(nodes, _) => {
            for node in nodes.iter(arena) {
                extract_letters(arena, buffer, node)?;
            }
        }
        Node::Number(n) => {
            buffer.push_str(n);
        }
        Node::Operator(op, _) | Node::OperatorWithSpacing { op, .. } => {
            buffer.push(op.into());
        }
        Node::Text(str_ref) => {
            let string = str_ref.as_str(buffer).to_string();
            buffer.push_str(string.as_str());
        }
        _ => return Err(LatexError::ExpectedText("\\operatorname")),
    }
    Ok(())
}
