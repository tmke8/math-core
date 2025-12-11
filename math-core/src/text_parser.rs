use mathml_renderer::{
    arena::StringBuilder,
    attribute::HtmlTextStyle,
    length::{Length, LengthUnit},
    symbol,
};

use crate::{
    error::{LatexErrKind, LatexError},
    parse::{ParseResult, Parser},
    specifications::LatexUnit,
    token::{TokLoc, Token},
};

impl<'cell, 'arena, 'source> Parser<'cell, 'arena, 'source> {
    pub(super) fn parse_in_text_mode(
        &mut self,
        initial_style: Option<HtmlTextStyle>,
    ) -> ParseResult<'cell, 'source, Vec<(Option<HtmlTextStyle>, &'arena str)>> {
        let mut style_stack = vec![(0usize, initial_style)];
        let mut str_builder: Option<StringBuilder> = None;
        let mut snippets: Vec<(Option<HtmlTextStyle>, &'arena str)> = Vec::new();
        let mut accent_to_insert: Option<char> = None;
        let mut brace_nesting = 0usize;

        while let Some((previous_nesting, current_style)) = style_stack.last().copied() {
            let tokloc = self.tokens.next();
            let TokLoc(loc, token) = tokloc?;
            let c: Result<char, LatexErrKind> = match token {
                Token::Letter(c) | Token::UprightLetter(c) => Ok(c),
                Token::Whitespace | Token::NonBreakingSpace => Ok('\u{A0}'),
                Token::Open(op) | Token::Close(op) => Ok(op.as_op().into()),
                Token::BinaryOp(op) => Ok(op.as_op().into()),
                Token::Relation(op) => Ok(op.as_op().into()),
                Token::SquareBracketOpen => Ok(symbol::LEFT_SQUARE_BRACKET.as_op().into()),
                Token::SquareBracketClose => Ok(symbol::RIGHT_SQUARE_BRACKET.as_op().into()),
                Token::Digit(digit) => Ok(digit),
                Token::Prime => Ok('’'),
                Token::ForceRelation(op) => Ok(op.as_char()),
                Token::Punctuation(op) => Ok(op.as_op().into()),
                tok @ (Token::OpGreaterThan
                | Token::OpLessThan
                | Token::OpAmpersand
                | Token::PseudoOperator(_)
                | Token::PseudoOperatorLimits(_)) => {
                    let output = match tok {
                        Token::OpGreaterThan => "&gt;",
                        Token::OpLessThan => "&lt;",
                        Token::OpAmpersand => "&amp;",
                        Token::PseudoOperator(name) | Token::PseudoOperatorLimits(name) => name,
                        _ => unreachable!(),
                    };
                    if let Some(str_builder) = &mut str_builder {
                        str_builder.push_str(output);
                        continue;
                    } else {
                        snippets.push((current_style, output));
                        style_stack.pop();
                        brace_nesting = previous_nesting;
                        if !style_stack.is_empty() {
                            // If there are still styles to process, we must have been within a group.
                            str_builder = Some(self.buffer.get_builder());
                        }
                        continue;
                    }
                }
                Token::Space(length) => {
                    if length == Length::new(1.0, LengthUnit::Em) {
                        Ok('\u{2003}')
                    } else if length == LatexUnit::Mu.length_with_unit(5.0) {
                        Ok('\u{2004}')
                    } else if length == LatexUnit::Mu.length_with_unit(4.0) {
                        Ok('\u{205F}')
                    } else if length == LatexUnit::Mu.length_with_unit(3.0) {
                        Ok('\u{2009}')
                    } else {
                        // Ignore other spaces in text mode.
                        if str_builder.is_none() {
                            // This space was the only thing after the style change.
                            // So we need to pop the style stack and restore the previous nesting.
                            style_stack.pop();
                            brace_nesting = previous_nesting;
                            if !style_stack.is_empty() {
                                // If there are still styles to process, we must have been within a group.
                                str_builder = Some(self.buffer.get_builder());
                            }
                        }
                        continue;
                    }
                }
                Token::GroupBegin => {
                    if str_builder.is_none() {
                        str_builder = Some(self.buffer.get_builder());
                    } else {
                        brace_nesting += 1;
                    }
                    continue;
                }
                Token::GroupEnd => {
                    if let Some(builder) = str_builder.take() {
                        if let Some(new_nesting) = brace_nesting.checked_sub(1) {
                            brace_nesting = new_nesting;
                            // TODO: Avoid this back-and-forth with the `str_builder`.
                            str_builder = Some(builder);
                            continue;
                        } else {
                            if !builder.is_empty() {
                                let text = builder.finish(self.arena);
                                snippets.push((current_style, text));
                            }
                            style_stack.pop();
                            brace_nesting = previous_nesting;
                            if !style_stack.is_empty() {
                                str_builder = Some(self.buffer.get_builder());
                            }
                            continue;
                        }
                    } else {
                        Err(LatexErrKind::UnexpectedClose(token))
                    }
                }
                Token::TextModeAccent(accent) => {
                    if str_builder.is_some() {
                        accent_to_insert = Some(accent);
                        continue;
                    } else {
                        Err(LatexErrKind::UnexpectedEOF)
                    }
                }
                Token::Text(style) => {
                    if let Some(builder) = str_builder.take()
                        && !builder.is_empty()
                    {
                        let text = builder.finish(self.arena);
                        snippets.push((current_style, text));
                    }
                    style_stack.push((brace_nesting, style));
                    brace_nesting = 0;
                    // Discard any whitespace that immediately follows the `Text` token.
                    if matches!(self.tokens.peek().token(), Token::Whitespace) {
                        self.tokens.next()?;
                    }
                    continue;
                }
                Token::Eof => Err(LatexErrKind::UnexpectedEOF),
                Token::End | Token::Right => Err(LatexErrKind::UnexpectedClose(token)),
                _ => Err(LatexErrKind::NotValidInTextMode(token)),
            };
            let c = c.map_err(|err| self.tokens.lexer.alloc_err(LatexError(loc, err)))?;
            if let Some(builder) = &mut str_builder {
                builder.push_char(c);
                if let Some(accent) = accent_to_insert.take() {
                    builder.push_char(accent);
                }
            } else {
                let mut b = [0u8; 4];
                let text = self.arena.alloc_str(c.encode_utf8(&mut b));
                snippets.push((current_style, text));
                style_stack.pop();
                brace_nesting = previous_nesting;
                if !style_stack.is_empty() {
                    // If there are still styles to process, we must have been within a group.
                    str_builder = Some(self.buffer.get_builder());
                }
            }
        }
        Ok(snippets)
    }
}
