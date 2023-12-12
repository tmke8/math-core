use crate::attribute::{Accent, Stretchy};

use super::{
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
    peek_token: Token,
}
impl<'a> Parser<'a> {
    pub(crate) fn new(l: Lexer<'a>) -> Self {
        let mut p = Parser {
            l,
            peek_token: Token::Null,
        };
        // Discard the null token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    fn next_token(&mut self) -> Token {
        let peek_token = if self.peek_token.acts_on_a_digit() && self.l.cur.is_ascii_digit() {
            let num = self.l.cur;
            self.l.read_char();
            Token::Number(num.to_string())
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

    pub(crate) fn parse(&mut self) -> Result<Node, LatexError> {
        let mut nodes = Vec::new();
        let mut cur_token = self.next_token();

        while cur_token != Token::EOF {
            nodes.push(self.parse_node(cur_token)?);
            cur_token = self.next_token();
        }

        if nodes.len() == 1 {
            // Safety: `nodes` is not empty.
            unsafe {
                let node = nodes.into_iter().next().unwrap_unchecked();
                Ok(node)
            }
        } else {
            Ok(Node::PseudoRow(nodes))
        }
    }

    fn parse_node(&mut self, cur_token: Token) -> Result<Node, LatexError> {
        let left = self.parse_single_node(cur_token)?;

        match self.peek_token {
            Token::Underscore => {
                self.next_token(); // Discard the underscore token.
                let right = self.parse_token()?;
                Ok(Node::Subscript(Box::new(left), Box::new(right)))
            }
            Token::Circumflex => {
                self.next_token(); // Discard the circumflex token.
                let right = self.parse_token()?;
                Ok(Node::Superscript(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    // 中置演算子 `_`, `^`, '\'' が続くかどうかを気にせずに, 直後のノードを読む
    //
    // 注) 中置演算子を考慮して正しくノードを読む場合は `parse_node()` を使う.
    fn parse_single_node(&mut self, cur_token: Token) -> Result<Node, LatexError> {
        let node = match cur_token {
            Token::Number(number) => Node::Number(number),
            Token::Letter(x) => Node::SingleLetterIdent(x, None),
            Token::NormalLetter(x) => Node::SingleLetterIdent(x, Some(MathVariant::Normal)),
            Token::Operator(op) => Node::Operator(op, None),
            Token::Function(fun) => Node::MultiLetterIdent(fun.to_string(), None),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace => Node::Text("\u{A0}".to_string()),
            Token::Sqrt => {
                let next_token = self.next_token();
                if next_token == Token::Paren(ops::LEFT_SQUARE_BRACKET) {
                    let degree = self.parse_group(Token::Paren(ops::RIGHT_SQUARE_BRACKET))?;
                    let content = self.parse_token()?;
                    Node::Root(Box::new(degree), Box::new(content))
                } else {
                    let content = self.parse_node(next_token)?;
                    Node::Sqrt(Box::new(content))
                }
            }
            Token::Frac(displaystyle) => {
                let numerator = self.parse_token()?;
                let denominator = self.parse_token()?;
                Node::Frac(
                    Box::new(numerator),
                    Box::new(denominator),
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
                    content: Box::new(Node::Frac(
                        Box::new(numerator),
                        Box::new(denominator),
                        LineThickness::Zero,
                        displaystyle,
                    )),
                }
            }
            Token::Over(op) => {
                let target = self.parse_token()?;
                Node::OverOp(op, Accent::True, Box::new(target))
            }
            Token::Under(op) => {
                let target = self.parse_token()?;
                Node::UnderOp(op, Accent::True, Box::new(target))
            }
            Token::Overset => {
                let over = self.parse_token()?;
                let target = self.parse_token()?;
                Node::Overset {
                    over: Box::new(over),
                    target: Box::new(target),
                }
            }
            Token::Underset => {
                let under = self.parse_token()?;
                let target = self.parse_token()?;
                Node::Underset {
                    under: Box::new(under),
                    target: Box::new(target),
                }
            }
            Token::Overbrace(x) => {
                let target = self.parse_single_token()?;
                if self.peek_token_is(Token::Circumflex) {
                    self.next_token(); // Discard the circumflex token.
                    let expl = self.parse_single_token()?;
                    let over = Node::Overset {
                        over: Box::new(expl),
                        target: Box::new(Node::Operator(x, None)),
                    };
                    Node::Overset {
                        over: Box::new(over),
                        target: Box::new(target),
                    }
                } else {
                    Node::Overset {
                        over: Box::new(Node::Operator(x, None)),
                        target: Box::new(target),
                    }
                }
            }
            Token::Underbrace(x) => {
                let target = self.parse_single_token()?;
                if self.peek_token_is(Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let expl = self.parse_single_token()?;
                    let under = Node::Underset {
                        under: Box::new(expl),
                        target: Box::new(Node::Operator(x, None)),
                    };
                    Node::Underset {
                        under: Box::new(under),
                        target: Box::new(target),
                    }
                } else {
                    Node::Underset {
                        under: Box::new(Node::Operator(x, None)),
                        target: Box::new(target),
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
                            target: Box::new(Node::Operator(op, None)),
                            under: Box::new(under),
                            over: Box::new(over),
                        }
                    } else {
                        Node::Underset {
                            target: Box::new(Node::Operator(op, None)),
                            under: Box::new(under),
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
                            target: Box::new(Node::Operator(op, None)),
                            under: Box::new(under),
                            over: Box::new(over),
                        }
                    } else {
                        Node::Overset {
                            over: Box::new(Node::Operator(op, None)),
                            target: Box::new(over),
                        }
                    }
                }
                _ => Node::Operator(op, None),
            },
            Token::Lim(lim) => {
                let lim = Node::MultiLetterIdent(lim.to_string(), None);
                if self.peek_token_is(Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    Node::Underset {
                        target: Box::new(lim),
                        under: Box::new(under),
                    }
                } else {
                    lim
                }
            }
            Token::Slashed => {
                self.next_token(); // Optimistically skip the next token.
                let node = self.parse_token()?;
                self.next_token(); // Optimistically skip the next token.
                Node::Slashed(Box::new(node))
            }
            Token::NormalVariant => {
                let node = self.parse_token()?;
                let node = if let Node::Row(nodes) = node {
                    merge_single_letters(nodes)
                } else {
                    node
                };
                set_variant(node, MathVariant::Normal)
            }
            Token::Style(var) => {
                let node = self.parse_token()?;
                let node = if let Node::Row(nodes) = node {
                    merge_single_letters(nodes)
                } else {
                    node
                };
                transform_text(node, var)
            }
            Token::Integral(int) => match self.peek_token {
                Token::Underscore => {
                    self.next_token(); // Discard the underscore token.
                    let sub = self.parse_single_token()?;
                    if self.peek_token_is(Token::Circumflex) {
                        self.next_token(); // Discard the circumflex token.
                        let sup = self.parse_single_token()?;
                        Node::SubSup {
                            target: Box::new(Node::Operator(int, None)),
                            sub: Box::new(sub),
                            sup: Box::new(sup),
                        }
                    } else {
                        Node::Subscript(Box::new(Node::Operator(int, None)), Box::new(sub))
                    }
                }
                Token::Circumflex => {
                    self.next_token(); // Discard the circumflex token.
                    let sup = self.parse_single_token()?;
                    if self.peek_token_is(Token::Underscore) {
                        self.next_token(); // Discard the underscore token.
                        let sub = self.parse_single_token()?;
                        Node::SubSup {
                            target: Box::new(Node::Operator(int, None)),
                            sub: Box::new(sub),
                            sup: Box::new(sup),
                        }
                    } else {
                        Node::Superscript(Box::new(Node::Operator(int, None)), Box::new(sup))
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
                    Node::PseudoRow(vec![
                        Node::OperatorWithSpacing {
                            op: ops::COLON,
                            left: Some("0.2222"),
                            right: Some("0"),
                        },
                        Node::OperatorWithSpacing {
                            op,
                            left: Some("0"),
                            right: None,
                        },
                    ])
                }
                _ => Node::OperatorWithSpacing {
                    op: ops::COLON,
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
                    content: Box::new(content),
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
                // 環境名を読み込む
                let environment = self.parse_text_group(Token::RBrace, WhiteSpace::Record)?;
                // \begin..\end の中身を読み込む
                let content = match self.parse_group(Token::End)? {
                    Node::Row(content) => content,
                    content => vec![content],
                };
                let node = match environment.as_str() {
                    "align" | "align*" | "aligned" => Node::Table(content, Align::Alternating),
                    "cases" => Node::Fenced {
                        open: ops::LEFT_CURLY_BRACKET,
                        close: ops::NULL,
                        content: Box::new(Node::Table(content, Align::Left)),
                    },
                    "matrix" => Node::Table(content, Align::Center),
                    "pmatrix" => Node::Fenced {
                        open: ops::LEFT_PARENTHESIS,
                        close: ops::RIGHT_PARENTHESIS,
                        content: Box::new(Node::Table(content, Align::Center)),
                    },
                    "bmatrix" => Node::Fenced {
                        open: ops::LEFT_SQUARE_BRACKET,
                        close: ops::RIGHT_SQUARE_BRACKET,
                        content: Box::new(Node::Table(content, Align::Center)),
                    },
                    "vmatrix" => Node::Fenced {
                        open: ops::VERTICAL_LINE,
                        close: ops::VERTICAL_LINE,
                        content: Box::new(Node::Table(content, Align::Center)),
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
                // 関数名を読み込む
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
                // テキストを読み込む
                let text = self.parse_text_group(Token::RBrace, WhiteSpace::Record)?;
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
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
                    Box::new(node),
                    Box::new(Node::Operator(ops::PRIME, None)),
                ))
            }
            _ => Ok(node),
        }
    }

    #[inline]
    fn parse_token(&mut self) -> Result<Node, LatexError> {
        let token = self.next_token();
        self.parse_node(token)
    }

    #[inline]
    fn parse_single_token(&mut self) -> Result<Node, LatexError> {
        let token = self.next_token();
        self.parse_single_node(token)
    }

    fn parse_group(&mut self, end_token: Token) -> Result<Node, LatexError> {
        let mut cur_token = self.next_token();
        let mut nodes = Vec::new();

        while {
            if cur_token == Token::EOF {
                // 閉じ括弧がないまま入力が終了した場合
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
            // Safety: `nodes` is not empty.
            unsafe {
                let node = nodes.into_iter().next().unwrap_unchecked();
                Ok(node)
            }
        } else {
            Ok(Node::Row(nodes))
        }
    }

    fn parse_text_group(
        &mut self,
        end_token: Token,
        whitespace: WhiteSpace,
    ) -> Result<String, LatexError> {
        // We immediately start recording whitespace, so that the peek token field
        // can be filled with whitespace characters.
        self.l.record_whitespace = matches!(whitespace, WhiteSpace::Record);
        self.next_token(); // Discard the opening brace token.

        let mut text = String::new();

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
}

enum WhiteSpace {
    Skip,
    Record,
}

fn set_variant(node: Node, var: MathVariant) -> Node {
    match node {
        Node::SingleLetterIdent(x, _) => Node::SingleLetterIdent(x, Some(var)),
        Node::MultiLetterIdent(x, _) => Node::MultiLetterIdent(x, Some(var)),
        Node::Row(vec) => Node::Row(
            vec.into_iter()
                .map(|node| set_variant(node, var.clone()))
                .collect(),
        ),
        node => node,
    }
}

fn transform_text(node: Node, var: TextTransform) -> Node {
    match node {
        Node::SingleLetterIdent(x, _) => Node::SingleLetterIdent(var.transform(x), None),
        Node::MultiLetterIdent(letters, _) => {
            Node::MultiLetterIdent(letters.chars().map(|c| var.transform(c)).collect(), None)
        }
        Node::Operator(op, _) => Node::SingleLetterIdent(
            var.transform(op.str().chars().next().unwrap_or('\u{0}')),
            None,
        ),
        Node::Row(vec) => Node::Row(
            vec.into_iter()
                .map(|node| transform_text(node, var))
                .collect(),
        ),
        node => node,
    }
}

fn merge_single_letters(nodes: Vec<Node>) -> Node {
    let mut new_nodes = Vec::new();
    let mut collected: Option<String> = None;
    for node in nodes {
        if let Node::SingleLetterIdent(c, _) = node {
            if let Some(ref mut letters) = collected {
                letters.push(c); // we add another single letter
            } else {
                collected = Some(c.to_string()); // we start collecting
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
        // Safety: `new_nodes` is not empty.
        unsafe { new_nodes.into_iter().next().unwrap_unchecked() }
    } else {
        Node::Row(new_nodes)
    }
}
