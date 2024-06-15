use std::{cell::Cell, mem};

use crate::{
    arena::{Arena, Buffer, NodeList, NodeReference, StrReference},
    ast::Node,
    attribute::{Accent, Align, MathSpacing, MathVariant, OpAttr, Style, TextTransform},
    commands::get_negated_op,
    error::LatexError,
    lexer::Lexer,
    ops,
    token::Token,
};

#[derive(Debug)]
pub(crate) struct Parser<'source, 'arena> {
    l: Lexer<'source>,
    peek_token: Token<'source>,
    arena: &'arena mut Arena<'source>,
    buffer: &'arena mut Buffer,
}
impl<'source, 'arena> Parser<'source, 'arena> {
    pub(crate) fn new(
        l: Lexer<'source>,
        arena: &'arena mut Arena<'source>,
        buffer: &'arena mut Buffer,
    ) -> Self {
        let mut p = Parser {
            l,
            peek_token: Token::EOF,
            arena,
            buffer,
        };
        // Discard the EOF token we just stored in `peek_token`.
        // This loads the first real token into `peek_token`.
        p.next_token();
        p
    }

    fn next_token(&mut self) -> Token<'source> {
        let peek_token = self.l.next_token(self.peek_token.acts_on_a_digit());
        // Return the previous peek token and store the new peek token.
        mem::replace(&mut self.peek_token, peek_token)
    }

    pub(crate) fn parse(&mut self) -> Result<Node<'source>, LatexError<'source>> {
        let mut nodes = NodeList::new();
        let mut cur_token = self.next_token();

        while !matches!(cur_token, Token::EOF) {
            let node = self.parse_node(cur_token)?;
            nodes.push_ref(self.arena, node);
            cur_token = self.next_token();
        }

        Ok(Node::PseudoRow(nodes))
    }

    fn parse_node(
        &mut self,
        cur_token: Token<'source>,
    ) -> Result<NodeReference, LatexError<'source>> {
        let left = self.parse_single_node(cur_token)?;

        match self.get_bounds()? {
            Bounds(Some(sub), Some(sup)) => Ok(self.new_node_ref(Node::SubSup {
                target: left,
                sub,
                sup,
            })),
            Bounds(Some(symbol), None) => Ok(self.new_node_ref(Node::Subscript(left, symbol))),
            Bounds(None, Some(symbol)) => Ok(self.new_node_ref(Node::Superscript(left, symbol))),
            Bounds(None, None) => Ok(left),
        }
    }

    fn new_node_ref(&mut self, node: Node<'source>) -> NodeReference {
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
            Token::Number(number, op) => match op {
                ops::NULL => Node::Number(number),
                op => {
                    let mut list = NodeList::new();
                    list.push(self.arena, Node::Number(number));
                    list.push(self.arena, Node::Operator(Cell::new(op), None));
                    Node::PseudoRow(list)
                }
            },
            Token::Letter(x) => Node::SingleLetterIdent(Cell::new(x), Cell::new(None)),
            Token::NormalLetter(x) => {
                Node::SingleLetterIdent(Cell::new(x), Cell::new(Some(MathVariant::Normal)))
            }
            Token::Operator(op) => Node::Operator(Cell::new(op), None),
            Token::OpGreaterThan => Node::OpGreaterThan,
            Token::OpLessThan => Node::OpLessThan,
            Token::OpAmpersand => Node::OpAmpersand,
            Token::Function(fun) => Node::MultiLetterIdent(self.buffer.push_str(fun)),
            Token::Space(space) => Node::Space(space),
            Token::NonBreakingSpace => Node::Text("\u{A0}"),
            Token::Sqrt => {
                let next_token = self.next_token();
                if matches!(next_token, Token::Paren(ops::LEFT_SQUARE_BRACKET)) {
                    let degree = self.parse_group(Token::Paren(ops::RIGHT_SQUARE_BRACKET))?;
                    self.next_token(); // Discard the closing token.
                    let content = self.parse_token()?;
                    Node::Root(self.squeeze(degree), content)
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
                        content: self.new_node_ref(inner),
                        style: None,
                    }
                } else {
                    Node::Frac(numerator, denominator, None, displaystyle)
                }
            }
            Token::Genfrac => {
                let node_ref = self.parse_token()?;
                let node = self.arena.get(node_ref);
                let open = match node {
                    Node::Operator(op, _) => op.get(),
                    Node::Row(elements, _) if elements.is_empty() => ops::NULL,
                    _ => return Err(LatexError::UnexpectedEOF),
                };
                let node_ref = self.parse_token()?;
                let node = self.arena.get(node_ref);
                let close = match node {
                    Node::Operator(op, _) => op.get(),
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
                let node_ref = self.parse_token()?;
                let node = self.arena.get(node_ref);
                let style = match node {
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
                let content = self.new_node_ref(inner);
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
                    let op = self.new_node_ref(Node::Operator(Cell::new(x), None));
                    if is_over {
                        let symbol = self.new_node_ref(Node::Overset {
                            symbol: expl,
                            target: op,
                        });
                        Node::Overset { symbol, target }
                    } else {
                        let symbol = self.new_node_ref(Node::Underset {
                            symbol: expl,
                            target: op,
                        });
                        Node::Underset { symbol, target }
                    }
                } else {
                    let symbol = self.new_node_ref(Node::Operator(Cell::new(x), None));
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
                    self.new_node_ref(Node::Operator(Cell::new(op), Some(OpAttr::NoMovableLimits)))
                } else {
                    self.new_node_ref(Node::Operator(Cell::new(op), None))
                };
                match self.get_bounds()? {
                    Bounds(Some(under), Some(over)) => Node::UnderOver {
                        target,
                        under,
                        over,
                    },
                    Bounds(Some(symbol), None) => Node::Underset { target, symbol },
                    Bounds(None, Some(symbol)) => Node::Overset { target, symbol },
                    Bounds(None, None) => Node::Operator(Cell::new(op), None),
                }
            }
            Token::Lim(lim) => {
                let lim = Node::MultiLetterIdent(self.buffer.push_str(lim));
                if matches!(self.peek_token, Token::Underscore) {
                    self.next_token(); // Discard the underscore token.
                    let under = self.parse_single_token()?;
                    Node::Underset {
                        target: self.new_node_ref(lim),
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
                            Node::Operator(Cell::new(negated), None)
                        } else {
                            Node::Operator(Cell::new(op), None)
                        }
                    }
                    Token::OpLessThan => {
                        self.next_token(); // Discard the less-than token.
                        Node::Operator(Cell::new(ops::NOT_LESS_THAN), None)
                    }
                    Token::OpGreaterThan => {
                        self.next_token(); // Discard the greater-than token.
                        Node::Operator(Cell::new(ops::NOT_GREATER_THAN), None)
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
                let node_ref = self.parse_single_token()?;
                let node = self.arena.get(node_ref);
                self.set_normal_variant(node);
                let node_ref = if let Node::Row(nodes, style) = node {
                    self.merge_single_letters(*nodes, style.clone())
                } else {
                    node_ref
                };
                return Ok(node_ref);
            }
            Token::Transform(tf) => {
                let node_ref = self.parse_single_token()?;
                let node = self.arena.get(node_ref);
                self.transform_letters(node, tf);
                if let Node::Row(nodes, style) = node {
                    return Ok(self.merge_single_letters(*nodes, style.clone()));
                } else {
                    return Ok(node_ref);
                }
            }
            Token::Integral(int) => {
                if matches!(self.peek_token, Token::Limits) {
                    self.next_token(); // Discard the limits token.
                    let target = self.new_node_ref(Node::Operator(Cell::new(int), None));
                    match self.get_bounds()? {
                        Bounds(Some(under), Some(over)) => Node::UnderOver {
                            target,
                            under,
                            over,
                        },
                        Bounds(Some(symbol), None) => Node::Underset { target, symbol },
                        Bounds(None, Some(symbol)) => Node::Overset { target, symbol },
                        Bounds(None, None) => Node::Operator(Cell::new(int), None),
                    }
                } else {
                    let target = self.new_node_ref(Node::Operator(Cell::new(int), None));
                    match self.get_bounds()? {
                        Bounds(Some(sub), Some(sup)) => Node::SubSup { target, sub, sup },
                        Bounds(Some(symbol), None) => Node::Subscript(target, symbol),
                        Bounds(None, Some(symbol)) => Node::Superscript(target, symbol),
                        Bounds(None, None) => Node::Operator(Cell::new(int), None),
                    }
                }
            }
            Token::Colon => match &self.peek_token {
                Token::Operator(op @ (ops::EQUALS_SIGN | ops::IDENTICAL_TO)) => {
                    let op = *op;
                    self.next_token(); // Discard the operator token.
                    let mut list = NodeList::new();
                    list.push(
                        self.arena,
                        Node::OperatorWithSpacing {
                            op: Cell::new(ops::COLON),
                            left: Some(MathSpacing::FourMu),
                            right: Some(MathSpacing::Zero),
                        },
                    );
                    list.push(
                        self.arena,
                        Node::OperatorWithSpacing {
                            op: Cell::new(op),
                            left: Some(MathSpacing::Zero),
                            right: None,
                        },
                    );
                    Node::PseudoRow(list)
                }
                _ => Node::OperatorWithSpacing {
                    op: Cell::new(ops::COLON),
                    left: Some(MathSpacing::FourMu),
                    right: Some(MathSpacing::FourMu),
                },
            },
            Token::GroupBegin => {
                let content = self.parse_group(Token::GroupEnd)?;
                self.next_token(); // Discard the closing token.
                if let Some(node_ref) = content.is_singleton() {
                    return Ok(node_ref);
                }
                Node::Row(content, None)
            }
            Token::Paren(paren) => Node::Operator(Cell::new(paren), Some(OpAttr::StretchyFalse)),
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
                    content: self.squeeze(content),
                    style: None,
                }
            }
            Token::Middle => match self.next_token() {
                Token::Operator(op) | Token::Paren(op) => {
                    Node::Operator(Cell::new(op), Some(OpAttr::StretchyTrue))
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
                    "cases" => {
                        let content = self.parse_table(Align::Left)?;
                        Node::Fenced {
                            open: ops::LEFT_CURLY_BRACKET,
                            close: ops::NULL,
                            content: self.new_node_ref(content),
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
                            content: self.new_node_ref(content),
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
                let node_ref = self.parse_single_token()?;
                let start = self.buffer.len();
                let node = self.arena.get(node_ref);
                extract_letters(self.arena, self.buffer, &node)?;
                let end = self.buffer.len();
                Node::MultiLetterIdent(StrReference::new(start, end))
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
        Ok(self.new_node_ref(node))
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
    fn parse_group(&mut self, end_token: Token<'source>) -> Result<NodeList, LatexError<'source>> {
        let mut nodes = NodeList::new();

        while self.peek_token != end_token {
            let token = self.next_token();
            if matches!(token, Token::EOF) {
                // When the input ends without the closing token.
                return Err(LatexError::UnclosedGroup(end_token));
            }
            let node = self.parse_node(token)?;
            nodes.push_ref(self.arena, node);
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
        Ok(Node::Table(content, align))
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
        let mut prime_counter: usize = 0;

        while matches!(self.peek_token, Token::Prime) {
            self.next_token(); // Discard the prime token.
            prime_counter += 1;
        }

        // Check whether the first bound is specified and is a lower bound.
        let first_underscore = matches!(self.peek_token, Token::Underscore);

        let (sub, mut sup) = if first_underscore || matches!(self.peek_token, Token::Circumflex) {
            let first_bound = Some(self.get_bound()?);

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
                let second_bound = Some(self.get_bound()?);
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

        if prime_counter > 0 {
            let mut superscripts = NodeList::new();
            for _ in 0..prime_counter {
                superscripts.push(self.arena, Node::Operator(Cell::new(ops::PRIME), None));
            }
            if let Some(sup) = sup {
                superscripts.push_ref(self.arena, sup);
            }
            sup = Some(self.squeeze(superscripts));
        }

        Ok(Bounds(sub, sup))
    }

    fn get_bound(&mut self) -> Result<NodeReference, LatexError<'source>> {
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

    fn squeeze(&mut self, nodes: NodeList) -> NodeReference {
        match nodes.is_singleton() {
            Some(value) => value,
            None => self.new_node_ref(Node::Row(nodes, None)),
        }
    }

    /// Set the math variant of all single-letter identifiers in `node` to `var`.
    /// The change is applied in-place.
    fn set_normal_variant(&self, node: &Node<'source>) {
        match node {
            Node::SingleLetterIdent(_, maybe_var) => {
                maybe_var.set(Some(MathVariant::Normal));
            }
            Node::Row(list, _) => {
                for node in list.iter(self.arena) {
                    self.set_normal_variant(node);
                }
            }
            _ => {}
        };
    }

    /// Transform the text of all single-letter identifiers and operators using `tf`.
    /// The change is applied in-place.
    fn transform_letters(&self, node: &Node<'source>, tf: TextTransform) {
        match node {
            Node::Row(list, _) => {
                for node in list.iter(self.arena) {
                    self.transform_letters(node, tf.clone());
                }
            }
            Node::SingleLetterIdent(x, _) => {
                x.set(tf.transform(x.get()));
            }
            // Node::Operator(ref op, _) => {
            //     let _ = mem::replace(
            //         node,
            //         Node::SingleLetterIdent(
            //             Cell::new(tf.transform(op.get().into())),
            //             Cell::new(None),
            //         ),
            //     );
            // }
            _ => {}
        }
    }

    fn merge_single_letters(&mut self, nodes: NodeList, style: Option<Style>) -> NodeReference {
        let nodes = if let Some(mut head) = nodes.get_head() {
            let mut new_nodes = NodeList::new();
            let mut start: Option<usize> = None;
            loop {
                let item = self.arena.get_raw(head);
                let node = &item.node;
                let next = item.next;
                if let Node::SingleLetterIdent(c, _) = node {
                    if start.is_none() {
                        // We start collecting.
                        start = Some(self.buffer.0.len());
                    }
                    self.buffer.0.push(c.get());
                } else {
                    // Commit the collected letters.
                    if let Some(start) = start.take() {
                        let slice = StrReference::new(start, self.buffer.0.len());
                        new_nodes.push(self.arena, Node::MultiLetterIdent(slice));
                    }
                    new_nodes.push_ref(self.arena, head);
                }
                match next {
                    Some(tail) => {
                        head = tail;
                    }
                    None => break,
                }
            }
            if let Some(start) = start {
                let slice = StrReference::new(start, self.buffer.0.len());
                new_nodes.push(self.arena, Node::MultiLetterIdent(slice));
            }
            if let Some(node_ref) = new_nodes.is_singleton() {
                return node_ref;
            }
            new_nodes
        } else {
            nodes
        };
        let node = Node::Row(nodes, style);
        self.new_node_ref(node)
    }
}

struct Bounds(Option<NodeReference>, Option<NodeReference>);

fn extract_letters<'source>(
    arena: &Arena<'source>,
    buffer: &mut Buffer,
    node: &Node<'source>,
) -> Result<(), LatexError<'source>> {
    match node {
        Node::SingleLetterIdent(c, _) => {
            buffer.0.push(c.get());
        }
        Node::Row(nodes, _) => {
            for node in nodes.iter(arena) {
                extract_letters(arena, buffer, node)?;
            }
        }
        Node::Number(n) => {
            buffer.0.push_str(n);
        }
        Node::Operator(op, _) | Node::OperatorWithSpacing { op, .. } => {
            buffer.0.push(op.get().into());
        }
        _ => return Err(LatexError::ExpectedText("\\operatorname")),
    }
    Ok(())
}
