use std::mem;

use crate::{
    ast::Node,
    attribute::{Accent, Align, MathSpacing, MathVariant, OpAttr, Style, TextTransform},
    commands::get_negated_op,
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
            peek_token: Token::EOF,
        };
        // Discard the EOF token we just stored in `peek_token`.
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

        match self.get_bounds()? {
            Bounds(Some(sub), Some(sup)) => Ok(Node::SubSup {
                target: Box::new(left),
                sub,
                sup,
            }),
            Bounds(Some(symbol), None) => Ok(Node::Subscript(Box::new(left), symbol)),
            Bounds(None, Some(symbol)) => Ok(Node::Superscript(Box::new(left), symbol)),
            Bounds(None, None) => Ok(left),
        }
    }

    // Read the node immediately after without worrying about whether
    // the infix operator `_`, `^`, `'` will continue
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
            Token::Function(fun) => Node::MultiLetterIdent(fun.to_string()),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace => Node::Text("\u{A0}"),
            Token::Sqrt => {
                let next_token = self.next_token();
                if matches!(next_token, Token::Paren(ops::LEFT_SQUARE_BRACKET)) {
                    let degree = self.parse_group(Token::Paren(ops::RIGHT_SQUARE_BRACKET))?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_token()?;
                    Node::Root(Box::new(squeeze(degree)), Box::new(content))
                } else {
                    let content = self.parse_node(next_token)?;
                    Node::Sqrt(Box::new(content))
                }
            }
            ref tok @ (Token::Frac(displaystyle) | Token::Binom(displaystyle)) => {
                let numerator = Box::new(self.parse_token()?);
                let denominator = Box::new(self.parse_token()?);
                if matches!(tok, Token::Binom(_)) {
                    let inner = Node::Frac(numerator, denominator, Some('0'), displaystyle);
                    Node::Fenced {
                        open: ops::LEFT_PARENTHESIS,
                        close: ops::RIGHT_PARENTHESIS,
                        content: Box::new(inner),
                        style: None,
                    }
                } else {
                    Node::Frac(numerator, denominator, None, displaystyle)
                }
            }
            Token::Genfrac => {
                let open = match self.parse_token()? {
                    Node::Operator(op, _) => op,
                    Node::Row(elements, _) if elements.is_empty() => ops::NULL,
                    _ => return Err(LatexError::UnexpectedEOF),
                };
                let close = match self.parse_token()? {
                    Node::Operator(op, _) => op,
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
                let style = match self.parse_token()? {
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
                let numerator = Box::new(self.parse_token()?);
                let denominator = Box::new(self.parse_token()?);
                let inner = Node::Frac(numerator, denominator, line_thickness, None);
                let content = Box::new(inner);
                Node::Fenced {
                    open,
                    close,
                    content,
                    style,
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
                let lim = Node::MultiLetterIdent(lim.to_string());
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
            tok @ Token::Not => {
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
                        Node::MultiLetterIdent(negated_letter.iter().collect())
                    }
                    _ => {
                        return Err(LatexError::CannotBeUsedHere {
                            got: tok,
                            correct_place: "before supported operators",
                        })
                    }
                }
            }
            Token::NormalVariant => {
                let node = self.parse_single_token()?;
                let mut node = if let Node::Row(nodes, style) = node {
                    merge_single_letters(nodes, style)
                } else {
                    node
                };
                set_normal_variant(&mut node);
                node
            }
            Token::Transform(tf) => {
                let mut node = self.parse_single_token()?;
                transform_letters(&mut node, tf);
                if let Node::Row(nodes, style) = node {
                    merge_single_letters(nodes, style)
                } else {
                    node
                }
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
                Token::Operator(op @ (ops::EQUALS_SIGN | ops::IDENTICAL_TO)) => {
                    let op = *op;
                    self.next_token(); // Discard the operator token.
                    Node::PseudoRow(vec![
                        Node::OperatorWithSpacing {
                            op: ops::COLON,
                            left: Some(MathSpacing::FourMu),
                            right: Some(MathSpacing::Zero),
                        },
                        Node::OperatorWithSpacing {
                            op,
                            left: Some(MathSpacing::Zero),
                            right: None,
                        },
                    ])
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
                squeeze(content)
            }
            Token::Paren(paren) => Node::Operator(paren, Some(OpAttr::StretchyFalse)),
            Token::Left => {
                let open = match self.next_token() {
                    Token::Paren(open) => open,
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
                    content: Box::new(squeeze(content)),
                    style: None,
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
                        style: None,
                    },
                    "matrix" => self.parse_table(Align::Center)?,
                    matrix_variant @ ("pmatrix" | "bmatrix" | "vmatrix") => {
                        let content = Box::new(self.parse_table(Align::Center)?);
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
                            content,
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
                let mut name = String::new();
                extract_letters(&mut name, self.parse_single_token()?)?;
                Node::MultiLetterIdent(name)
            }
            Token::Text => {
                self.check_lbrace()?;
                // Read the text.
                let text = self.parse_text_group()?;
                Node::Text(text)
            }
            Token::Ampersand => Node::ColumnSeparator,
            Token::NewLine => Node::RowSeparator,
            Token::Mathstrut => Node::Mathstrut,
            Token::Style(style) => Node::Row(self.parse_group(Token::GroupEnd)?, Some(style)),
            Token::UnknownCommand(name) => {
                return Err(LatexError::UnknownCommand(name));
            }
            // tok @ (Token::Underscore | Token::Circumflex) => {
            tok @ (Token::Circumflex | Token::Prime) => {
                return Err(LatexError::CannotBeUsedHere {
                    got: tok,
                    correct_place: "after an identifier or operator",
                });
            }
            Token::Underscore => {
                let sub = Box::new(self.parse_single_token()?);
                let base = Box::new(self.parse_single_token()?);
                Node::Multiscript { base, sub }
            }
            tok @ Token::Limits => {
                return Err(LatexError::CannotBeUsedHere {
                    got: tok,
                    correct_place: r"after \int, \sum, ...",
                })
            }
            Token::EOF => return Err(LatexError::UnexpectedEOF),
            tok @ (Token::End | Token::Right | Token::GroupEnd) => {
                return Err(LatexError::UnexpectedClose(tok))
            }
        };
        Ok(node)
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
        let mut nodes = Vec::new();

        while self.peek_token != end_token {
            let token = self.next_token();
            if matches!(token, Token::EOF) {
                // When the input ends without the closing token.
                return Err(LatexError::UnclosedGroup(end_token));
            }
            nodes.push(self.parse_node(token)?);
        }
        Ok(nodes)
    }

    /// Parse the contents of a group which can only contain text.
    fn parse_text_group(&mut self) -> Result<&'a str, LatexError<'a>> {
        let result = self
            .l
            .read_text_content()
            .ok_or(LatexError::UnclosedGroup(Token::GroupEnd));
        self.next_token(); // Discard the opening token (which is still stored as `peek`).
        result
    }

    #[inline]
    fn parse_table(&mut self, align: Align) -> Result<Node<'a>, LatexError<'a>> {
        // Read the contents of \begin..\end.
        let content = self.parse_group(Token::End)?;
        self.next_token(); // Discard the closing token.
        Ok(Node::Table(content, align))
    }

    fn check_lbrace(&mut self) -> Result<(), LatexError<'a>> {
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
    fn get_bounds(&mut self) -> Result<Bounds<'a>, LatexError<'a>> {
        let mut prime_counter: usize = 0;

        while matches!(self.peek_token, Token::Prime) {
            self.next_token(); // Discard the prime token.
            prime_counter += 1;
        }

        let next_underscore = matches!(self.peek_token, Token::Underscore);
        let (sub, mut sup) = if next_underscore || matches!(self.peek_token, Token::Circumflex) {
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
            let first_bound = Some(self.parse_single_node(next_token)?);

            // Check whether both an upper and a lower bound were specified.
            if (next_underscore && matches!(self.peek_token, Token::Circumflex))
                || (!next_underscore && matches!(self.peek_token, Token::Underscore))
            {
                self.next_token(); // Discard the circumflex or underscore token.
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
                let second_bound = Some(self.parse_single_node(next_token)?);
                // Depending on whether the underscore or the circumflex came first,
                // we have to swap the bounds.
                if next_underscore {
                    (first_bound, second_bound)
                } else {
                    (second_bound, first_bound)
                }
            } else if next_underscore {
                (first_bound, None)
            } else {
                (None, first_bound)
            }
        } else {
            (None, None)
        };

        if prime_counter > 0 {
            let mut superscripts: Vec<Node> = (0..prime_counter)
                .map(|_| Node::Operator(ops::PRIME, None))
                .collect();
            if let Some(sup) = sup {
                superscripts.push(sup);
            }
            sup = Some(squeeze(superscripts));
        }

        Ok(Bounds(sub.map(Box::new), sup.map(Box::new)))
    }
}

struct Bounds<'a>(Option<Box<Node<'a>>>, Option<Box<Node<'a>>>);

fn squeeze(nodes: Vec<Node>) -> Node {
    if nodes.len() == 1 {
        // SAFETY: `nodes` is not empty.
        unsafe { nodes.into_iter().next().unwrap_unchecked() }
    } else {
        Node::Row(nodes, None)
    }
}

/// Set the math variant of all single-letter identifiers in `node` to `var`.
/// The change is applied in-place.
fn set_normal_variant(node: &mut Node) {
    match node {
        Node::SingleLetterIdent(_, maybe_var) => {
            *maybe_var = Some(MathVariant::Normal);
        }
        Node::Row(vec, _) => {
            for node in vec.iter_mut() {
                set_normal_variant(node);
            }
        }
        _ => {}
    };
}

/// Transform the text of all single-letter identifiers and operators using `tf`.
/// The change is applied in-place.
fn transform_letters(node: &mut Node, tf: TextTransform) {
    match node {
        Node::SingleLetterIdent(x, _) => {
            *x = tf.transform(*x);
        }
        Node::Operator(op, _) => {
            let op = *op;
            let _ = mem::replace(node, Node::SingleLetterIdent(tf.transform(op.into()), None));
        }
        Node::Row(vec, _) => {
            for node in vec.iter_mut() {
                transform_letters(node, tf.clone());
            }
        }
        _ => {}
    }
}

fn merge_single_letters(nodes: Vec<Node>, style: Option<Style>) -> Node {
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
                new_nodes.push(Node::MultiLetterIdent(letters));
            }
            new_nodes.push(node);
        }
    }
    if let Some(letters) = collected {
        new_nodes.push(Node::MultiLetterIdent(letters));
    }
    if new_nodes.len() == 1 {
        // SAFETY: `new_nodes` is not empty.
        unsafe { new_nodes.into_iter().next().unwrap_unchecked() }
    } else {
        Node::Row(new_nodes, style)
    }
}

fn extract_letters<'a>(s: &mut String, node: Node<'a>) -> Result<(), LatexError<'a>> {
    match node {
        Node::SingleLetterIdent(c, _) => {
            s.push(c);
        }
        Node::Row(nodes, _) => {
            for node in nodes {
                extract_letters(s, node)?;
            }
        }
        Node::Number(n) => {
            s.push_str(n);
        }
        Node::Operator(op, _) | Node::OperatorWithSpacing { op, .. } => {
            s.push(op.into());
        }
        _ => return Err(LatexError::ExpectedText("\\operatorname")),
    }
    Ok(())
}
