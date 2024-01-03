use bumpalo::{
    boxed::Box,
    collections::{CollectIn, String, Vec},
    Bump,
};

use crate::attribute::{Accent, PhantomWidth, Stretchy};
use crate::{
    ast::Node,
    attribute::{Align, LineThickness, MathVariant, TextTransform},
    error::LatexError,
    lexer::Lexer,
    ops,
    token::Token,
};
use std::mem;

#[derive(Debug, Clone)]
pub(crate) struct Parser<'a> {
    l: Lexer<'a>,
    peek_token: Token<'a>,
    alloc: &'a Bump,
}
impl<'a> Parser<'a> {
    pub(crate) fn new(l: Lexer<'a>, alloc: &'a Bump) -> Self {
        let mut p = Parser {
            l,
            peek_token: Token::Null,
            alloc,
        };
        // Discard the null token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    fn next_token(&mut self) -> Token<'a> {
        let peek_token = if self.peek_token.acts_on_a_digit() && self.l.cur.is_ascii_digit() {
            let num = self.l.cur;
            self.l.read_char();
            let mut buf: String<'a> = String::new_in(self.alloc);
            buf.push(num);
            Token::<'a>::Number(buf)
        } else {
            self.l.next_token()
        };
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek_token, peek_token)
    }

    #[inline]
    fn peek_token_is(&self, expected_token: Token) -> bool {
        self.peek_token == expected_token
    }

    pub(crate) fn parse(&mut self) -> Result<Node<'a>, LatexError<'a>> {
        let mut nodes = Vec::new_in(self.alloc);
        let mut cur_token = self.next_token();

        while cur_token != Token::EOF {
            nodes.push(self.parse_node(cur_token)?);
            cur_token = self.next_token();
        }

        if nodes.len() == 1 {
            // SAFETY: `nodes` is not empty.
            unsafe { Ok(nodes.into_iter().next().unwrap_unchecked()) }
        } else {
            Ok(Node::PseudoRow(nodes))
        }
    }

    fn parse_node(&mut self, cur_token: Token<'a>) -> Result<Node<'a>, LatexError<'a>> {
        let left = self.parse_single_node(cur_token)?;

        match self.peek_token {
            Token::Underscore => {
                self.next_token(); // Discard the underscore token.
                let right = self.parse_token()?;
                Ok(Node::Subscript(
                    Box::new_in(left, self.alloc),
                    Box::new_in(right, self.alloc),
                ))
            }
            Token::Circumflex => {
                self.next_token(); // Discard the circumflex token.
                let right = self.parse_token()?;
                Ok(Node::Superscript(
                    Box::new_in(left, self.alloc),
                    Box::new_in(right, self.alloc),
                ))
            }
            _ => Ok(left),
        }
    }

    // Read the node immediately after without worrying about whether
    // the infix operator `_`, `^`, '\' '' will continue
    //
    // Note: Use `parse_node()` when reading nodes correctly in
    // consideration of infix operators.
    fn parse_single_node(&mut self, cur_token: Token<'a>) -> Result<Node<'a>, LatexError<'a>> {
        let node = match cur_token {
            Token::Number(number) => Node::Number(number),
            Token::Letter(x) => Node::SingleLetterIdent(x, None),
            Token::NormalLetter(x) => Node::SingleLetterIdent(x, Some(MathVariant::Normal)),
            Token::Operator(op) => Node::Operator(op, None),
            Token::Function(fun) => {
                Node::MultiLetterIdent(String::from_str_in(fun, self.alloc), None)
            }
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace => Node::Text(String::from_str_in("\u{A0}", self.alloc)),
            Token::Sqrt => {
                let next_token = self.next_token();
                if next_token == Token::Paren(ops::LEFT_SQUARE_BRACKET) {
                    let degree = self.parse_group(Token::Paren(ops::RIGHT_SQUARE_BRACKET))?;
                    let content = self.parse_token()?;
                    Node::Root(
                        Box::new_in(degree, self.alloc),
                        Box::new_in(content, self.alloc),
                    )
                } else {
                    let content = self.parse_node(next_token)?;
                    Node::Sqrt(Box::new_in(content, self.alloc))
                }
            }
            Token::Frac(displaystyle) => {
                let numerator = self.parse_token()?;
                let denominator = self.parse_token()?;
                Node::Frac(
                    Box::new_in(numerator, self.alloc),
                    Box::new_in(denominator, self.alloc),
                    LineThickness::Medium,
                    displaystyle,
                )
            }
            Token::Binom(displaystyle) => {
                let numerator = self.parse_token()?;
                let denominator = self.parse_token()?;

                Node::Fenced {
                    open: ops::LEFT_PARENTHESIS,
                    close: ops::RIGHT_PARENTHESIS,
                    content: Box::new_in(
                        Node::Frac(
                            Box::new_in(numerator, self.alloc),
                            Box::new_in(denominator, self.alloc),
                            LineThickness::Zero,
                            displaystyle,
                        ),
                        self.alloc,
                    ),
                }
            }
            Token::Over(op) => {
                let target = self.parse_token()?;
                Node::OverOp(op, Accent::True, Box::new_in(target, self.alloc))
            }
            Token::Under(op) => {
                let target = self.parse_token()?;
                Node::UnderOp(op, Accent::True, Box::new_in(target, self.alloc))
            }
            Token::Overset => {
                let over = self.parse_token()?;
                let target = self.parse_token()?;
                Node::Overset {
                    over: Box::new_in(over, self.alloc),
                    target: Box::new_in(target, self.alloc),
                }
            }
            Token::Underset => {
                let under = self.parse_token()?;
                let target = self.parse_token()?;
                Node::Underset {
                    under: Box::new_in(under, self.alloc),
                    target: Box::new_in(target, self.alloc),
                }
            }
            Token::Overbrace(x) => {
                let target = self.parse_single_token()?;
                if self.peek_token_is(Token::Circumflex) {
                    self.next_token(); // Discard the circumflex token.
                    let expl = self.parse_single_token()?;
                    let over = Node::Overset {
                        over: Box::new_in(expl, self.alloc),
                        target: Box::new_in(Node::Operator(x, None), self.alloc),
                    };
                    Node::Overset {
                        over: Box::new_in(over, self.alloc),
                        target: Box::new_in(target, self.alloc),
                    }
                } else {
                    Node::Overset {
                        over: Box::new_in(Node::Operator(x, None), self.alloc),
                        target: Box::new_in(target, self.alloc),
                    }
                }
            }
            Token::Underbrace(x) => {
                let target = self.parse_single_token()?;
                if self.peek_token_is(Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let expl = self.parse_single_token()?;
                    let under = Node::Underset {
                        under: Box::new_in(expl, self.alloc),
                        target: Box::new_in(Node::Operator(x, None), self.alloc),
                    };
                    Node::Underset {
                        under: Box::new_in(under, self.alloc),
                        target: Box::new_in(target, self.alloc),
                    }
                } else {
                    Node::Underset {
                        under: Box::new_in(Node::Operator(x, None), self.alloc),
                        target: Box::new_in(target, self.alloc),
                    }
                }
            }
            Token::BigOp(op) => match self.peek_token {
                Token::Underscore => {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    if self.peek_token_is(Token::Circumflex) {
                        self.next_token(); // Discard the circumflex token.
                        let over = self.parse_single_token()?;
                        Node::UnderOver {
                            target: Box::new_in(Node::Operator(op, None), self.alloc),
                            under: Box::new_in(under, self.alloc),
                            over: Box::new_in(over, self.alloc),
                        }
                    } else {
                        Node::Underset {
                            target: Box::new_in(Node::Operator(op, None), self.alloc),
                            under: Box::new_in(under, self.alloc),
                        }
                    }
                }
                Token::Circumflex => {
                    self.next_token(); // Discard the circumflex token.
                    let over = self.parse_single_token()?;
                    if self.peek_token_is(Token::Underscore) {
                        self.next_token(); // Discard the underscore token.
                        let under = self.parse_single_token()?;
                        Node::UnderOver {
                            target: Box::new_in(Node::Operator(op, None), self.alloc),
                            under: Box::new_in(under, self.alloc),
                            over: Box::new_in(over, self.alloc),
                        }
                    } else {
                        Node::Overset {
                            over: Box::new_in(Node::Operator(op, None), self.alloc),
                            target: Box::new_in(over, self.alloc),
                        }
                    }
                }
                _ => Node::Operator(op, None),
            },
            Token::Lim(lim) => {
                let lim = Node::MultiLetterIdent(String::from_str_in(lim, self.alloc), None);
                if self.peek_token_is(Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    Node::Underset {
                        target: Box::new_in(lim, self.alloc),
                        under: Box::new_in(under, self.alloc),
                    }
                } else {
                    lim
                }
            }
            Token::Slashed => {
                self.next_token(); // Optimistically skip the next token.
                let node = self.parse_token()?;
                self.next_token(); // Optimistically skip the next token.
                Node::Slashed(Box::new_in(node, self.alloc))
            }
            Token::NormalVariant => {
                let node = self.parse_token()?;
                let node = if let Node::Row(nodes) = node {
                    self.merge_single_letters(nodes)
                } else {
                    node
                };
                self.set_variant(node, MathVariant::Normal)
            }
            Token::Style(var) => {
                let node = self.parse_token()?;
                let node = if let Node::Row(nodes) = node {
                    self.merge_single_letters(nodes)
                } else {
                    node
                };
                self.transform_text(node, var)
            }
            Token::Integral(int) => match self.peek_token {
                Token::Underscore => {
                    self.next_token(); // Discard the underscore token.
                    let sub = self.parse_single_token()?;
                    if self.peek_token_is(Token::Circumflex) {
                        self.next_token(); // Discard the circumflex token.
                        let sup = self.parse_single_token()?;
                        Node::SubSup {
                            target: Box::new_in(Node::Operator(int, None), self.alloc),
                            sub: Box::new_in(sub, self.alloc),
                            sup: Box::new_in(sup, self.alloc),
                        }
                    } else {
                        Node::Subscript(
                            Box::new_in(Node::Operator(int, None), self.alloc),
                            Box::new_in(sub, self.alloc),
                        )
                    }
                }
                Token::Circumflex => {
                    self.next_token(); // Discard the circumflex token.
                    let sup = self.parse_single_token()?;
                    if self.peek_token_is(Token::Underscore) {
                        self.next_token(); // Discard the underscore token.
                        let sub = self.parse_single_token()?;
                        Node::SubSup {
                            target: Box::new_in(Node::Operator(int, None), self.alloc),
                            sub: Box::new_in(sub, self.alloc),
                            sup: Box::new_in(sup, self.alloc),
                        }
                    } else {
                        Node::Superscript(
                            Box::new_in(Node::Operator(int, None), self.alloc),
                            Box::new_in(sup, self.alloc),
                        )
                    }
                }
                _ => Node::Operator(int, None),
            },
            Token::Colon => match self.peek_token {
                Token::Operator(ops::EQUAL | ops::EQUIV) => {
                    let Token::Operator(op) = self.next_token() else {
                        // We have just verified that the next token is an operator.
                        unreachable!()
                    };
                    Node::PseudoRow(bumpalo::vec![
                        in self.alloc;
                        Node::OperatorWithSpacing {
                            op: ops::COLON,
                            stretchy: None,
                            left: Some("0.2222"),
                            right: Some("0"),
                        },
                        Node::OperatorWithSpacing {
                            op,
                            stretchy: None,
                            left: Some("0"),
                            right: None,
                        },
                    ])
                }
                _ => Node::OperatorWithSpacing {
                    op: ops::COLON,
                    stretchy: None,
                    left: Some("0.2222"),
                    right: Some("0.2222"),
                },
            },
            Token::LBrace => self.parse_group(Token::RBrace)?,
            Token::Paren(paren) => Node::Operator(paren, Some(Stretchy::False)),
            Token::Left => {
                let open = match self.next_token() {
                    Token::Paren(open) => open,
                    Token::Operator(ops::DOT) => ops::NULL,
                    token => {
                        return Err(LatexError::MissingParenthesis {
                            location: Token::Left,
                            got: token,
                        })
                    }
                };
                let content = self.parse_group(Token::Right)?;
                let close = match self.next_token() {
                    Token::Paren(close) => close,
                    Token::Operator(ops::DOT) => ops::NULL,
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
                    content: Box::new_in(content, self.alloc),
                }
            }
            Token::Middle => {
                let stretchy = Some(Stretchy::True);
                let tok = self.next_token();
                match self.parse_single_node(tok)? {
                    Node::Operator(op, _) => Node::Operator(op, stretchy),
                    _ => unimplemented!(),
                }
            }
            Token::Big(size) => match self.next_token() {
                Token::Paren(paren) => Node::SizedParen { size, paren },
                tok => {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::Paren(ops::NULL),
                        got: tok,
                    });
                }
            },
            Token::Begin => {
                // Read the environment name.
                let environment = self.parse_text_group(Token::RBrace, WhiteSpace::Record)?;
                // Read the contents of \begin..\end.
                let content = match self.parse_group(Token::End)? {
                    Node::Row(content) => content,
                    content => bumpalo::vec![in self.alloc; content],
                };
                let node = match environment.as_str() {
                    "align" | "align*" | "aligned" => Node::Table(content, Align::Alternating),
                    "cases" => Node::Fenced {
                        open: ops::LEFT_CURLY_BRACKET,
                        close: ops::NULL,
                        content: Box::new_in(Node::Table(content, Align::Left), self.alloc),
                    },
                    "matrix" => Node::Table(content, Align::Center),
                    "pmatrix" => Node::Fenced {
                        open: ops::LEFT_PARENTHESIS,
                        close: ops::RIGHT_PARENTHESIS,
                        content: Box::new_in(Node::Table(content, Align::Center), self.alloc),
                    },
                    "bmatrix" => Node::Fenced {
                        open: ops::LEFT_SQUARE_BRACKET,
                        close: ops::RIGHT_SQUARE_BRACKET,
                        content: Box::new_in(Node::Table(content, Align::Center), self.alloc),
                    },
                    "vmatrix" => Node::Fenced {
                        open: ops::VERTICAL_LINE,
                        close: ops::VERTICAL_LINE,
                        content: Box::new_in(Node::Table(content, Align::Center), self.alloc),
                    },
                    _ => {
                        return Err(LatexError::UnknownEnvironment(environment));
                    }
                };
                let end_name = self.parse_text_group(Token::RBrace, WhiteSpace::Record)?;
                if end_name != environment {
                    return Err(LatexError::MismatchedEnvironment {
                        expected: environment,
                        got: end_name,
                    });
                }

                node
            }
            Token::OperatorName => {
                if !self.peek_token_is(Token::LBrace) {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::LBrace,
                        got: self.next_token(),
                    });
                }
                // Read the function name.
                let function = self.parse_text_group(Token::RBrace, WhiteSpace::Skip)?;
                Node::MultiLetterIdent(function, None)
            }
            Token::Text => {
                if !self.peek_token_is(Token::LBrace) {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::LBrace,
                        got: self.next_token(),
                    });
                }
                // Read the text.
                let text = self.parse_text_group(Token::RBrace, WhiteSpace::Record)?;
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Mathstrut => Node::Phantom(
                Box::new_in(
                    Node::OperatorWithSpacing {
                        op: ops::LEFT_PARENTHESIS,
                        stretchy: Some(Stretchy::False),
                        left: Some("0"),
                        right: Some("0"),
                    },
                    self.alloc,
                ),
                PhantomWidth::Zero,
            ),
            Token::UnknownCommand(name) => {
                return Err(LatexError::UnknownCommand(name));
            }
            Token::Underscore => {
                return Err(LatexError::InvalidCharacter {
                    expected: "identifier",
                    got: '_',
                });
            }
            Token::Circumflex => {
                return Err(LatexError::InvalidCharacter {
                    expected: "identifier",
                    got: '^',
                });
            }
            Token::Null => {
                unreachable!()
            }
            Token::EOF => return Err(LatexError::UnexpectedEOF),
            tok @ (Token::End | Token::Right | Token::RBrace) => {
                return Err(LatexError::UnexpectedClose(tok))
            }
        };

        match self.peek_token {
            Token::Operator(ops::APOS) => {
                self.next_token(); // Discard the apostrophe token.
                Ok(Node::Superscript(
                    Box::new_in(node, self.alloc),
                    Box::new_in(Node::Operator(ops::PRIME, None), self.alloc),
                ))
            }
            _ => Ok(node),
        }
    }

    #[inline]
    fn parse_token(&mut self) -> Result<Node<'a>, LatexError<'a>> {
        let token = self.next_token();
        self.parse_node(token)
    }

    #[inline]
    fn parse_single_token(&mut self) -> Result<Node<'a>, LatexError<'a>> {
        let token = self.next_token();
        self.parse_single_node(token)
    }

    fn parse_group(&mut self, end_token: Token<'a>) -> Result<Node<'a>, LatexError<'a>> {
        let mut cur_token = self.next_token();
        let mut nodes = Vec::new_in(self.alloc);

        while {
            if cur_token == Token::EOF {
                // When the input is completed without closed parentheses.
                return Err(LatexError::UnexpectedToken {
                    expected: end_token,
                    got: cur_token,
                });
            }

            cur_token != end_token
        } {
            nodes.push(self.parse_node(cur_token)?);
            cur_token = self.next_token();
        }

        if nodes.len() == 1 {
            // SAFETY: `nodes` is not empty.
            unsafe { Ok(nodes.into_iter().next().unwrap_unchecked()) }
        } else {
            Ok(Node::Row(nodes))
        }
    }

    fn parse_text_group(
        &mut self,
        end_token: Token<'a>,
        whitespace: WhiteSpace,
    ) -> Result<String<'a>, LatexError<'a>> {
        // We immediately start recording whitespace, so that the peek token field
        // can be filled with whitespace characters.
        self.l.record_whitespace = matches!(whitespace, WhiteSpace::Record);
        self.next_token(); // Discard the opening brace token.

        let mut text = String::new_in(self.alloc);

        loop {
            match &self.peek_token {
                Token::Letter(x) | Token::NormalLetter(x) => {
                    text.push(*x); // Copy the character.
                    self.next_token(); // Discard the token.
                }
                _ => {
                    // We turn off the whitespace recording here because we don't want to put
                    // any whitespace chracters into the peek token field.
                    self.l.record_whitespace = false;
                    // Get whatever non-letter token is next.
                    // (We know it is not a letter because we matched on the peek token.)
                    let non_letter_tok = self.next_token();
                    if non_letter_tok == end_token {
                        break; // Everything is fine.
                    } else {
                        return Err(LatexError::UnexpectedToken {
                            expected: end_token,
                            got: non_letter_tok,
                        });
                    }
                }
            }
        }

        Ok(text)
    }

    fn set_variant(&self, node: Node<'a>, var: MathVariant) -> Node<'a> {
        match node {
            Node::SingleLetterIdent(x, _) => Node::SingleLetterIdent(x, Some(var)),
            Node::MultiLetterIdent(x, _) => Node::MultiLetterIdent(x, Some(var)),
            Node::Row(vec) => Node::Row(
                vec.into_iter()
                    .map(|node| self.set_variant(node, var.clone()))
                    .collect_in(self.alloc),
            ),
            node => node,
        }
    }

    fn transform_text(&self, node: Node<'a>, var: TextTransform) -> Node<'a> {
        match node {
            Node::SingleLetterIdent(x, _) => Node::SingleLetterIdent(var.transform(x), None),
            Node::MultiLetterIdent(letters, _) => Node::MultiLetterIdent(
                letters
                    .chars()
                    .map(|c| var.transform(c))
                    .collect_in(self.alloc),
                None,
            ),
            Node::Operator(op, _) => Node::SingleLetterIdent(var.transform(op.into_char()), None),
            Node::Row(vec) => Node::Row(
                vec.into_iter()
                    .map(|node| self.transform_text(node, var))
                    .collect_in(self.alloc),
            ),
            node => node,
        }
    }

    fn merge_single_letters(&self, nodes: Vec<'a, Node<'a>>) -> Node<'a> {
        let mut new_nodes = Vec::new_in(self.alloc);
        let mut collected: Option<String> = None;
        for node in nodes {
            if let Node::SingleLetterIdent(c, _) = node {
                if let Some(ref mut letters) = collected {
                    letters.push(c); // we add another single letter
                } else {
                    let mut buf = String::new_in(self.alloc);
                    buf.push(c);
                    collected = Some(buf); // we start collecting
                }
            } else {
                if let Some(letters) = collected.take() {
                    new_nodes.push(Node::MultiLetterIdent(letters, None));
                }
                new_nodes.push(node);
            }
        }
        if let Some(letters) = collected {
            new_nodes.push(Node::MultiLetterIdent(letters, None));
        }
        if new_nodes.len() == 1 {
            // SAFETY: `new_nodes` is not empty.
            unsafe { new_nodes.into_iter().next().unwrap_unchecked() }
        } else {
            Node::Row(new_nodes)
        }
    }
}

enum WhiteSpace {
    Skip,
    Record,
}
