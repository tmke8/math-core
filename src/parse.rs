use super::{
    ast::Node,
    attribute::{LineThickness, MathVariant, TextTransform},
    error::LatexError,
    lexer::Lexer,
    token::{Op, Token},
};

#[derive(Debug, Clone)]
pub(crate) struct Parser<'a> {
    l: Lexer<'a>,
    cur_token: Token,
    peek_token: Token,
}
impl<'a> Parser<'a> {
    pub(crate) fn new(l: Lexer<'a>) -> Self {
        let mut p = Parser {
            l,
            cur_token: Token::Illegal('\u{0}'),
            peek_token: Token::Illegal('\u{0}'),
        };
        p.next_token();
        p.next_token();
        p
    }

    fn next_token(&mut self) {
        self.cur_token = self.peek_token.clone();
        self.peek_token = if self.cur_token.acts_on_a_digit() && self.l.cur.is_ascii_digit() {
            let num = self.l.cur;
            self.l.read_char();
            Token::Number(format!("{}", num))
        } else {
            self.l.next_token()
        };
    }

    fn cur_token_is(&self, expected_token: &Token) -> bool {
        &self.cur_token == expected_token
    }

    fn peek_token_is(&self, expected_token: Token) -> bool {
        self.peek_token == expected_token
    }

    pub(crate) fn parse(&mut self) -> Result<Vec<Node>, LatexError> {
        let mut nodes = Vec::new();

        while !self.cur_token_is(&Token::EOF) {
            nodes.push(self.parse_node()?);
            self.next_token();
        }

        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<Node, LatexError> {
        let left = self.parse_single_node()?;

        match self.peek_token {
            Token::Underscore => {
                self.next_token();
                self.next_token();
                let right = self.parse_node()?;
                Ok(Node::Subscript(Box::new(left), Box::new(right)))
            }
            Token::Circumflex => {
                self.next_token();
                self.next_token();
                let right = self.parse_node()?;
                Ok(Node::Superscript(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    // 中置演算子 `_`, `^`, '\'' が続くかどうかを気にせずに, 直後のノードを読む
    //
    // 注) 中置演算子を考慮して正しくノードを読む場合は `parse_node()` を使う.
    fn parse_single_node(&mut self) -> Result<Node, LatexError> {
        let node = match &self.cur_token {
            Token::Number(number) => Node::Number(number.clone()),
            Token::Letter(x, v) => Node::SingleLetterIdent(*x, *v),
            Token::Operator(op) => Node::Operator(*op),
            Token::Function(fun) => Node::MultiLetterIdent(fun.to_string(), None),
            Token::Space(space) => Node::Space(*space),
            Token::NonBreakingSpace => Node::Text("\u{A0}".to_string()),
            Token::Sqrt => {
                self.next_token();
                if self.cur_token_is(&Token::Paren(Op('['))) {
                    let degree = self.parse_group(&Token::Paren(Op(']')))?;
                    self.next_token();
                    let content = self.parse_node()?;
                    Node::Root(Box::new(degree), Box::new(content))
                } else {
                    let content = self.parse_node()?;
                    Node::Sqrt(Box::new(content))
                }
            }
            Token::Frac(displaystyle) => {
                let displaystyle = *displaystyle;
                self.next_token();
                let numerator = self.parse_node()?;
                self.next_token();
                let denominator = self.parse_node()?;
                Node::Frac(
                    Box::new(numerator),
                    Box::new(denominator),
                    LineThickness::Medium,
                    displaystyle,
                )
            }
            Token::Binom(displaystyle) => {
                let displaystyle = *displaystyle;
                self.next_token();
                let numerator = self.parse_node()?;
                self.next_token();
                let denominator = self.parse_node()?;

                Node::Fenced {
                    open: Op('('),
                    close: Op(')'),
                    content: Box::new(Node::Frac(
                        Box::new(numerator),
                        Box::new(denominator),
                        LineThickness::Length(0),
                        displaystyle,
                    )),
                }
            }
            Token::Over(op, acc) => {
                let (op, acc) = (*op, *acc);
                self.next_token();
                let target = self.parse_node()?;
                Node::OverOp(op, acc, Box::new(target))
            }
            Token::Under(op, acc) => {
                let (op, acc) = (*op, *acc);
                self.next_token();
                let target = self.parse_node()?;
                Node::UnderOp(op, acc, Box::new(target))
            }
            Token::Overset => {
                self.next_token();
                let over = self.parse_node()?;
                self.next_token();
                let target = self.parse_node()?;
                Node::Overset {
                    over: Box::new(over),
                    target: Box::new(target),
                }
            }
            Token::Underset => {
                self.next_token();
                let under = self.parse_node()?;
                self.next_token();
                let target = self.parse_node()?;
                Node::Underset {
                    under: Box::new(under),
                    target: Box::new(target),
                }
            }
            Token::Overbrace(x) => {
                let x = *x;
                self.next_token();
                let target = self.parse_single_node()?;
                if self.peek_token_is(Token::Circumflex) {
                    self.next_token();
                    self.next_token();
                    let expl = self.parse_single_node()?;
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
                let x = *x;
                self.next_token();
                let target = self.parse_single_node()?;
                if self.peek_token_is(Token::Underscore) {
                    self.next_token();
                    self.next_token();
                    let expl = self.parse_single_node()?;
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
            Token::BigOp(op) => {
                let op = *op;
                match self.peek_token {
                    Token::Underscore => {
                        self.next_token();
                        self.next_token();
                        let under = self.parse_single_node()?;
                        if self.peek_token_is(Token::Circumflex) {
                            self.next_token();
                            self.next_token();
                            let over = self.parse_single_node()?;
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
                        self.next_token();
                        self.next_token();
                        let over = self.parse_single_node()?;
                        if self.peek_token_is(Token::Underscore) {
                            self.next_token();
                            self.next_token();
                            let under = self.parse_single_node()?;
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
                }
            }
            Token::Lim(lim) => {
                let lim = Node::MultiLetterIdent(lim.to_string(), None);
                if self.peek_token_is(Token::Underscore) {
                    self.next_token();
                    self.next_token();
                    let under = self.parse_single_node()?;
                    Node::Underset {
                        target: Box::new(lim),
                        under: Box::new(under),
                    }
                } else {
                    lim
                }
            }
            Token::Slashed => {
                self.next_token();
                self.next_token();
                let node = self.parse_node()?;
                self.next_token();
                Node::Slashed(Box::new(node))
            }
            Token::NormalVariant => {
                self.next_token();
                let node = self.parse_node()?;
                let node = if let Node::Row(nodes) = node {
                    merge_single_letters(nodes)
                } else {
                    node
                };
                set_variant(node, MathVariant::Normal)
            }
            Token::Style(var) => {
                let var = *var;
                self.next_token();
                let node = self.parse_node()?;
                let node = if let Node::Row(nodes) = node {
                    merge_single_letters(nodes)
                } else {
                    node
                };
                transform_text(node, var)
            }
            Token::Integral(int) => {
                let int = *int;
                match self.peek_token {
                    Token::Underscore => {
                        self.next_token();
                        self.next_token();
                        let sub = self.parse_single_node()?;
                        if self.peek_token_is(Token::Circumflex) {
                            self.next_token();
                            self.next_token();
                            let sup = self.parse_single_node()?;
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
                        self.next_token();
                        self.next_token();
                        let sup = self.parse_single_node()?;
                        if self.peek_token_is(Token::Underscore) {
                            self.next_token();
                            self.next_token();
                            let sub = self.parse_single_node()?;
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
                }
            }
            Token::Colon => match self.peek_token {
                Token::Operator(op @ Op('=' | '≡')) => {
                    self.next_token();
                    Node::PseudoRow(vec![
                        Node::OperatorWithSpacing {
                            op: Op(':'),
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
                    op: Op(':'),
                    left: 2. / 9.,
                    right: 2. / 9.,
                },
            },
            Token::LBrace => self.parse_group(&Token::RBrace)?,
            Token::Paren(paren) => Node::Paren(*paren),
            Token::Left => {
                self.next_token();
                let open = match &self.cur_token {
                    Token::Paren(open) => *open,
                    Token::Operator(Op('.')) => Op('\u{0}'),
                    token => {
                        return Err(LatexError::MissingParenthesis {
                            location: Token::Left,
                            got: token.clone(),
                        })
                    }
                };
                let content = self.parse_group(&Token::Right)?;
                self.next_token();
                let close = match &self.cur_token {
                    Token::Paren(close) => *close,
                    Token::Operator(Op('.')) => Op('\u{0}'),
                    token => {
                        return Err(LatexError::MissingParenthesis {
                            location: Token::Right,
                            got: token.clone(),
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
                self.next_token();
                match self.parse_single_node()? {
                    Node::Operator(op) => Node::StretchedOp(stretchy, op),
                    Node::Paren(op) => Node::StretchedOp(stretchy, op),
                    _ => unimplemented!(),
                }
            }
            Token::Big(size) => {
                let size = *size;
                self.next_token();
                match self.cur_token {
                    Token::Paren(paren) => Node::SizedParen { size, paren },
                    _ => {
                        return Err(LatexError::UnexpectedToken {
                            expected: Token::Paren(Op('\u{0}')),
                            got: self.cur_token.clone(),
                        });
                    }
                }
            }
            Token::Begin => {
                self.next_token();
                // 環境名を読み込む
                let environment = self.parse_text();
                // \begin..\end の中身を読み込む
                let content = match self.parse_group(&Token::End)? {
                    Node::Row(content) => content,
                    content => vec![content],
                };
                let node = if matches!(environment.as_str(), "align" | "align*" | "aligned") {
                    Node::AlignedTable(content)
                } else {
                    let content = Node::Table(content);

                    // 環境名により処理を分岐
                    match environment.as_str() {
                        "matrix" => content,
                        "pmatrix" => Node::Fenced {
                            open: Op('('),
                            close: Op(')'),
                            content: Box::new(content),
                        },
                        "bmatrix" => Node::Fenced {
                            open: Op('['),
                            close: Op(']'),
                            content: Box::new(content),
                        },
                        "vmatrix" => Node::Fenced {
                            open: Op('|'),
                            close: Op('|'),
                            content: Box::new(content),
                        },
                        environment => {
                            return Err(LatexError::UnknownEnvironment(environment.to_owned()));
                        }
                    }
                };
                self.next_token();
                let _ = self.parse_text();

                node
            }
            Token::OperatorName => {
                self.next_token();
                // 関数名を読み込む
                let function = self.parse_text();
                Node::MultiLetterIdent(function, None)
            }
            Token::Text => {
                self.next_token();
                // テキストを読み込む
                let text = self.parse_text();
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            token => Node::Undefined(format!("{:?}", token)),
        };

        match self.peek_token {
            Token::Operator(Op('\'')) => {
                self.next_token();
                Ok(Node::Superscript(
                    Box::new(node),
                    Box::new(Node::Operator(Op('′'))),
                ))
            }
            _ => Ok(node),
        }
    }

    fn parse_group(&mut self, end_token: &Token) -> Result<Node, LatexError> {
        self.next_token();
        let mut nodes = Vec::new();

        while {
            if self.cur_token_is(&Token::EOF) {
                // 閉じ括弧がないまま入力が終了した場合
                return Err(LatexError::UnexpectedToken {
                    expected: end_token.clone(),
                    got: self.cur_token.clone(),
                });
            }

            !self.cur_token_is(end_token)
        } {
            nodes.push(self.parse_node()?);
            self.next_token();
        }

        if nodes.len() == 1 {
            let node = nodes.into_iter().nth(0).unwrap();
            Ok(node)
        } else {
            Ok(Node::Row(nodes))
        }
    }

    fn parse_text(&mut self) -> String {
        // `{` を読み飛ばす
        self.next_token();

        // テキストを読み取る
        let mut text = String::new();
        while let Token::Letter(x, _) = self.cur_token {
            text.push(x);
            self.next_token();
        }
        // 終わったら最後の `}` を cur が指した状態で抜ける

        text
    }
}

fn set_variant(node: Node, var: MathVariant) -> Node {
    match node {
        Node::SingleLetterIdent(x, _) => Node::SingleLetterIdent(x, Some(var)),
        Node::MultiLetterIdent(x, _) => Node::MultiLetterIdent(x, Some(var)),
        Node::Row(vec) => Node::Row(vec.into_iter().map(|node| set_variant(node, var)).collect()),
        node => node,
    }
}

fn transform_text(node: Node, var: TextTransform) -> Node {
    match node {
        Node::SingleLetterIdent(x, _) => Node::SingleLetterIdent(var.transform(x), None),
        Node::MultiLetterIdent(letters, _) => {
            Node::MultiLetterIdent(letters.chars().map(|c| var.transform(c)).collect(), None)
        }
        Node::Operator(Op(op)) => Node::SingleLetterIdent(var.transform(op), None),
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
        new_nodes.into_iter().nth(0).unwrap()
    } else {
        Node::Row(new_nodes)
    }
}
