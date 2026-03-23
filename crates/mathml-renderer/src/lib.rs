//! Internal library for the `math-core` crate for rendering MathML.
//!
//! This library allows you to construct an AST representing MathML and then render it to a string.
//!
//! # Example
//!
//! ```rust
//! use math_core_renderer_internal::ast::Node;
//! use math_core_renderer_internal::symbol;
//! use math_core_renderer_internal::attribute::{MathSpacing, LetterAttr, OpAttrs};
//!
//! let ast = Node::Row {
//!     nodes: &[
//!         &Node::Under {
//!             target: &Node::Operator {
//!                 op: symbol::N_ARY_SUMMATION.as_op(),
//!                 attrs: OpAttrs::empty(),
//!                 left: Some(MathSpacing::Zero),
//!                 right: None,
//!                 size: None,
//!             },
//!             symbol: &Node::IdentifierChar('i', LetterAttr::Default),
//!         },
//!         &Node::IdentifierChar('i', LetterAttr::Default),
//!      ],
//!      attr: None,
//! };
//!
//! let mut output = String::new();
//! ast.emit(&mut output, 0).unwrap();
//! assert_eq!(
//!     output,
//!     "<mrow><munder><mo lspace=\"0\">∑</mo><mi>i</mi></munder><mi>i</mi></mrow>"
//! );
//! ```
pub mod arena;
pub mod ast;
pub mod attribute;
pub mod fmt;
mod itoa;
pub mod length;
pub mod symbol;
pub mod table;
