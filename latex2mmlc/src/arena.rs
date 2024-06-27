use crate::ast::Node;
use crate::error::{ExpectOptim, GetUnwrap};
use core::num::NonZero;

/// A reference to a node in the arena.
/// This is a wrapper around an index into the arena's nodes vector.
/// The index is guaranteed to be non-zero, so that we can use NonZero<usize>
/// as the type for the index.
/// For any node, there should be only one NodeReference pointing to it;
/// otherwise, the arena's invariants are violated.
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct NodeReference(NonZero<usize>);

impl NodeReference {
    /// Convert a reference to a node by looking it up in the arena.
    /// This is a pretty cheap operation, as it's just a vector lookup.
    #[inline]
    pub fn as_node<'arena, 'source>(&self, arena: &'arena Arena<'source>) -> &'arena Node<'source> {
        &arena.get(self).node
    }

    #[inline]
    pub fn as_node_mut<'arena, 'source>(
        &self,
        arena: &'arena mut Arena<'source>,
    ) -> &'arena mut Node<'source> {
        &mut arena.get_mut(self).node
    }
}

#[derive(Debug)]
struct NodeListElement<'source> {
    node: Node<'source>,
    next: Option<NodeReference>,
}

#[derive(Debug)]
pub struct Arena<'source> {
    nodes: Vec<NodeListElement<'source>>,
}

impl<'source> Arena<'source> {
    pub fn new() -> Self {
        // We fill the arena with one dummy element, so that all indices
        // are non-zero. This allows us to use NonZero<usize> as the
        // NodeReference type.
        // TODO: Investigate the alternative of adding 1 to the index and
        //       then subtracting 1 when using it for the lookup.
        Arena {
            nodes: vec![NodeListElement {
                node: Node::RowSeparator,
                next: None,
            }],
        }
    }

    pub fn push(&mut self, node: Node<'source>) -> NodeReference {
        let index = self.nodes.len();
        let item = NodeListElement { node, next: None };
        self.nodes.push(item);
        // if matches!(self.nodes.try_reserve(1), Err(_)) {}
        // unsafe {
        //     self.nodes.as_mut_ptr().add(self.nodes.len()).write(item);
        //     self.nodes.set_len(self.nodes.len() + 1);
        // }
        NodeReference(NonZero::<usize>::new(index).expect_optim("index should be non-zero"))
    }

    fn get<'arena>(&'arena self, reference: &NodeReference) -> &'arena NodeListElement<'source> {
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        self.nodes
            .get(reference.0.get())
            .expect_optim("NodeReference should be a valid index")
    }

    fn get_mut<'arena>(
        &'arena mut self,
        reference: &NodeReference,
    ) -> &'arena mut NodeListElement<'source> {
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        self.nodes
            .get_mut(reference.0.get())
            .expect_optim("NodeReference should be a valid index")
    }
}

/// This helper type is there to make string slices at least a little bit safe.
#[derive(Debug)]
#[repr(transparent)]
pub struct StrBound(usize);

#[derive(Debug)]
pub struct StrReference(StrBound, StrBound);

impl StrReference {
    pub fn new(start: StrBound, end: StrBound) -> Self {
        debug_assert!(start.0 <= end.0);
        StrReference(start, end)
    }

    #[inline]
    pub fn as_str<'buffer>(&self, buffer: &'buffer Buffer) -> &'buffer str {
        buffer.get_str(self)
    }
}

#[derive(Debug)]
pub struct Buffer {
    buffer: String,
}

impl Buffer {
    pub fn new(size_hint: usize) -> Self {
        Buffer {
            buffer: String::with_capacity(size_hint),
        }
    }

    pub fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) -> StrReference {
        let start = self.end();
        self.buffer.extend(iter);
        let end = self.end();
        StrReference(start, end)
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        let start = self.end();
        self.buffer.push_str(string);
        let end = self.end();
        StrReference(start, end)
    }

    pub fn push(&mut self, ch: char) {
        self.buffer.push(ch);
    }

    fn get_str(&self, reference: &StrReference) -> &str {
        self.buffer.get_unwrap(reference.0 .0..reference.1 .0)
    }

    #[inline]
    pub fn end(&self) -> StrBound {
        StrBound(self.buffer.len())
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeListBuilder(Option<InhabitedNodeList>);

#[derive(Debug)]
struct InhabitedNodeList {
    head: NodeReference,
    tail: NodeReference,
}

pub enum SingletonOrList {
    List(NodeList),
    Singleton(NodeReference),
}

impl NodeListBuilder {
    pub fn new() -> Self {
        NodeListBuilder(None)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Push a node reference to the list.
    /// If the referenced node was already part of some other list,
    /// then that list will be broken.
    pub fn push(&mut self, arena: &mut Arena<'_>, node_ref: NodeReference) {
        // Duplicate the reference to the node.
        // This is a bit dangerous because it could lead to two references to one node,
        // but we will relinquish the second reference on the `.finish()` call.
        let new_tail = NodeReference(node_ref.0);
        match &mut self.0 {
            None => {
                self.0 = Some(InhabitedNodeList {
                    head: node_ref,
                    tail: new_tail,
                });
            }
            Some(InhabitedNodeList { tail, .. }) => {
                // We want to avoid cycles in the list, so we assert that the new node
                // has a higher index than the current tail.
                debug_assert!(tail.0 < node_ref.0, "list index should always increase");
                // Update the tail to point to the new node.
                arena.get_mut(tail).next = Some(node_ref);
                // Update the tail of the list.
                *tail = new_tail;
            }
        }
    }

    /// If the list contains exactly one element, return it.
    /// This is a very efficient operation, because we don't need to look up
    /// anything in the arena.
    pub fn as_singleton_or_finish(self) -> SingletonOrList {
        match self.0 {
            Some(list) if list.head == list.tail => SingletonOrList::Singleton(list.head),
            _ => SingletonOrList::List(self.finish()),
        }
    }

    /// Finish building the list and return it.
    /// This method consumes the builder.
    pub fn finish(self) -> NodeList {
        NodeList(self.0.map(|list| list.head))
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeList(Option<NodeReference>);

impl NodeList {
    #[inline]
    pub fn empty() -> Self {
        NodeList(None)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn from_two_nodes(
        arena: &mut Arena<'_>,
        first: NodeReference,
        second: NodeReference,
    ) -> Self {
        arena.get_mut(&first).next = Some(second);
        NodeList(Some(first))
    }

    pub fn iter<'arena, 'source>(
        &self,
        arena: &'arena Arena<'source>,
    ) -> NodeListIterator<'arena, 'source> {
        NodeListIterator {
            arena,
            current: self.0.as_ref().map(|reference| arena.get(reference)),
        }
    }

    /// Iterate over the list manually.
    ///
    /// This iterator cannot be used with a for loop, because the .next() method
    /// requires a reference to the arena. This is useful when you want to use
    /// a mutable reference to the arena within the loop body.
    pub fn into_man_iter(self) -> NodeListManualIterator {
        NodeListManualIterator { current: self.0 }
    }
}

pub struct NodeListIterator<'arena, 'source> {
    arena: &'arena Arena<'source>,
    current: Option<&'arena NodeListElement<'source>>,
}

impl<'arena, 'source> Iterator for NodeListIterator<'arena, 'source> {
    type Item = &'arena Node<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => None,
            Some(element) => {
                self.current = element
                    .next
                    .as_ref()
                    .map(|reference| self.arena.get(reference));
                Some(&element.node)
            }
        }
    }
}

pub struct NodeListManualIterator {
    current: Option<NodeReference>,
}

impl NodeListManualIterator {
    pub fn next<'arena, 'source>(
        &mut self,
        arena: &'arena mut Arena<'source>,
    ) -> Option<(NodeReference, &'arena Node<'source>)> {
        match self.current.take() {
            None => None,
            Some(reference) => {
                let element = arena.get_mut(&reference);
                // Ownership of the next reference is transferred to the iterator.
                // This ensures that returned elements can be added to new lists,
                // without having a "next" reference that points to an element in the old list.
                self.current = element.next.take();
                Some((reference, &element.node))
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
        assert_eq!(reference.0.get(), 1);
        assert!(matches!(
            reference.as_node(&arena),
            Node::Space("Hello, world!")
        ));
    }

    #[test]
    fn list() {
        let mut arena = Arena::new();
        let mut builder = NodeListBuilder::new();
        let node_ref = arena.push(Node::Space("Hello, world!"));
        builder.push(&mut arena, node_ref);
        let node_ref = arena.push(Node::Space("Goodbye, world!"));
        builder.push(&mut arena, node_ref);
        let list = builder.finish();
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
        let mut builder = NodeListBuilder::new();
        let node_ref = arena.push(Node::Space("Hello, world!"));
        builder.push(&mut arena, node_ref);
        if let SingletonOrList::Singleton(element) = builder.as_singleton_or_finish() {
            assert!(matches!(
                element.as_node(&arena),
                Node::Space("Hello, world!")
            ));
        } else {
            panic!("List should be a singleton");
        }
    }

    #[test]
    fn list_empty() {
        let arena = Arena::new();
        let builder = NodeListBuilder::new();
        let list = builder.finish();
        assert!(list.is_empty());
        let mut iter = list.iter(&arena);
        assert!(iter.next().is_none(), "Empty list should return None");
    }

    #[test]
    fn list_manual_iter() {
        let mut arena = Arena::new();
        let mut builder = NodeListBuilder::new();
        let node_ref = arena.push(Node::Space("Hello, world!"));
        builder.push(&mut arena, node_ref);
        let node_ref = arena.push(Node::Space("Goodbye, world!"));
        builder.push(&mut arena, node_ref);
        let list = builder.finish();
        let mut iter = list.into_man_iter();
        let (reference, node) = iter.next(&mut arena).unwrap();
        assert!(matches!(node, Node::Space("Hello, world!")));
        assert_eq!(reference.0.get(), 1);
        let (reference, node) = iter.next(&mut arena).unwrap();
        assert!(matches!(node, Node::Space("Goodbye, world!")));
        assert_eq!(reference.0.get(), 2);
    }

    #[test]
    fn buffer_extend() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.extend("Hello, world!".chars());
        assert_eq!(buffer.get_str(&str_ref), "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.push_str("Hello, world!");
        assert_eq!(buffer.get_str(&str_ref), "Hello, world!");
    }

    #[test]
    fn buffer_manual_reference() {
        let mut buffer = Buffer::new(0);
        let start = buffer.end();
        assert_eq!(start.0, 0);
        buffer.push('H');
        buffer.push('i');
        buffer.push('↩'); // This is a multi-byte character.
        let end = buffer.end();
        assert_eq!(end.0, 5);
        let str_ref = StrReference::new(start, end);
        assert_eq!(buffer.get_str(&str_ref), "Hi↩");
    }
}
