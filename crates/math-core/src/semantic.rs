//! Infers the invisible operators for math equations.
//!
//! Some of the rules in this file are lifted from MathGrammar in LaTeXML,
//! but with significant simplification. In particular, they try to
//! infer '\u{2062}' (invisible times), but we don't, because it's too much
//! work to do correctly (and even LaTeXML gets it wrong in a few cases).
//!
//! Maybe that's a bad idea, and we should bite the bullet and use a
//! semantic IR? We'd have to parse things like `a + 2 \bmod b`, recognizing
//! `\bmod` as an infix operator, which isn't how we currently do it.
//!
//! The approach to parsing mostly comes from [matklad's][1] guide to PRATT parsing.
//!
//! [1]: https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
use alloc::{vec, vec::Vec};

use crate::parser::node_vec_to_node;
use mathml_renderer::arena::Arena;
use mathml_renderer::ast::Node;
use mathml_renderer::attribute::{MathSpacing, OpAttrs, OpRoles, RowAttrs};
use mathml_renderer::symbol::{FUNCTION_APPLICATION, INVISIBLE_TIMES};

pub struct EnrichParseResult<'arena> {
    consumed: usize,
    replaced_with: Option<Vec<&'arena Node<'arena>>>,
}

/// Convenience function to enrich and pass into a row.
pub fn enrich_to_node<'arena>(
    arena: &'arena Arena,
    input: &'_ [&'arena Node<'arena>],
    reset_spacing: bool,
) -> &'arena Node<'arena> {
    if reset_spacing
        && let [single] = input
        && let Node::Operator { op, attrs, roles, .. } = **single
    {
        return arena.push(Node::Operator {
            op,
            attrs,
            roles,
            left: None,
            right: None,
            size: None,
        });
    }
    node_vec_to_node(arena, &enrich(arena, input))
}

/// The base parsing function, which operates on a list of plain LaTeX AST
/// nodes and turns out another list of AST nodes, but enriched with invisible
/// operators.
pub fn enrich<'arena>(
    arena: &'arena Arena,
    input: &'_ [&'arena Node<'arena>],
) -> Vec<&'arena Node<'arena>> {
    let mut lhs: EnrichParseResult<'arena> = enrich_formula(arena, input, 0);
    while lhs.consumed < input.len() {
        let rhs: EnrichParseResult = enrich_formula(arena, &input[lhs.consumed..], 0);
        if rhs.consumed == 0 {
            // Failed to parse. Just add it to the end and keep going.
            if let Some(lhs_replaced_with) = lhs.replaced_with.as_mut() {
                lhs_replaced_with.push(&input[lhs.consumed]);
            }
            lhs.consumed += 1;
            continue;
        }
        if lhs.replaced_with.is_some() || rhs.replaced_with.is_some() {
            let mut lhs_replaced_with = lhs
                .replaced_with
                .take()
                .unwrap_or_else(|| input[..lhs.consumed].to_vec());
            lhs_replaced_with.extend_from_slice(
                rhs.replaced_with
                    .as_ref()
                    .map(|r| &r[..])
                    .unwrap_or(&input[lhs.consumed..][..rhs.consumed]),
            );
            lhs.replaced_with = Some(lhs_replaced_with);
        }
        lhs.consumed += rhs.consumed;
    }

    lhs.replaced_with
        .unwrap_or_else(|| input[..lhs.consumed].to_vec())
}

/// Enriches a single formula.
///
/// Returns early if it reaches an operator that binds tighter than `min_bp`.
fn enrich_formula<'tmp, 'arena>(
    arena: &'arena Arena,
    input: &'tmp [&'arena Node<'arena>],
    min_bp: u8,
) -> EnrichParseResult<'arena> {
    let mut lhs: EnrichParseResult<'arena> = {
        let mut lhs = EnrichParseResult {
            consumed: 0,
            replaced_with: None,
        };
        // include spaces in the LHS without affecting precedence
        while let Some(Node::Space(..)) = input.get(lhs.consumed) {
            // lhs.replaced_with is definitely None at this point
            lhs.consumed += 1;
        }
        if lhs.consumed >= input.len() {
            return lhs;
        }
        let new_lhs = if let Some(lhs_bracketed) = enrich_bracketed(arena, &input[lhs.consumed..]) {
            lhs_bracketed
        } else if let Some(lhs_prefix) = enrich_prefix(arena, &input[lhs.consumed..]) {
            lhs_prefix
        } else if let Some(lhs_fn) = enrich_pseudo_operator(arena, &input[lhs.consumed..]) {
            lhs_fn
        } else if infix_binding_power(input[lhs.consumed]).is_some() {
            // Bust out if this is an infix operator.
            // Let the caller deal with it.
            return EnrichParseResult {
                consumed: 0,
                replaced_with: None,
            };
        } else {
            // LHS is an atom
            EnrichParseResult {
                consumed: 1,
                replaced_with: None,
            }
        };
        if lhs.replaced_with.is_some() || new_lhs.replaced_with.is_some() {
            let lhs_replaced_with = lhs.replaced_with.get_or_insert_with(|| {
                let mut lhs_replaced_with = Vec::new();
                lhs_replaced_with.extend_from_slice(&input[..lhs.consumed]);
                lhs_replaced_with
            });
            if let Some(mut new_lhs_replaced_with) = new_lhs.replaced_with {
                lhs_replaced_with.append(&mut new_lhs_replaced_with);
            } else {
                lhs_replaced_with.extend_from_slice(&input[lhs.consumed..][..new_lhs.consumed]);
            }
        }
        lhs.consumed += new_lhs.consumed;
        lhs
    };
    while let Some([op, ..]) = input.get(lhs.consumed..) {
        // include spaces in the RHS without affecting precedence
        if let Node::Space(..) = op {
            // lhs.replaced_with is definitely None at this point
            lhs.consumed += 1;
            if let Some(lhs_replaced_with) = lhs.replaced_with.as_mut() {
                lhs_replaced_with.push(op);
            }
        } else if let Some((bp_l, ())) = postfix_binding_power(op) {
            if bp_l < min_bp {
                break;
            }
            if let Some(lhs_replaced_with) = lhs.replaced_with.as_mut() {
                lhs_replaced_with.push(op);
            }
            lhs.consumed += 1;
        } else {
            let (bp_r, push_op) = if let Some((bp_l, bp_r)) = infix_binding_power(op) {
                if bp_l < min_bp {
                    break;
                }
                lhs.consumed += 1;
                (bp_r, true)
            } else {
                let (bp_l, bp_r) = INFIX_BINDING_POWER_INVISIBLE_TIMES;
                if bp_l < min_bp {
                    break;
                }
                (bp_r, false)
            };
            let rhs: EnrichParseResult<'arena> =
                enrich_formula(arena, &input[lhs.consumed..], bp_r);
            if lhs.replaced_with.is_some() || rhs.replaced_with.is_some() {
                let mut lhs_replaced_with = lhs
                    .replaced_with
                    .map(|mut lhs_replaced_with| {
                        if push_op {
                            lhs_replaced_with.push(op);
                        }
                        lhs_replaced_with
                    })
                    .unwrap_or_else(|| input[..lhs.consumed].to_vec());
                if let Some(mut rhs_replaced_with) = rhs.replaced_with {
                    lhs_replaced_with.append(&mut rhs_replaced_with);
                } else {
                    lhs_replaced_with.extend_from_slice(&input[lhs.consumed..][..rhs.consumed]);
                }
                lhs.replaced_with = Some(lhs_replaced_with);
            }
            lhs.consumed += rhs.consumed;
        }
    }
    lhs
}

// LHS starts with a parenthesized list
// (1 + 2) * 3
// ^^^^^^^
fn enrich_bracketed<'tmp, 'arena>(
    arena: &'arena Arena,
    input: &'tmp [&'arena Node<'arena>],
) -> Option<EnrichParseResult<'arena>> {
    if input.is_empty() {
        return None;
    }
    let open = match operator(input.first()?)? {
        Node::Operator { roles, .. } if roles.contains(OpRoles::ROLE_OPEN) => input[0],
        _ => return None,
    };
    let mut lhs: EnrichParseResult<'arena> = EnrichParseResult {
        consumed: 0,
        replaced_with: None,
    };
    lhs.consumed += 1;
    // OpRoles::ROLE_CLOSE has an infix binding power of 0, so that it can parse as an infix
    // operator when it's unmatched.
    //
    // So, if we set min_bp to 1, enrich_formula will return control back to us if it finds one.
    let rhs: EnrichParseResult<'arena> = enrich_formula(arena, &input[lhs.consumed..], 1);
    let close = match operator(input.get(lhs.consumed + rhs.consumed)?)? {
        Node::Operator { roles, .. } if roles.contains(OpRoles::ROLE_CLOSE) => {
            input[lhs.consumed + rhs.consumed]
        }
        _ => return None,
    };
    if let Some(rhs_replaced_with) = rhs.replaced_with {
        // lhs.replaced_with is definitely None
        lhs.replaced_with = Some(
            [open][..]
                .iter()
                .chain(&rhs_replaced_with[..])
                .chain(&[close])
                .copied()
                .collect(),
        );
    }
    lhs.consumed += rhs.consumed + 1;
    Some(lhs)
}

// LHS starts with a prefix operator
// \not x \and y
// ^^^^^^
fn enrich_prefix<'tmp, 'arena>(
    arena: &'arena Arena,
    input: &'tmp [&'arena Node<'arena>],
) -> Option<EnrichParseResult<'arena>> {
    if input.is_empty() {
        return None;
    }
    let mut lhs: EnrichParseResult<'arena> = EnrichParseResult {
        consumed: 0,
        replaced_with: None,
    };
    let ((), bp_r) = prefix_binding_power(input.get(lhs.consumed)?)?;
    lhs.consumed += 1;
    let rhs: EnrichParseResult<'arena> = enrich_formula(arena, &input[lhs.consumed..], bp_r);
    if let Some(rhs_replaced_with) = rhs.replaced_with {
        // lhs.replaced_with is definitely None
        lhs.replaced_with = Some(
            input[..lhs.consumed]
                .iter()
                .chain(&rhs_replaced_with)
                .copied()
                .collect(),
        );
    }
    lhs.consumed += rhs.consumed;
    Some(lhs)
}

fn enrich_pseudo_operator<'tmp, 'arena>(
    arena: &'arena Arena,
    input: &'tmp [&'arena Node<'arena>],
) -> Option<EnrichParseResult<'arena>> {
    if let (
        identifier,
        Node::PseudoOp {
            name: _,
            left,
            force_movable_limits: _,
            right,
        },
    ) = rewrite_pseudo_operator(input.first()?, arena)?
    {
        // LHS starts with a prefix pseudo-operator
        // \sin x + y
        // ^^^^^^
        let mut lhs: EnrichParseResult<'arena> = EnrichParseResult {
            consumed: 1,
            replaced_with: None,
        };
        let rhs: EnrichParseResult<'arena> = enrich_formula(
            arena,
            &input[lhs.consumed..],
            PREFIX_BINDING_POWER_PSEUDO_OPERATOR.1,
        );
        if rhs.consumed == 0 {
            return None;
        }
        let mut replaced_with: Vec<&Node<'_>> = vec![
            arena.push(Node::Row {
                nodes: arena.push_slice(&[
                    identifier,
                    arena.push(Node::Operator {
                        op: FUNCTION_APPLICATION.as_op(),
                        attrs: OpAttrs::empty(),
                        roles: OpRoles::ROLE_INFIX,
                        // when the pseudo-op is being rewritten as a function, this U+2061
                        // is to the right of the function name, so its right becomes our left
                        left: *right,
                        right: None,
                        size: None,
                    }),
                    node_vec_to_node(
                        arena,
                        rhs.replaced_with
                            .as_ref()
                            .map(|r| &r[..])
                            .unwrap_or_else(|| &input[lhs.consumed..][..rhs.consumed]),
                    ),
                ]),
                attrs: RowAttrs::default(),
            }),
        ];
        if *left != Some(MathSpacing::Zero) {
            // this is a bit of a bold assumption, but we definitely are
            // placed next to *something*, or we would need no space
            replaced_with.insert(
                0,
                arena.push(Node::Operator {
                    op: INVISIBLE_TIMES.as_op(),
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    left: *left,
                    right: None,
                    size: None,
                }),
            );
        }
        lhs.replaced_with = Some(replaced_with);
        lhs.consumed += rhs.consumed;
        Some(lhs)
    } else {
        None
    }
}

const INFIX_BINDING_POWER_CLOSE: (u8, u8) = (0, 0);
const INFIX_BINDING_POWER: (u8, u8) = (1, 2);
// We pretend that there's an INVISIBLE_TIMES operator for parsing precedence,
// but we don't actually generate U+2062, because correctly generating it would
// require a lot more parsing logic. Even LaTeXML gets it wrong sometimes!
const INFIX_BINDING_POWER_INVISIBLE_TIMES: (u8, u8) = (3, 4);
const INFIX_BINDING_POWER_FUNCTION_APPLICATION: (u8, u8) = (5, 6);
const PREFIX_BINDING_POWER_PSEUDO_OPERATOR: ((), u8) = ((), 6);
const PREFIX_BINDING_POWER: ((), u8) = ((), 8);
const POSTFIX_BINDING_POWER: (u8, ()) = (9, ());

fn prefix_binding_power(node: &Node<'_>) -> Option<((), u8)> {
    Some(match operator(node)? {
        Node::Operator { roles, .. } if roles.contains(OpRoles::ROLE_PREFIX) => {
            PREFIX_BINDING_POWER
        }
        Node::PseudoOp {
            name: _,
            left: _,
            right: _,
            force_movable_limits: _,
        } => PREFIX_BINDING_POWER_PSEUDO_OPERATOR,
        _ => return None,
    })
}

fn postfix_binding_power(node: &Node<'_>) -> Option<(u8, ())> {
    Some(match operator(node)? {
        Node::Operator { roles, .. } if roles.contains(OpRoles::ROLE_POSTFIX) => {
            POSTFIX_BINDING_POWER
        }
        _ => return None,
    })
}

fn infix_binding_power(node: &Node<'_>) -> Option<(u8, u8)> {
    Some(match operator(node)? {
        Node::Operator { op, roles, .. } if roles.contains(OpRoles::ROLE_INFIX) => {
            match op.as_superchar().base_char() {
                '\u{2062}' => INFIX_BINDING_POWER_INVISIBLE_TIMES,
                '\u{2061}' => INFIX_BINDING_POWER_FUNCTION_APPLICATION,
                _ => INFIX_BINDING_POWER,
            }
        }
        Node::Operator { roles, .. } if roles.contains(OpRoles::ROLE_CLOSE) => {
            INFIX_BINDING_POWER_CLOSE
        }
        _ => return None,
    })
}

fn rewrite_pseudo_operator<'tmp, 'arena>(
    node: &'tmp Node<'arena>,
    arena: &'arena Arena,
) -> Option<(&'arena Node<'arena>, &'tmp Node<'arena>)> {
    let mut original = None;
    fn do_rewrite<'tmp, 'arena>(
        node: &'tmp Node<'arena>,
        arena: &'arena Arena,
        original: &mut Option<&'tmp Node<'arena>>,
        recursion: usize,
    ) -> Option<&'arena Node<'arena>> {
        if recursion >= 1000 {
            return None;
        }
        Some(arena.push(match node {
            Node::Sub { target, symbol } => Node::Sub {
                target: do_rewrite(target, arena, original, recursion + 1)?,
                symbol,
            },
            Node::Sup { target, symbol } => Node::Sup {
                target: do_rewrite(target, arena, original, recursion + 1)?,
                symbol,
            },
            Node::SubSup { target, sub, sup } => Node::SubSup {
                target: do_rewrite(target, arena, original, recursion + 1)?,
                sub,
                sup,
            },
            Node::Over { target, symbol } => Node::Over {
                target: do_rewrite(target, arena, original, recursion + 1)?,
                symbol,
            },
            Node::Under { target, symbol } => Node::Under {
                target: do_rewrite(target, arena, original, recursion + 1)?,
                symbol,
            },
            Node::UnderOver {
                target,
                over,
                under,
            } => Node::UnderOver {
                target: do_rewrite(target, arena, original, recursion + 1)?,
                over,
                under,
            },
            Node::PseudoOp { name, .. } => {
                *original = Some(node);
                Node::IdentifierStr(name)
            }
            _ => return None,
        }))
    }
    let rewritten = do_rewrite(node, arena, &mut original, 0)?;
    Some((rewritten, original?))
}

fn operator<'tmp, 'arena>(mut node: &'tmp Node<'arena>) -> Option<&'tmp Node<'arena>> {
    let mut recursion = 0;
    while recursion < 1000 {
        node = match node {
            Node::Sub { target, .. }
            | Node::Sup { target, .. }
            | Node::SubSup { target, .. }
            | Node::Under { target, .. }
            | Node::Over { target, .. }
            | Node::UnderOver { target, .. } => target,
            _ => break,
        };
        recursion += 1;
    }
    if let Node::Operator { .. } = node {
        Some(node)
    } else {
        None
    }
}

#[test]
fn enrich_test() {
    const FN_OP: mathml_renderer::symbol::MathMLOperator = const { FUNCTION_APPLICATION.as_op() };
    const ADD_OP: mathml_renderer::symbol::MathMLOperator =
        const { mathml_renderer::symbol::PLUS_SIGN.as_op() };
    use std::assert_matches;
    let arena = Arena::default();
    // Basic tests
    assert_matches!(
        enrich_to_node(&arena, &[&Node::Number("1")][..], false),
        &Node::Number("1")
    );
    assert_matches!(
        enrich_to_node(&arena, &[][..], false),
        &Node::Row { nodes: &[], .. }
    );
    // Basic infix operator tests
    assert_matches!(
        enrich_to_node(
            &arena,
            &[
                &Node::Number("1"),
                &Node::Operator {
                    op: ADD_OP,
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::Number("1"),
            ][..],
            false,
        ),
        &Node::Row {
            nodes: [
                Node::Number("1"),
                Node::Operator { op: ADD_OP, .. },
                Node::Number("1"),
            ],
            ..
        }
    );
    assert_matches!(
        enrich_to_node(
            &arena,
            &[
                &Node::Number("1"),
                &Node::Operator {
                    op: ADD_OP,
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::Number("1"),
                &Node::Operator {
                    op: ADD_OP,
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::IdentifierStr("x"),
            ][..],
            false,
        ),
        &Node::Row {
            nodes: [
                Node::Number("1"),
                Node::Operator { op: ADD_OP, .. },
                Node::Number("1"),
                Node::Operator { op: ADD_OP, .. },
                Node::IdentifierStr("x"),
            ],
            ..
        }
    );
    // Combining an infix op with \sin
    assert_matches!(
        enrich_to_node(
            &arena,
            &[
                &Node::PseudoOp {
                    left: Some(MathSpacing::Zero),
                    right: None,
                    name: "sin",
                    force_movable_limits: false,
                },
                &Node::Number("1"),
                &Node::Operator {
                    op: const { mathml_renderer::symbol::PLUS_SIGN.as_op() },
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::Number("1"),
            ][..],
            false,
        ),
        &Node::Row {
            nodes: [
                Node::Row {
                    nodes: [
                        Node::IdentifierStr("sin"),
                        Node::Operator { op: FN_OP, .. },
                        Node::Number("1"),
                    ],
                    ..
                },
                Node::Operator { op: ADD_OP, .. },
                Node::Number("1")
            ],
            ..
        }
    );
    // Invisible times combined with \sin
    assert_matches!(
        enrich_to_node(
            &arena,
            &[
                &Node::PseudoOp {
                    left: Some(MathSpacing::Zero),
                    right: None,
                    name: "sin",
                    force_movable_limits: false,
                },
                &Node::Number("1"),
                &Node::IdentifierStr("x"),
            ][..],
            false,
        ),
        &Node::Row {
            nodes: [
                Node::Row {
                    nodes: [
                        Node::IdentifierStr("sin"),
                        Node::Operator { op: FN_OP, .. },
                        Node::Number("1"),
                    ],
                    ..
                },
                Node::IdentifierStr("x")
            ],
            ..
        }
    );
    // Invisible times combined with infix operator
    assert_matches!(
        enrich_to_node(
            &arena,
            &[
                &Node::Number("1"),
                &Node::Operator {
                    op: const { mathml_renderer::symbol::PLUS_SIGN.as_op() },
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::Number("1"),
                &Node::IdentifierStr("x"),
                &Node::Operator {
                    op: const { mathml_renderer::symbol::PLUS_SIGN.as_op() },
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::Number("1"),
            ][..],
            false,
        ),
        &Node::Row {
            nodes: [
                Node::Number("1"),
                Node::Operator { op: ADD_OP, .. },
                Node::Number("1"),
                Node::IdentifierStr("x"),
                Node::Operator { op: ADD_OP, .. },
                Node::Number("1"),
            ],
            ..
        }
    );
    // Invisible times chain
    assert_matches!(
        enrich_to_node(
            &arena,
            &[
                &Node::IdentifierStr("a"),
                &Node::Operator {
                    op: const { mathml_renderer::symbol::PLUS_SIGN.as_op() },
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::IdentifierStr("b"),
                &Node::IdentifierStr("c"),
                &Node::IdentifierStr("d"),
                &Node::Operator {
                    op: const { mathml_renderer::symbol::PLUS_SIGN.as_op() },
                    attrs: OpAttrs::empty(),
                    roles: OpRoles::ROLE_INFIX,
                    size: None,
                    left: None,
                    right: None,
                },
                &Node::IdentifierStr("e"),
            ][..],
            false,
        ),
        &Node::Row {
            nodes: [
                Node::IdentifierStr("a"),
                Node::Operator { op: ADD_OP, .. },
                Node::IdentifierStr("b"),
                Node::IdentifierStr("c"),
                Node::IdentifierStr("d"),
                Node::Operator { op: ADD_OP, .. },
                Node::IdentifierStr("e"),
            ],
            ..
        }
    );
}
