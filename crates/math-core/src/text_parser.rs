use mathml_renderer::{
    arena::StringBuilder,
    attribute::HtmlTextStyle,
    length::{Length, LengthUnit},
    symbol,
};

use crate::{
    error::{LatexErrKind, LatexError},
    parser::{ParseResult, Parser},
    specifications::LatexUnit,
    token::{EndToken, Mode, TextToken, Token},
};

impl<'cell, 'arena, 'source, 'config> Parser<'cell, 'arena, 'source, 'config> {
    pub(super) fn extract_text(
        &mut self,
        initial_style: Option<HtmlTextStyle>,
        text_mode: bool,
    ) -> ParseResult<Vec<(Option<HtmlTextStyle>, &'arena str)>> {
        let mut style_stack = vec![(0usize, initial_style)];
        let mut str_builder: Option<StringBuilder> = None;
        let mut snippets: Vec<(Option<HtmlTextStyle>, &'arena str)> = Vec::new();
        let mut accent_to_insert: Option<char> = None;
        let mut brace_nesting = 0usize;

        while let Some((previous_nesting, current_style)) = style_stack.last().copied() {
            let tokloc = if text_mode {
                self.tokens.next_with_whitespace()
            } else {
                self.tokens.next()
            };
            let (token, span) = tokloc?.into_parts();
            let c: Result<char, LatexErrKind> = if let Token::TextMode(text_token) = token {
                if text_mode {
                    match text_token {
                        TextToken::Accent(accent) => {
                            if str_builder.is_some() {
                                accent_to_insert = Some(accent);
                                continue;
                            } else {
                                Err(LatexErrKind::ExpectedArgumentGotEOI)
                            }
                        }
                        TextToken::Letter(ch) => Ok(ch),
                    }
                } else {
                    Err(LatexErrKind::NotValidInMathMode)
                }
            } else if matches!(
                token,
                Token::Letter(_, Mode::MathOrText)
                    | Token::SquareBracketOpen
                    | Token::SquareBracketClose
                    | Token::Digit(_)
                    | Token::Prime
                    | Token::Whitespace
                    | Token::NonBreakingSpace
                    | Token::OpGreaterThan
                    | Token::OpLessThan
                    | Token::OpAmpersand
                    | Token::InternalStringLiteral(_)
                    | Token::GroupBegin
                    | Token::GroupEnd
                    | Token::MathOrTextMode(_, _)
                    | Token::Text(_)
                    | Token::Eoi
                    | Token::Right
                    | Token::End(_)
            ) {
                // These tokens are valid in both math and text mode.
                match token {
                    Token::Letter(c, Mode::MathOrText) => Ok(c),
                    Token::SquareBracketOpen => Ok(symbol::LEFT_SQUARE_BRACKET.as_op().into()),
                    Token::SquareBracketClose => Ok(symbol::RIGHT_SQUARE_BRACKET.as_op().into()),
                    Token::Digit(digit) => Ok(digit),
                    Token::Prime => Ok('’'),
                    Token::Whitespace | Token::NonBreakingSpace => Ok('\u{A0}'),
                    tok @ (Token::OpGreaterThan
                    | Token::OpLessThan
                    | Token::OpAmpersand
                    | Token::InternalStringLiteral(_)) => {
                        let output = match tok {
                            Token::OpGreaterThan => "&gt;",
                            Token::OpLessThan => "&lt;",
                            Token::OpAmpersand => "&amp;",
                            Token::InternalStringLiteral(content) => content,
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
                            Err(LatexErrKind::ExpectedArgumentGotClose)
                        }
                    }
                    Token::MathOrTextMode(_, ch) => Ok(ch),
                    Token::Text(style) => {
                        if let Some(builder) = str_builder.take()
                            && !builder.is_empty()
                        {
                            let text = builder.finish(self.arena);
                            snippets.push((current_style, text));
                        }
                        style_stack.push((brace_nesting, style));
                        brace_nesting = 0;
                        continue;
                    }
                    Token::Eoi => {
                        if str_builder.is_some() {
                            Err(LatexErrKind::UnclosedGroup(EndToken::GroupClose))
                        } else {
                            Err(LatexErrKind::ExpectedArgumentGotEOI)
                        }
                    }
                    Token::Right | Token::End(_) => Err(LatexErrKind::ExpectedArgumentGotClose),
                    _ => unreachable!(),
                }
            } else if !text_mode {
                // These tokens are only valid in math mode.
                match token {
                    Token::Letter(c, _) => Ok(c),
                    Token::UprightLetter(c) => Ok(c),
                    Token::Open(op) | Token::Close(op) => Ok(op.as_op().into()),
                    Token::BinaryOp(op) => Ok(op.as_op().into()),
                    Token::Relation(op) => Ok(op.as_op().into()),
                    Token::Ord(op) => Ok(op.as_op().into()),
                    Token::ForceRelation(op) => Ok(op.as_char()),
                    Token::Punctuation(op) => Ok(op.as_op().into()),

                    Token::PseudoOperator(output) | Token::PseudoOperatorLimits(output) => {
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
                    _ => Err(LatexErrKind::CouldNotExtractText),
                }
            } else {
                Err(LatexErrKind::NotValidInTextMode)
            };
            let c = c.map_err(|err| Box::new(LatexError(span.into(), err)))?;
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
