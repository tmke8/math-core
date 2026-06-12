use mathml_renderer::{
    arena::StringBuilder,
    attribute::{HtmlTextSize, HtmlTextStyle},
    length::{Length, LengthUnit},
    super_char::SuperChar,
    symbol,
};

use crate::{
    error::{LatexErrKind, LatexError},
    parser::{ParseResult, Parser},
    specifications::LatexUnit,
    token::{EndToken, Mode, TextToken, Token},
};

/// A run of text with uniform style and size.
pub(super) struct TextSnippet<'arena>(
    pub Option<HtmlTextStyle>,
    pub Option<HtmlTextSize>,
    pub &'arena str,
);

impl<'arena> Parser<'_, '_, 'arena> {
    pub(super) fn extract_text(
        &mut self,
        initial_style: Option<HtmlTextStyle>,
        text_mode: bool,
    ) -> ParseResult<Vec<TextSnippet<'arena>>> {
        let mut style_stack = vec![(0usize, initial_style)];
        let mut str_builder: Option<StringBuilder> = None;
        let mut snippets: Vec<TextSnippet<'arena>> = Vec::new();
        let mut accent_to_insert: Option<char> = None;
        let mut brace_nesting = 0usize;
        // Size declarations like `\large`. Each entry records the `style_stack` depth and the
        // brace nesting level at which the size was declared, because the size stays in effect
        // until either that style scope or that brace group is closed. (`brace_nesting` is
        // reset on every style push, so the nesting level alone would be ambiguous.)
        let mut size_stack: Vec<(usize, usize, HtmlTextSize)> = Vec::new();

        while let Some((previous_nesting, current_style)) = style_stack.last().copied() {
            let current_size = size_stack.last().map(|&(_, _, size)| size);
            let tokloc = if text_mode {
                self.tokens.next_any_token()
            } else {
                self.tokens.next()
            };
            let (token, span) = tokloc?.into_parts();
            let c: Result<SuperChar, LatexErrKind> = if let Token::TextMode(text_token) = token {
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
                        TextToken::Letter(ch) => Ok(ch.into()),
                        TextToken::Style(style) => {
                            if let Some(builder) = str_builder.take()
                                && !builder.is_empty()
                            {
                                let text = builder.finish(self.arena);
                                snippets.push(TextSnippet(current_style, current_size, text));
                            }
                            style_stack.push((brace_nesting, Some(style)));
                            brace_nesting = 0;
                            continue;
                        }
                        TextToken::Size(text_size) => {
                            if let Some(builder) = str_builder.take() {
                                if builder.is_empty() {
                                    str_builder = Some(builder);
                                } else {
                                    let text = builder.finish(self.arena);
                                    snippets.push(TextSnippet(current_style, current_size, text));
                                    str_builder = Some(self.buffer.get_builder());
                                }
                                size_stack.push((style_stack.len(), brace_nesting, text_size));
                            } else {
                                // The size command itself is the argument of the style command,
                                // e.g. `\text\large x`, which is equivalent to `\text{}x`:
                                // the text is empty and the size never takes effect.
                                style_stack.pop();
                                brace_nesting = previous_nesting;
                                discard_dead_sizes(
                                    &mut size_stack,
                                    style_stack.len(),
                                    brace_nesting,
                                );
                                if !style_stack.is_empty() {
                                    // If there are still styles to process, we must have been within a group.
                                    str_builder = Some(self.buffer.get_builder());
                                }
                            }
                            continue;
                        }
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
                    | Token::Whitespace
                    | Token::NonBreakingSpace
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
                    Token::SquareBracketOpen => {
                        Ok(symbol::LEFT_SQUARE_BRACKET.as_op().as_superchar())
                    }
                    Token::SquareBracketClose => {
                        Ok(symbol::RIGHT_SQUARE_BRACKET.as_op().as_superchar())
                    }
                    Token::Digit(digit) => Ok(digit.into()),
                    Token::Whitespace | Token::NonBreakingSpace => {
                        Ok(SuperChar::from_char(symbol::NO_BREAK_SPACE))
                    }
                    Token::InternalStringLiteral(output) => {
                        if let Some(str_builder) = &mut str_builder {
                            str_builder.push_str(output);
                            continue;
                        } else {
                            snippets.push(TextSnippet(current_style, current_size, output));
                            style_stack.pop();
                            brace_nesting = previous_nesting;
                            discard_dead_sizes(&mut size_stack, style_stack.len(), brace_nesting);
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
                                let group_had_sizes = matches!(
                                    size_stack.last(),
                                    Some(&(depth, nesting, _))
                                        if depth == style_stack.len() && nesting > brace_nesting
                                );
                                if group_had_sizes {
                                    // The group that just closed contained size declarations,
                                    // which now go out of scope.
                                    if !builder.is_empty() {
                                        let text = builder.finish(self.arena);
                                        snippets.push(TextSnippet(
                                            current_style,
                                            current_size,
                                            text,
                                        ));
                                    }
                                    discard_dead_sizes(
                                        &mut size_stack,
                                        style_stack.len(),
                                        brace_nesting,
                                    );
                                    str_builder = Some(self.buffer.get_builder());
                                } else {
                                    // TODO: Avoid this back-and-forth with the `str_builder`.
                                    str_builder = Some(builder);
                                }
                                continue;
                            } else {
                                if !builder.is_empty() {
                                    let text = builder.finish(self.arena);
                                    snippets.push(TextSnippet(current_style, current_size, text));
                                }
                                style_stack.pop();
                                brace_nesting = previous_nesting;
                                discard_dead_sizes(
                                    &mut size_stack,
                                    style_stack.len(),
                                    brace_nesting,
                                );
                                if !style_stack.is_empty() {
                                    str_builder = Some(self.buffer.get_builder());
                                }
                                continue;
                            }
                        } else {
                            Err(LatexErrKind::ExpectedArgumentGotClose)
                        }
                    }
                    Token::MathOrTextMode(_, ch) => Ok(ch.into()),
                    Token::Text(style) => {
                        if let Some(builder) = str_builder.take()
                            && !builder.is_empty()
                        {
                            let text = builder.finish(self.arena);
                            snippets.push(TextSnippet(current_style, current_size, text));
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
                    Token::Letter(c, _) | Token::UprightLetter(c) => Ok(c),
                    Token::Open(op) | Token::Close(op) | Token::Ord(op) => {
                        Ok(op.as_op().as_superchar())
                    }
                    Token::BinaryOp(op) => Ok(op.as_op().as_superchar()),
                    Token::Relation(op) => Ok(op.as_op().as_superchar()),
                    Token::ForceRelation(op) => Ok(op.as_superchar()),
                    Token::Punctuation(op) => Ok(op.as_op().as_superchar()),

                    Token::PseudoOperator(output) | Token::PseudoOperatorLimits(output) => {
                        if let Some(str_builder) = &mut str_builder {
                            str_builder.push_str(output);
                            continue;
                        }
                        snippets.push(TextSnippet(current_style, current_size, output));
                        style_stack.pop();
                        brace_nesting = previous_nesting;
                        discard_dead_sizes(&mut size_stack, style_stack.len(), brace_nesting);
                        if !style_stack.is_empty() {
                            // If there are still styles to process, we must have been within a group.
                            str_builder = Some(self.buffer.get_builder());
                        }
                        continue;
                    }
                    Token::Space(length) => {
                        if length == Length::new(1.0, LengthUnit::Em) {
                            Ok(SuperChar::from_char('\u{2003}'))
                        } else if length == LatexUnit::Mu.length_with_unit(5.0) {
                            Ok(SuperChar::from_char('\u{2004}'))
                        } else if length == LatexUnit::Mu.length_with_unit(4.0) {
                            Ok(SuperChar::from_char('\u{205F}'))
                        } else if length == LatexUnit::Mu.length_with_unit(3.0) {
                            Ok(SuperChar::from_char('\u{2009}'))
                        } else {
                            // Ignore other spaces in text mode.
                            if str_builder.is_none() {
                                // This space was the only thing after the style change.
                                // So we need to pop the style stack and restore the previous nesting.
                                style_stack.pop();
                                brace_nesting = previous_nesting;
                                discard_dead_sizes(
                                    &mut size_stack,
                                    style_stack.len(),
                                    brace_nesting,
                                );
                                if !style_stack.is_empty() {
                                    // If there are still styles to process, we must have been within a group.
                                    str_builder = Some(self.buffer.get_builder());
                                }
                            }
                            continue;
                        }
                    }
                    Token::Prime(kind) => Ok(kind.to_ord().as_op().as_superchar()),

                    _ => Err(LatexErrKind::CouldNotExtractText),
                }
            } else {
                Err(LatexErrKind::NotValidInTextMode)
            };
            let c = c.map_err(|err| Box::new(LatexError(span.into(), err)))?;
            if let Some(builder) = &mut str_builder {
                builder.push_superchar(c);
                if let Some(accent) = accent_to_insert.take() {
                    builder.push_char(accent);
                }
            } else {
                let mut b = [0u8; SuperChar::MAX_LEN_UTF8];
                let text = self.arena.alloc_str(c.encode_utf8(&mut b));
                snippets.push(TextSnippet(current_style, current_size, text));
                style_stack.pop();
                brace_nesting = previous_nesting;
                discard_dead_sizes(&mut size_stack, style_stack.len(), brace_nesting);
                if !style_stack.is_empty() {
                    // If there are still styles to process, we must have been within a group.
                    str_builder = Some(self.buffer.get_builder());
                }
            }
        }
        Ok(snippets)
    }
}

/// Remove all size declarations that have gone out of scope, i.e. those declared in a style
/// scope or brace group that has been closed.
fn discard_dead_sizes(
    size_stack: &mut Vec<(usize, usize, HtmlTextSize)>,
    style_depth: usize,
    brace_nesting: usize,
) {
    while let Some(&(depth, nesting, _)) = size_stack.last() {
        if depth > style_depth || (depth == style_depth && nesting > brace_nesting) {
            size_stack.pop();
        } else {
            break;
        }
    }
}
