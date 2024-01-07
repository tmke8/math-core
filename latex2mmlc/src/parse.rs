use std::mem;

use crate::{
    ast::Node,
    attribute::{Accent, Align, LineThickness, MathVariant, OpAttr, PhantomWidth, TextTransform},
    error::LatexError,
    lexer::Lexer,
    ops,
    token::Token,
};

#[derive(Debug)]
pub(crate) struct Parser<'a> {
    l: Lexer<'a>,
    peek_token: Token<'a>,
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

    fn next_token(&mut self) -> Token<'a> {
        let peek_token = self.l.next_token(self.peek_token.acts_on_a_digit());
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek_token, peek_token)
    }

    pub(crate) fn parse(&mut self) -> Result<Node<'a>, LatexError<'a>> {
        let mut nodes = Vec::new();
        let mut cur_token = self.next_token();

        while !matches!(cur_token, Token::EOF) {
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

    // Read the node immediately after without worrying about whether
    // the infix operator `_`, `^`, '\' '' will continue
    //
    // Note: Use `parse_node()` when reading nodes correctly in
    // consideration of infix operators.
    fn parse_single_node(&mut self, cur_token: Token<'a>) -> Result<Node<'a>, LatexError<'a>> {
        let node = match cur_token {
            Token::Number(number, op) => match op {
                ops::NULL => Node::Number(number),
                op => Node::PseudoRow(vec![Node::Number(number), Node::Operator(op, None)]),
            },
            Token::Letter(x) => Node::SingleLetterIdent(x, None),
            Token::NormalLetter(x) => Node::SingleLetterIdent(x, Some(MathVariant::Normal)),
            Token::Operator(op) => Node::Operator(op, None),
            Token::OpGreaterThan => Node::OpGreaterThan,
            Token::OpLessThan => Node::OpLessThan,
            Token::OpAmpersand => Node::OpAmpersand,
            Token::Function(fun) => Node::MultiLetterIdent(fun.to_string(), None),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace => Node::Text("\u{A0}"),
            Token::Sqrt => {
                let next_token = self.next_token();
                if matches!(next_token, Token::Paren(ops::LEFT_SQUARE_BRACKET)) {
                    let degree = self.parse_group(Token::Paren(ops::RIGHT_SQUARE_BRACKET))?;
                    let content = self.parse_token()?;
                    Node::Root(Box::new(squeeze(degree)), Box::new(content))
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
            ref tok @ (Token::Over(op) | Token::Under(op)) => {
                let target = self.parse_token()?;
                let boxed = Box::new(target);
                if matches!(tok, Token::Over(_)) {
                    Node::OverOp(op, Accent::True, boxed)
                } else {
                    Node::UnderOp(op, Accent::True, boxed)
                }
            }
            tok @ (Token::Overset | Token::Underset) => {
                let symbol = self.parse_token()?;
                let target = self.parse_token()?;
                let symbol = Box::new(symbol);
                let target = Box::new(target);
                if matches!(tok, Token::Overset) {
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
                    let expl = Box::new(self.parse_single_token()?);
                    let op = Box::new(Node::Operator(x, None));
                    let target = Box::new(target);
                    if is_over {
                        let symbol = Box::new(Node::Overset {
                            symbol: expl,
                            target: op,
                        });
                        Node::Overset { symbol, target }
                    } else {
                        let symbol = Box::new(Node::Underset {
                            symbol: expl,
                            target: op,
                        });
                        Node::Underset { symbol, target }
                    }
                } else {
                    let symbol = Box::new(Node::Operator(x, None));
                    let target = Box::new(target);
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
                    Box::new(Node::Operator(op, Some(OpAttr::NoMovableLimits)))
                } else {
                    Box::new(Node::Operator(op, None))
                };
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
                let lim = Node::MultiLetterIdent(lim.to_string(), None);
                if matches!(self.peek_token, Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    Node::Underset {
                        target: Box::new(lim),
                        symbol: Box::new(under),
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
                let node = self.parse_single_token()?;
                let node = if let Node::Row(nodes) = node {
                    merge_single_letters(nodes)
                } else {
                    node
                };
                transform_text(node, var)
            }
            Token::Integral(int) => {
                if matches!(self.peek_token, Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    let target = Box::new(Node::Operator(int, None));
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
                    let target = Box::new(Node::Operator(int, None));
                    match self.get_bounds()? {
                        Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                        Bounds(Some(symbol), None) => Node::Subscript(target, symbol),
                        Bounds(None, Some(symbol)) => Node::Superscript(target, symbol),
                        Bounds(None, None) => Node::Operator(int, None),
                    }
                }
            }
            Token::Colon => match &self.peek_token {
                Token::Operator(op @ (ops::EQUAL | ops::EQUIV)) => {
                    let op = *op;
                    self.next_token(); // Discard the operator token.
                    Node::PseudoRow(vec![
                        Node::OperatorWithSpacing {
                            op: ops::COLON,
                            attrs: None,
                            left: Some("0.2222"),
                            right: Some("0"),
                        },
                        Node::OperatorWithSpacing {
                            op,
                            attrs: None,
                            left: Some("0"),
                            right: None,
                        },
                    ])
                }
                _ => Node::OperatorWithSpacing {
                    op: ops::COLON,
                    attrs: None,
                    left: Some("0.2222"),
                    right: Some("0.2222"),
                },
            },
            Token::LBrace => squeeze(self.parse_group(Token::RBrace)?),
            Token::Paren(paren) => Node::Operator(paren, Some(OpAttr::StretchyFalse)),
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
                    content: Box::new(squeeze(content)),
                }
            }
            Token::Middle => match self.next_token() {
                Token::Operator(op) | Token::Paren(op) => {
                    Node::Operator(op, Some(OpAttr::StretchyTrue))
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
                    "cases" => Node::Fenced {
                        open: ops::LEFT_CURLY_BRACKET,
                        close: ops::NULL,
                        content: Box::new(self.parse_table(Align::Left)?),
                    },
                    "matrix" => self.parse_table(Align::Center)?,
                    matrix_variant @ ("pmatrix" | "bmatrix" | "vmatrix") => {
                        let content = Box::new(self.parse_table(Align::Center)?);
                        let (open, close) = match matrix_variant {
                            "pmatrix" => (ops::LEFT_PARENTHESIS, ops::RIGHT_PARENTHESIS),
                            "bmatrix" => (ops::LEFT_SQUARE_BRACKET, ops::RIGHT_SQUARE_BRACKET),
                            "vmatrix" => (ops::VERTICAL_LINE, ops::VERTICAL_LINE),
                            _ => return Err(LatexError::UnknownEnvironment(matrix_variant)),
                        };
                        Node::Fenced {
                            open,
                            close,
                            content,
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
                self.check_lbrace()?;
                // Read the function name.
                let name = self.parse_text_group()?;
                // Filter out whitespace characters.
                Node::MultiLetterIdent(
                    name.chars().filter(|c| !c.is_ascii_whitespace()).collect(),
                    None,
                )
            }
            Token::Text => {
                self.check_lbrace()?;
                // Read the text.
                let text = self.parse_text_group()?;
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Mathstrut => Node::Phantom(
                Box::new(Node::OperatorWithSpacing {
                    op: ops::LEFT_PARENTHESIS,
                    attrs: Some(OpAttr::StretchyFalse),
                    left: Some("0"),
                    right: Some("0"),
                }),
                PhantomWidth::Zero,
            ),
            Token::UnknownCommand(name) => {
                return Err(LatexError::UnknownCommand(name));
            }
            tok @ (Token::Underscore | Token::Circumflex) => {
                return Err(LatexError::CannotBeUsedHere {
                    got: tok,
                    correct_place: "after an identifier or operator",
                });
            }
            tok @ Token::Limits => {
                return Err(LatexError::CannotBeUsedHere {
                    got: tok,
                     correct_place: "after \\int, \\sum, ...",
                })
            }
            Token::EOF | Token::Null => return Err(LatexError::UnexpectedEOF),
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
    fn parse_token(&mut self) -> Result<Node<'a>, LatexError<'a>> {
        let token = self.next_token();
        self.parse_node(token)
    }

    #[inline]
    fn parse_single_token(&mut self) -> Result<Node<'a>, LatexError<'a>> {
        let token = self.next_token();
        self.parse_single_node(token)
    }

    /// Parse the contents of a group which can contain any expression.
    fn parse_group(&mut self, end_token: Token<'a>) -> Result<Vec<Node<'a>>, LatexError<'a>> {
        let mut cur_token = self.next_token();
        let mut nodes = Vec::new();

        while {
            if matches!(cur_token, Token::EOF) {
                // When the input is completed without closed parentheses.
                return Err(LatexError::UnclosedGroup(end_token));
            }

            cur_token != end_token
        } {
            nodes.push(self.parse_node(cur_token)?);
            cur_token = self.next_token();
        }
        Ok(nodes)
    }

    /// Parse the contents of a group which can only contain text.
    fn parse_text_group(&mut self) -> Result<&'a str, LatexError<'a>> {
        let result = self
            .l
            .read_text_content()
            .ok_or(LatexError::UnclosedGroup(Token::RBrace));
        self.next_token(); // Discard the opening token (which is still stored as `peek`).
        result
    }

    #[inline]
    fn parse_table(&mut self, align: Align) -> Result<Node<'a>, LatexError<'a>> {
        // Read the contents of \begin..\end.
        Ok(Node::Table(self.parse_group(Token::End)?, align))
    }

    fn check_lbrace(&mut self) -> Result<(), LatexError<'a>> {
        if !matches!(self.peek_token, Token::LBrace) {
            return Err(LatexError::UnexpectedToken {
                expected: Token::LBrace,
                got: self.next_token(),
            });
        }
        Ok(())
    }

    fn get_bounds(&mut self) -> Result<Bounds<'a>, LatexError<'a>> {
        let next_underscore = matches!(self.peek_token, Token::Underscore);
        if next_underscore || matches!(self.peek_token, Token::Circumflex) {
            self.next_token(); // Discard the underscore or circumflex token.
            let first_bound = Some(Box::new(self.parse_single_token()?));

            // Check whether both an upper and a lower bound were specified.
            if (next_underscore && matches!(self.peek_token, Token::Circumflex))
                || (!next_underscore && matches!(self.peek_token, Token::Underscore))
            {
                self.next_token(); // Discard the circumflex or underscore token.
                let second_bound = Some(Box::new(self.parse_single_token()?));
                // Depending on whether the underscore or the circumflex came first,
                // we have to swap the bounds.
                if next_underscore {
                    Ok(Bounds(first_bound, second_bound))
                } else {
                    Ok(Bounds(second_bound, first_bound))
                }
            } else if next_underscore {
                Ok(Bounds(first_bound, None))
            } else {
                Ok(Bounds(None, first_bound))
            }
        } else {
            Ok(Bounds(None, None))
        }
    }
}

struct Bounds<'a>(Option<Box<Node<'a>>>, Option<Box<Node<'a>>>);

fn squeeze(nodes: Vec<Node>) -> Node {
    if nodes.len() == 1 {
        // SAFETY: `nodes` is not empty.
        unsafe { nodes.into_iter().next().unwrap_unchecked() }
    } else {
        Node::Row(nodes)
    }
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
        Node::Operator(op, _) => Node::SingleLetterIdent(var.transform(op.into()), None),
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
        // SAFETY: `new_nodes` is not empty.
        unsafe { new_nodes.into_iter().next().unwrap_unchecked() }
    } else {
        Node::Row(new_nodes)
    }
}
