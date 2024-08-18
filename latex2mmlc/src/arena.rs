use std::ptr::NonNull;

use typed_arena::Arena;

use crate::{ast::Node, attribute::TextTransform};

pub type NodeRef<'arena, 'source> = &'arena mut NodeListElement<'arena, 'source>;

pub trait NodeArenaExt<'arena, 'source> {
    fn push(&self, node: Node<'arena, 'source>) -> &mut NodeListElement<'arena, 'source>;
}

impl<'arena, 'source> NodeArenaExt<'arena, 'source> for Arena<NodeListElement<'arena, 'source>> {
    fn push(&self, node: Node<'arena, 'source>) -> &mut NodeListElement<'arena, 'source> {
        self.alloc(NodeListElement { node, next: None })
    }
}

pub trait StrArenaExt {
    fn push_char(&self, c: char) -> &str;
    fn extend<I>(&self, iterator: I) -> &str
    where
        I: Iterator<Item = char>;
    fn transform_and_push<'arena>(&'arena self, input: &str, tf: TextTransform) -> &'arena str {
        self.extend(input.chars().map(|c| tf.transform(c)))
    }
    fn end(&self) -> (usize,) {
        (0,)
    }
}

impl StrArenaExt for Arena<u8> {
    fn push_char(&self, c: char) -> &str {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.alloc_str(s)
    }
    fn extend<I>(&self, iterator: I) -> &str
    where
        I: Iterator<Item = char>,
    {
        let u8_ref = self.alloc_extend(Utf8Iterator::new(iterator));
        unsafe { std::str::from_utf8_unchecked(u8_ref) }
    }
}

struct Utf8Iterator<I>
where
    I: Iterator<Item = char>,
{
    chars: I,
    buffer: [u8; 4],
    buffer_pos: usize,
    buffer_len: usize,
}

impl<I> Utf8Iterator<I>
where
    I: Iterator<Item = char>,
{
    fn new(chars: I) -> Self {
        Utf8Iterator {
            chars,
            buffer: [0; 4],
            buffer_pos: 0,
            buffer_len: 0,
        }
    }
}

impl<I> Iterator for Utf8Iterator<I>
where
    I: Iterator<Item = char>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer_pos < self.buffer_len && self.buffer_pos < 4 {
            let byte = self.buffer[self.buffer_pos];
            self.buffer_pos += 1;
            Some(byte)
        } else {
            if let Some(ch) = self.chars.next() {
                let s = ch.encode_utf8(&mut self.buffer);
                self.buffer_len = s.len();
                self.buffer_pos = 1;
                Some(self.buffer[0])
            } else {
                None
            }
        }
    }
}

pub struct StrReference;

impl StrReference {
    pub fn new(start: (usize,), end: (usize,)) -> &'static str {
        ""
    }
}

#[derive(Debug)]
pub struct NodeListElement<'arena, 'source> {
    pub node: Node<'arena, 'source>,
    // next: Option<NonNull<NodeListElement<'arena, 'source>>>,
    next: Option<NodeRef<'arena, 'source>>,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeListBuilder<'arena, 'source>(Option<InhabitedNodeList<'arena, 'source>>);

#[derive(Debug)]
struct InhabitedNodeList<'arena, 'source> {
    head: NodeRef<'arena, 'source>,
    // tail: NodeRef<'arena, 'source>,
    // head: NonNull<NodeListElement<'arena, 'source>>,
    tail: NonNull<NodeListElement<'arena, 'source>>,
}

pub enum SingletonOrList<'arena, 'source> {
    List(NodeList<'arena, 'source>),
    Singleton(NodeRef<'arena, 'source>),
}

impl Default for NodeListBuilder<'_, '_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'arena, 'source> NodeListBuilder<'arena, 'source> {
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
    pub fn push(&mut self, node_ref: NodeRef<'arena, 'source>) {
        // Duplicate the reference to the node.
        // This is a bit dangerous because it could lead to two references to one node,
        // but we will relinquish the second reference on the `.finish()` call.
        // let new_tail = NonNull::from(node_ref);
        let new_tail: *mut _ = &mut *node_ref;
        let new_tail = unsafe { NonNull::new_unchecked(new_tail) };
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
                debug_assert!(*tail < new_tail, "list index should always increase");
                // Update the tail to point to the new node.
                unsafe {
                    tail.as_mut().next = Some(node_ref);
                };
                // Update the tail of the list.
                *tail = new_tail;
            }
        }
    }

    /// If the list contains exactly one element, return it.
    /// This is a very efficient operation, because we don't need to look up
    /// anything in the arena.
    pub fn as_singleton_or_finish(self) -> SingletonOrList<'arena, 'source> {
        match self.0 {
            Some(list) => {
                if NonNull::new(list.head as *mut _) == Some(list.tail) {
                    SingletonOrList::Singleton(list.head)
                } else {
                    SingletonOrList::List(NodeList(Some(list.head)))
                }
            }
            _ => SingletonOrList::List(self.finish()),
        }
    }

    /// Finish building the list and return it.
    /// This method consumes the builder.
    pub fn finish(self) -> NodeList<'arena, 'source> {
        NodeList(self.0.map(|list| list.head))
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeList<'arena, 'source>(Option<NodeRef<'arena, 'source>>);

impl<'arena, 'source> NodeList<'arena, 'source> {
    #[inline]
    pub fn empty() -> Self {
        NodeList(None)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn from_two_nodes(
        first: NodeRef<'arena, 'source>,
        second: NodeRef<'arena, 'source>,
    ) -> Self {
        first.next = Some(second.into());
        NodeList(Some(first))
    }

    pub fn iter(&'arena self) -> NodeListIterator<'arena, 'source> {
        NodeListIterator {
            current: self.0.as_ref().map(|reference| &**reference),
        }
    }

    /// Iterate over the list manually.
    ///
    /// This iterator cannot be used with a for loop, because the .next() method
    /// requires a reference to the arena. This is useful when you want to use
    /// a mutable reference to the arena within the loop body.
    pub fn into_iter(self) -> NodeListManualIterator<'arena, 'source> {
        NodeListManualIterator { current: self.0 }
    }
}

pub struct NodeListIterator<'arena, 'source> {
    current: Option<&'arena NodeListElement<'arena, 'source>>,
}

impl<'arena, 'source> Iterator for NodeListIterator<'arena, 'source> {
    type Item = &'arena Node<'arena, 'source>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => None,
            Some(element) => {
                self.current = element.next.as_ref().map(|next| &**next);
                Some(&element.node)
            }
        }
    }
}

pub struct NodeListManualIterator<'arena, 'source> {
    current: Option<NodeRef<'arena, 'source>>,
}

impl<'arena, 'source> Iterator for NodeListManualIterator<'arena, 'source> {
    type Item = NodeRef<'arena, 'source>;
    fn next(&mut self) -> Option<NodeRef<'arena, 'source>> {
        match self.current.take() {
            None => None,
            Some(reference) => {
                // Ownership of the next reference is transferred to the iterator.
                // This ensures that returned elements can be added to new lists,
                // without having a "next" reference that points to an element in the old list.
                self.current = reference.next.take();
                Some(reference)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_test() {
        let arena = Arena::new();
        let node = Node::Space("Hello, world!");
        let reference = arena.push(node);
        assert!(matches!(reference.node, Node::Space("Hello, world!")));
    }

    #[test]
    fn list() {
        let arena = Arena::new();
        let mut builder = NodeListBuilder::new();
        let node_ref = arena.push(Node::Space("Hello, world!"));
        builder.push(node_ref);
        let node_ref = arena.push(Node::Space("Goodbye, world!"));
        builder.push(node_ref);
        let list = builder.finish();
        let mut iter = list.iter();
        assert!(matches!(iter.next().unwrap(), Node::Space("Hello, world!")));
        assert!(matches!(
            iter.next().unwrap(),
            Node::Space("Goodbye, world!")
        ));
        assert!(iter.next().is_none());
    }

    #[test]
    fn list_singleton() {
        let arena = Arena::new();
        let mut builder = NodeListBuilder::new();
        let node_ref = arena.push(Node::Space("Hello, world!"));
        builder.push(node_ref);
        if let SingletonOrList::Singleton(element) = builder.as_singleton_or_finish() {
            assert!(matches!(element.node, Node::Space("Hello, world!")));
        } else {
            panic!("List should be a singleton");
        }
    }

    #[test]
    fn list_empty() {
        let builder = NodeListBuilder::new();
        let list = builder.finish();
        assert!(list.is_empty());
        let mut iter = list.iter();
        assert!(iter.next().is_none(), "Empty list should return None");
    }

    #[test]
    fn list_manual_iter() {
        let arena = Arena::new();
        let mut builder = NodeListBuilder::new();
        let node_ref = arena.push(Node::Space("Hello, world!"));
        builder.push(node_ref);
        let node_ref = arena.push(Node::Space("Goodbye, world!"));
        builder.push(node_ref);
        let list = builder.finish();
        let mut iter = list.into_iter();
        let reference = iter.next().unwrap();
        assert!(matches!(reference.node, Node::Space("Hello, world!")));
        let reference = iter.next().unwrap();
        assert!(matches!(reference.node, Node::Space("Goodbye, world!")));
    }

    #[test]
    fn buffer_extend() {
        let buffer = Arena::<u8>::new();
        let str_ref = buffer.extend("Hello, world!".chars());
        assert_eq!(str_ref, "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let buffer = Arena::<u8>::new();
        let str_ref = buffer.alloc_str("Hello, world!");
        assert_eq!(str_ref, "Hello, world!");
    }

    #[test]
    fn buffer_manual_reference() {
        let buffer = Arena::<u8>::new();
        let start = buffer.end();
        assert_eq!(start.0, 0);
        buffer.push_char('H');
        buffer.push_char('i');
        buffer.push_char('↩'); // This is a multi-byte character.
        let end = buffer.end();
        assert_eq!(end.0, 5);
        let str_ref = StrReference::new(start, end);
        // assert_eq!(buffer.get_str(&str_ref), "Hi↩");
    }

    #[test]
    fn utf8_iter() {
        let chars = "aß".chars();
        let mut iter = Utf8Iterator::new(chars);
        assert_eq!(iter.next(), Some(b'a'));
        assert_eq!(iter.next(), Some(0xC3));
        assert_eq!(iter.next(), Some(0x9F));
        assert_eq!(iter.next(), None);
    }

    struct CycleParticipant<'a> {
        val: i32,
        next: Option<&'a mut CycleParticipant<'a>>,
    }

    #[test]
    fn basic_arena() {
        let arena = Arena::new();

        let a = arena.alloc(CycleParticipant { val: 1, next: None });
        let b = arena.alloc(CycleParticipant { val: 2, next: None });
        a.next = Some(b);
        let c = arena.alloc(CycleParticipant { val: 3, next: None });
        a.next.as_mut().unwrap().next = Some(c);

        // for (i, node) in arena.iter_mut().enumerate() {
        //     match i {
        //         0 => assert_eq!(node.val, 1),
        //         1 => assert_eq!(node.val, 2),
        //         2 => assert_eq!(node.val, 3),
        //         _ => panic!("Too many nodes"),
        //     }
        // }

        assert_eq!(a.val, 1);
        assert_eq!(a.next.as_ref().unwrap().val, 2);
        assert_eq!(a.next.as_ref().unwrap().next.as_ref().unwrap().val, 3);
    }
}
