use std::{cell::Cell, ops::Range};

use crate::ast::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NodeReference(usize);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StrReference(usize, usize);

impl From<StrReference> for Range<usize> {
    #[inline]
    fn from(reference: StrReference) -> Self {
        reference.0..reference.1
    }
}

pub struct NodeListElement<'source> {
    node: Node<'source>,
    next: Cell<Option<NodeReference>>,
}

pub struct Arena<'source> {
    buffer: String, // comes first because it hopefully doesn't need to grow
    nodes: Vec<NodeListElement<'source>>,
}

impl<'source> Arena<'source> {
    pub fn new() -> Self {
        Arena {
            buffer: String::new(),
            nodes: Vec::new(),
        }
    }

    pub fn push<'arena>(&'arena mut self, node: Node<'source>) -> NodeReference {
        let index = self.nodes.len();
        self.nodes.push(NodeListElement {
            node,
            next: Cell::new(None),
        });
        NodeReference(index)
    }

    pub fn get<'arena>(&'arena self, reference: NodeReference) -> &'arena NodeListElement<'source> {
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        unsafe { self.nodes.get_unchecked(reference.0) }
    }

    pub fn get_mut<'arena>(
        &'arena mut self,
        reference: NodeReference,
    ) -> &'arena mut NodeListElement<'source> {
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        unsafe { self.nodes.get_unchecked_mut(reference.0) }
    }
    pub fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) -> StrReference {
        let start = self.buffer.len();
        self.buffer.extend(iter);
        let end = self.buffer.len();
        StrReference(start, end)
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        let start = self.buffer.len();
        self.buffer.push_str(string);
        let end = self.buffer.len();
        StrReference(start, end)
    }

    pub fn get_str(&self, reference: StrReference) -> &str {
        &self.buffer[Range::<usize>::from(reference)]
    }
}

pub struct NodeList(Cell<Option<InhabitedNodeList>>);

#[derive(Copy, Clone)]
struct InhabitedNodeList {
    head: NodeReference,
    tail: NodeReference,
}

impl NodeList {
    pub fn new() -> Self {
        NodeList(Cell::new(None))
    }

    pub fn push<'source>(&self, arena: &mut Arena<'source>, node: Node<'source>) {
        // Add node to the arena and get a reference to it.
        let new_tail = arena.push(node);
        self.push_ref(arena, new_tail)
    }

    pub fn push_ref(&self, arena: &Arena<'_>, node_ref: NodeReference) {
        match self.0.get() {
            None => {
                self.0.set(Some(InhabitedNodeList {
                    head: node_ref,
                    tail: node_ref,
                }));
            }
            Some(InhabitedNodeList { head, tail }) => {
                // Update the tail to point to the new node.
                arena.get(tail).next.set(Some(node_ref));
                // Update the tail of the list.
                self.0.set(Some(InhabitedNodeList {
                    head,
                    tail: node_ref,
                }));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.0.get() {
            None => true,
            Some(_) => false,
        }
    }

    pub fn is_singleton(&self) -> bool {
        match self.0.get() {
            None => false,
            Some(list) => list.head == list.tail,
        }
    }

    pub fn iter<'arena, 'source: 'arena>(
        &self,
        arena: &'arena Arena<'source>,
    ) -> NodeListIterator<'arena, 'source> {
        NodeListIterator {
            arena,
            current: match self.0.get() {
                None => None,
                Some(list) => Some(list.head),
            },
        }
    }
}

pub struct NodeListIterator<'arena, 'source> {
    arena: &'arena Arena<'source>,
    current: Option<NodeReference>,
}

impl<'arena, 'source> Iterator for NodeListIterator<'arena, 'source> {
    type Item = &'arena Node<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => None,
            Some(reference) => {
                let node = self.arena.get(reference);
                self.current = node.next.get();
                Some(&node.node)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena() {
        let mut arena = Arena::new();
        let node = Node::Space("Hello, world!");
        let reference = arena.push(node);
        assert!(matches!(
            arena.get(reference).node,
            Node::Space("Hello, world!")
        ));
    }

    #[test]
    fn list() {
        let mut arena = Arena::new();
        let list = NodeList::new();
        list.push(&mut arena, Node::Space("Hello, world!"));
        list.push(&mut arena, Node::Space("Goodbye, world!"));
        let mut iter = list.iter(&arena);
        assert!(matches!(iter.next().unwrap(), Node::Space("Hello, world!")));
        assert!(matches!(
            iter.next().unwrap(),
            Node::Space("Goodbye, world!")
        ));
        assert!(iter.next().is_none());
    }

    #[test]
    fn list_singleton() {
        let mut arena = Arena::new();
        let list = NodeList::new();
        list.push(&mut arena, Node::Space("Hello, world!"));
        assert!(list.is_singleton());
        let mut iter = list.iter(&arena);
        assert!(matches!(iter.next().unwrap(), Node::Space("Hello, world!")));
        assert!(iter.next().is_none());
    }

    #[test]
    fn list_empty() {
        let arena = Arena::new();
        let list = NodeList::new();
        assert!(list.is_empty());
        let mut iter = list.iter(&arena);
        assert!(iter.next().is_none());
    }

    #[test]
    fn buffer_extend() {
        let mut arena = Arena::new();
        let str_ref = arena.extend("Hello, world!".chars());
        assert_eq!(arena.get_str(str_ref), "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let mut arena = Arena::new();
        let str_ref = arena.push_str("Hello, world!");
        assert_eq!(arena.get_str(str_ref), "Hello, world!");
    }
}
