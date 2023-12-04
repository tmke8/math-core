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
            Token::Number(format!("{}", num))
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
            let node = nodes.into_iter().next().unwrap();
            Ok(node)
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
            Token::Letter(x, v) => Node::SingleLetterIdent(x, v),
            Token::Operator(op) => Node::Operator(op),
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
                        LineThickness::Length(0),
                        displaystyle,
                    )),
                }
            }
            Token::Over(op, acc) => {
                let target = self.parse_token()?;
                Node::OverOp(op, acc, Box::new(target))
            }
            Token::Under(op, acc) => {
                let target = self.parse_token()?;
                Node::UnderOp(op, acc, Box::new(target))
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
                        target: Box::new(Node::Operator(x)),
                    };
                    Node::Overset {
                        over: Box::new(over),
                        target: Box::new(target),
                    }
                } else {
                    Node::Overset {
                        over: Box::new(Node::Operator(x)),
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
                        target: Box::new(Node::Operator(x)),
                    };
                    Node::Underset {
                        under: Box::new(under),
                        target: Box::new(target),
                    }
                } else {
                    Node::Underset {
                        under: Box::new(Node::Operator(x)),
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
                            target: Box::new(Node::Operator(op)),
                            under: Box::new(under),
                            over: Box::new(over),
                        }
                    } else {
                        Node::Underset {
                            target: Box::new(Node::Operator(op)),
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
                            target: Box::new(Node::Operator(op)),
                            under: Box::new(under),
                            over: Box::new(over),
                        }
                    } else {
                        Node::Overset {
                            over: Box::new(Node::Operator(op)),
                            target: Box::new(over),
                        }
                    }
                }
                _ => Node::Operator(op),
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
                            target: Box::new(Node::Operator(int)),
                            sub: Box::new(sub),
                            sup: Box::new(sup),
                        }
                    } else {
                        Node::Subscript(Box::new(Node::Operator(int)), Box::new(sub))
                    }
                }
                Token::Circumflex => {
                    self.next_token(); // Discard the circumflex token.
                    let sup = self.parse_single_token()?;
                    if self.peek_token_is(Token::Underscore) {
                        self.next_token(); // Discard the underscore token.
                        let sub = self.parse_single_token()?;
                        Node::SubSup {
                            target: Box::new(Node::Operator(int)),
                            sub: Box::new(sub),
                            sup: Box::new(sup),
                        }
                    } else {
                        Node::Superscript(Box::new(Node::Operator(int)), Box::new(sup))
                    }
                }
                _ => Node::Operator(int),
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
                            left: 2. / 9.,
                            right: 0.,
                        },
                        Node::OperatorWithSpacing {
                            op,
                            left: 0.,
                            right: f32::NAN,
                        },
                    ])
                }
                _ => Node::OperatorWithSpacing {
                    op: ops::COLON,
                    left: 2. / 9.,
                    right: 2. / 9.,
                },
            },
            Token::LBrace => self.parse_group(Token::RBrace)?,
            Token::Paren(paren) => Node::Paren(paren),
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
                let stretchy = true;
                let tok = self.next_token();
                match self.parse_single_node(tok)? {
                    Node::Operator(op) => Node::StretchedOp(stretchy, op),
                    Node::Paren(op) => Node::StretchedOp(stretchy, op),
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
                self.next_token();
                // 環境名を読み込む
                let environment = self.parse_text(WhiteSpace::Skip);
                // \begin..\end の中身を読み込む
                let content = match self.parse_group(Token::End)? {
                    Node::Row(content) => content,
                    content => vec![content],
                };
                let node = if matches!(environment.as_str(), "align" | "align*" | "aligned") {
                    Node::Table(content, Align::Alternating)
                } else if environment == "cases" {
                    Node::Fenced {
                        open: ops::LEFT_CURLY_BRACKET,
                        close: ops::NULL,
                        content: Box::new(Node::Table(content, Align::Left)),
                    }
                } else {
                    let content = Node::Table(content, Align::Center);

                    // 環境名により処理を分岐
                    match environment.as_str() {
                        "matrix" => content,
                        "pmatrix" => Node::Fenced {
                            open: ops::LEFT_PARENTHESIS,
                            close: ops::RIGHT_PARENTHESIS,
                            content: Box::new(content),
                        },
                        "bmatrix" => Node::Fenced {
                            open: ops::LEFT_SQUARE_BRACKET,
                            close: ops::RIGHT_SQUARE_BRACKET,
                            content: Box::new(content),
                        },
                        "vmatrix" => Node::Fenced {
                            open: ops::VERTICAL_LINE,
                            close: ops::VERTICAL_LINE,
                            content: Box::new(content),
                        },
                        environment => {
                            return Err(LatexError::UnknownEnvironment(environment.to_owned()));
                        }
                    }
                };
                self.next_token();
                let _ = self.parse_text(WhiteSpace::Skip);

                node
            }
            Token::OperatorName => {
                let next_token = self.next_token();
                if !matches!(next_token, Token::LBrace) {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::LBrace,
                        got: next_token,
                    });
                }
                // 関数名を読み込む
                let function = self.parse_text(WhiteSpace::Skip);
                Node::MultiLetterIdent(function, None)
            }
            Token::Text => {
                let next_token = self.next_token();
                if !matches!(next_token, Token::LBrace) {
                    return Err(LatexError::UnexpectedToken {
                        expected: Token::LBrace,
                        got: next_token,
                    });
                }
                // テキストを読み込む
                let text = self.parse_text(WhiteSpace::Record);
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Null | Token::EOF | Token::Underscore | Token::Circumflex => {
                unreachable!()
            }
            Token::UnknownCommand(name) => {
                return Err(LatexError::UnknownCommand(name));
            }
            tok @ (Token::End | Token::Right | Token::RBrace) => {
                return Err(LatexError::UnexpectedClose(tok))
            }
        };

        match self.peek_token {
            Token::Operator(ops::APOS) => {
                self.next_token(); // Discard the apostrophe token.
                Ok(Node::Superscript(
                    Box::new(node),
                    Box::new(Node::Operator(ops::PRIME)),
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
            let node = nodes.into_iter().next().unwrap();
            Ok(node)
        } else {
            Ok(Node::Row(nodes))
        }
    }

    fn parse_text(&mut self, whitespace: WhiteSpace) -> String {
        // `{` を読み飛ばす
        let mut cur_token = self.next_token();

        self.l.record_whitespace = matches!(whitespace, WhiteSpace::Record);

        // テキストを読み取る
        let mut text = String::new();
        while let Token::Letter(x, _) = cur_token {
            text.push(x);
            cur_token = self.next_token();
        }
        // 終わったら最後の `}` を cur が指した状態で抜ける
        self.l.record_whitespace = false;

        text
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
        Node::Operator(op) => Node::SingleLetterIdent(var.transform(op.into_char()), None),
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
        new_nodes.into_iter().next().unwrap()
    } else {
        Node::Row(new_nodes)
    }
}
