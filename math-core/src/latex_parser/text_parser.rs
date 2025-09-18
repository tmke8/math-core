use std::mem;

use crate::mathml_renderer::{
    arena::{Arena, StringBuilder},
    attribute::TextTransform,
    length::{Length, LengthUnit},
    symbol,
};

use super::{
    error::{LatexErrKind, LatexError},
    specifications::LatexUnit,
    token::{TokLoc, Token},
    token_manager::TokenManager,
};

/// Turn tokens into text, applying text transformations as needed.
///
/// This is, for example, used to parse the argument of `\text{...}`,
/// but also `\textbf{...}` and `\operatorname{...}`.
pub(super) struct TextParser<'arena, 'builder, 'source, 'parser> {
    builder: &'builder mut StringBuilder<'parser>,
    tokens: &'parser mut TokenManager<'source>,
    arena: &'arena Arena,
    tf: Option<TextTransform>,
}

impl<'arena, 'builder, 'source, 'parser> TextParser<'arena, 'builder, 'source, 'parser> {
    pub(super) fn new(
        builder: &'builder mut StringBuilder<'parser>,
        tokens: &'parser mut TokenManager<'source>,
        arena: &'arena Arena,
    ) -> Self {
        Self {
            builder,
            tokens,
            arena,
            tf: None,
        }
    }

    fn next_token(&mut self) -> Result<TokLoc<'source>, &'arena LatexError<'source>> {
        self.tokens.next(self.arena)
    }

    /// Parse the given token as text.
    ///
    /// This function may read in more tokens from the lexer, but it will always leave the last
    /// processed token in `peek`. This is important for turning of the text mode in the lexer at
    /// the right time.
    pub(super) fn parse_token_as_text(
        &mut self,
        tokloc: TokLoc<'source>,
    ) -> Result<(), &'arena LatexError<'source>> {
        let TokLoc(loc, token) = tokloc;
        let c: Result<char, LatexErrKind> = match token {
            Token::Letter(c) | Token::UprightLetter(c) => Ok(c),
            Token::Whitespace | Token::NonBreakingSpace => Ok('\u{A0}'),
            Token::Open(op) | Token::Close(op) => Ok(op.as_op().into()),
            Token::BinaryOp(op) => Ok(op.as_op().into()),
            Token::Relation(op) => Ok(op.as_op().into()),
            Token::SquareBracketOpen => Ok(symbol::LEFT_SQUARE_BRACKET.as_op().into()),
            Token::SquareBracketClose => Ok(symbol::RIGHT_SQUARE_BRACKET.as_op().into()),
            Token::Number(digit) => Ok(digit as u8 as char),
            Token::Prime => Ok('’'),
            Token::ForceRelation(op) => Ok(op.as_char()),
            Token::Punctuation(op) => Ok(op.as_op().into()),
            Token::PseudoOperator(name) | Token::PseudoOperatorLimits(name) => {
                // We don't transform these strings.
                self.builder.push_str(name);
                return Ok(());
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
                    return Ok(());
                }
            }
            Token::TextModeAccent(accent) => {
                // Discard `TextModeAccent` token.
                self.next_token()?;
                let tokloc = self.tokens.peek;
                self.parse_token_as_text(tokloc)?;
                self.builder.push_char(accent);
                return Ok(());
            }
            Token::Text(tf) => {
                // Discard `Text` token.
                self.next_token()?;
                let old_tf = mem::replace(&mut self.tf, tf);
                let tokloc = self.tokens.peek;
                self.parse_token_as_text(tokloc)?;
                self.tf = old_tf;
                return Ok(());
            }
            Token::GroupBegin => {
                // Discard opening token.
                self.next_token()?;
                while !matches!(self.tokens.peek.token(), Token::GroupEnd) {
                    let tokloc = self.tokens.peek;
                    self.parse_token_as_text(tokloc)?;
                    // Discard the last token.
                    // We must do this here, because `parse_token_in_text_mode` always leaves the
                    // last token in `peek`, but we want to continue here, so we need to discard it.
                    self.next_token()?;
                }
                return Ok(());
            }
            Token::Eof => Err(LatexErrKind::UnclosedGroup(Token::GroupEnd)),
            Token::End(_) | Token::Right | Token::GroupEnd => {
                Err(LatexErrKind::UnexpectedClose(token))
            }
            _ => Err(LatexErrKind::NotValidInTextMode(token)),
        };
        match c {
            Err(e) => Err(self.arena.alloc(LatexError(loc, e))),
            Ok(c) => {
                self.builder
                    .push_char(self.tf.map(|tf| tf.transform(c, false)).unwrap_or(c));
                Ok(())
            }
        }
    }
}
