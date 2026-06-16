//! Internal library for the `math-core` crate for rendering MathML.
//!
//! This library allows you to construct an AST representing MathML and then render it to a string.
//!
//! # Example
//!
//! ```rust
//! use rustc_hash::FxHashMap;
//!
//! use math_core_renderer_internal::ast::{Emitter, Node, RowAttrs};
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
//!             symbol: &Node::IdentifierChar('i'.into(), LetterAttr::Default),
//!         },
//!         &Node::IdentifierChar('i'.into(), LetterAttr::Default),
//!      ],
//!      attrs: RowAttrs::DEFAULT,
//! };
//!
//! let label_map = FxHashMap::default();
//! let mut emitter = Emitter::new(String::new(), &label_map);
//! emitter.emit(&ast, 0).unwrap();
//! let output = emitter.into_string();
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
pub mod super_char;
pub mod symbol;
pub mod table;
