use crate::ast::Node;
use core::num::NonZero;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct NodeReference(NonZero<usize>);

impl NodeReference {
    /// Convert a reference to a node by looking it up in the arena.
    #[inline]
    pub fn as_node<'arena, 'source>(&self, arena: &'arena Arena<'source>) -> &'arena Node<'source> {
        arena.lookup(*self)
    }

    #[inline]
    pub fn as_node_mut<'arena, 'source>(
        &self,
        arena: &'arena mut Arena<'source>,
    ) -> &'arena mut Node<'source> {
        arena.lookup_mut(*self)
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
        debug_assert!(index != 0, "NodeReference index should never be zero");
        NodeReference(unsafe { NonZero::<usize>::new_unchecked(index) })
    }

    fn lookup(&self, reference: NodeReference) -> &Node<'source> {
        &self.get_raw(reference).node
    }

    fn get_raw<'arena>(&'arena self, reference: NodeReference) -> &'arena NodeListElement<'source> {
        debug_assert!(reference.0.get() < self.nodes.len());
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        unsafe { self.nodes.get(reference.0.get()).unwrap_unchecked() }
    }

    fn get_raw_mut<'arena>(
        &'arena mut self,
        reference: NodeReference,
    ) -> &'arena mut NodeListElement<'source> {
        debug_assert!(reference.0.get() < self.nodes.len());
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        unsafe { self.nodes.get_unchecked_mut(reference.0.get()) }
    }

    fn lookup_mut<'arena>(
        &'arena mut self,
        reference: NodeReference,
    ) -> &'arena mut Node<'source> {
        &mut self.get_raw_mut(reference).node
    }
}

/// This helper type is there to make string slices at least a little bit safe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct StrBound(usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StrReference(StrBound, StrBound);

impl StrReference {
    pub fn new(start: StrBound, end: StrBound) -> Self {
        debug_assert!(start.0 <= end.0);
        StrReference(start, end)
    }

    #[inline]
    pub fn as_str<'buffer>(&self, buffer: &'buffer Buffer) -> &'buffer str {
        buffer.get_str(*self)
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

    fn get_str(&self, reference: StrReference) -> &str {
        debug_assert!(reference.1 .0 <= self.buffer.len());
        // &self.buffer[Range::<usize>::from(reference)]
        unsafe { self.buffer.get_unchecked(reference.0 .0..reference.1 .0) }
    }

    #[inline]
    pub fn end(&self) -> StrBound {
        StrBound(self.buffer.len())
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct NodeListBuilder(Option<InhabitedNodeList>);

#[derive(Debug, Clone)]
struct InhabitedNodeList {
    head: NodeReference,
    tail: NodeReference,
}

impl NodeListBuilder {
    pub fn new() -> Self {
        NodeListBuilder(None)
    }

    /// Push a node to the list.
    pub fn push<'source>(&mut self, arena: &mut Arena<'source>, node: Node<'source>) {
        // Add node to the arena and get a reference to it.
        let new_tail = arena.push(node);
        self.push_ref(arena, new_tail)
    }

    /// Push a node reference to the list.
    /// This method is a bit dangerous, because if the referenced node was already
    /// part of some other list, then that list will be broken.
    pub fn push_ref(&mut self, arena: &mut Arena<'_>, node_ref: NodeReference) {
        match &mut self.0 {
            None => {
                self.0 = Some(InhabitedNodeList {
                    head: node_ref,
                    tail: node_ref,
                });
            }
            Some(InhabitedNodeList { head: _, tail }) => {
                // Update the tail to point to the new node.
                arena.get_raw_mut(*tail).next = Some(node_ref);
                // Update the tail of the list.
                *tail = node_ref;
            }
        }
    }

    /// If the list contains exactly one element, return it.
    /// This is a very efficient operation, because we don't need to look up
    /// anything in the arena.
    pub fn is_singleton(&self) -> Option<NodeReference> {
        match &self.0 {
            None => None,
            Some(list) => {
                if list.head == list.tail {
                    Some(list.head)
                } else {
                    None
                }
            }
        }
    }

    /// Finish building the list and return it.
    /// This method consumes the builder.
    pub fn finish(self) -> NodeList {
        NodeList(self.0.map(|list| list.head))
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct NodeList(Option<NodeReference>);

impl NodeList {
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn iter<'arena, 'source>(
        &self,
        arena: &'arena Arena<'source>,
    ) -> NodeListIterator<'arena, 'source> {
        NodeListIterator {
            arena,
            current: self.0,
        }
    }

    /// Iterate over the list manually.
    ///
    /// This iterator cannot be used with a for loop, because the .next() method
    /// requires a reference to the arena. This is useful when you want to use
    /// a mutable reference to the arena within the loop body.
    pub fn iter_manually(&self) -> NodeListManualIterator {
        NodeListManualIterator { current: self.0 }
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
                let node = self.arena.get_raw(reference);
                self.current = node.next;
                Some(&node.node)
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
        arena: &'arena Arena<'source>,
    ) -> Option<(NodeReference, &'arena Node<'source>)> {
        match self.current {
            None => None,
            Some(reference) => {
                let node = arena.get_raw(reference);
                self.current = node.next;
                Some((reference, &node.node))
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
            arena.lookup(reference),
            Node::Space("Hello, world!")
        ));
    }

    #[test]
    fn list() {
        let mut arena = Arena::new();
        let mut builder = NodeListBuilder::new();
        builder.push(&mut arena, Node::Space("Hello, world!"));
        builder.push(&mut arena, Node::Space("Goodbye, world!"));
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
        builder.push(&mut arena, Node::Space("Hello, world!"));
        assert!(builder.is_singleton().is_some());
        let list = builder.finish();
        let mut iter = list.iter(&arena);
        assert!(matches!(iter.next().unwrap(), Node::Space("Hello, world!")));
        assert!(iter.next().is_none());
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
        builder.push(&mut arena, Node::Space("Hello, world!"));
        builder.push(&mut arena, Node::Space("Goodbye, world!"));
        let list = builder.finish();
        let mut iter = list.iter_manually();
        let (reference, node) = iter.next(&arena).unwrap();
        assert!(matches!(node, Node::Space("Hello, world!")));
        assert_eq!(reference.0.get(), 1);
        let (reference, node) = iter.next(&arena).unwrap();
        assert!(matches!(node, Node::Space("Goodbye, world!")));
        assert_eq!(reference.0.get(), 2);
    }

    #[test]
    fn buffer_extend() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.extend("Hello, world!".chars());
        assert_eq!(buffer.get_str(str_ref), "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.push_str("Hello, world!");
        assert_eq!(buffer.get_str(str_ref), "Hello, world!");
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
        assert_eq!(buffer.get_str(str_ref), "Hi↩");
    }
}
