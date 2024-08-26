use std::marker::PhantomData;
use std::ptr::NonNull;

use bumpalo::collections::String as BumpString;
use bumpalo::Bump;

use crate::ast::Node;
use crate::attribute::TextTransform;
use crate::error::GetUnwrap;

#[derive(Debug)]
pub struct NodeListElement<'arena, 'source> {
    pub node: Node<'arena, 'source>,
    next: Option<NonNull<NodeListElement<'arena, 'source>>>,
}

pub type NodeRef<'arena, 'source> = &'arena mut NodeListElement<'arena, 'source>;

pub struct Arena<'source> {
    pub bump: Bump,
    phantom: PhantomData<&'source ()>,
}

impl<'source> Arena<'source> {
    pub fn new() -> Self {
        Arena {
            bump: Bump::new(),
            phantom: PhantomData,
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub fn push<'arena>(
        &'arena self,
        node: Node<'arena, 'source>,
    ) -> &mut NodeListElement<'arena, 'source> {
        // This fails if the bump allocator is out of memory.
        self.bump
            .try_alloc_with(|| NodeListElement { node, next: None })
            .unwrap_or_else(|_| std::process::abort())
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub fn push<'arena>(
        &'arena self,
        node: Node<'arena, 'source>,
    ) -> &mut NodeListElement<'arena, 'source> {
        self.bump
            .alloc_with(|| NodeListElement { node, next: None })
    }

    #[inline]
    pub fn push_str<'arena>(&'arena self, s: &str) -> &'arena str {
        self.bump.alloc_str(s)
    }

    pub fn extend<'arena, I: IntoIterator<Item = char>>(&'arena self, iter: I) -> &'arena str {
        let s = BumpString::from_iter_in(iter, &self.bump);
        // s.shrink_to_fit();
        s.into_bump_str()
    }

    pub fn transform_and_push<'arena>(&'arena self, input: &str, tf: TextTransform) -> &'arena str {
        self.extend(input.chars().map(|c| tf.transform(c)))
    }

    pub fn get_builder<'arena>(&'arena self) -> BumpString<'arena> {
        BumpString::new_in(&self.bump)
    }
}

impl Default for Arena<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// This helper type is there to make string slices at least a little bit safe.
#[derive(Debug)]
#[repr(transparent)]
struct StrBound(usize);

#[derive(Debug)]
pub struct StrReference(StrBound, StrBound);

impl StrReference {
    #[inline]
    pub fn as_str<'buffer>(&self, buffer: &'buffer Buffer) -> &'buffer str {
        buffer.buffer.get_unwrap(self.0 .0..self.1 .0)
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

    #[inline]
    pub fn extend<I: Iterator<Item = char>>(&mut self, iter: I) -> StrReference {
        let start = self.end();
        self.buffer.extend(iter);
        let end = self.end();
        StrReference(start, end)
    }

    /// Copy the contents of the given reference to the end of the buffer.
    ///
    /// If the given reference is invalid, this function will panic.
    /// However, on WASM, this function will instead do nothing.
    fn extend_from_within(&mut self, reference: &StrReference) -> StrReference {
        let start = self.end();
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(self.buffer.is_char_boundary(reference.0 .0));
            assert!(self.buffer.is_char_boundary(reference.1 .0));
            assert!(reference.0 .0 <= reference.1 .0);
            assert!(reference.1 .0 <= self.buffer.len());
        }
        // SAFETY: the bounds have been checked above
        unsafe {
            let begin = reference.0 .0;
            let end = reference.1 .0;
            let as_vec = self.buffer.as_mut_vec();
            // The following conditions should always hold true, but we check them
            // so that the compiler knows that this cannot panic.
            if begin <= end && begin < as_vec.len() && end <= as_vec.len() {
                as_vec.extend_from_within(begin..end);
            }
        }
        let end = self.end();
        StrReference(start, end)
    }

    pub fn transform_and_push(&mut self, input: &str, tf: TextTransform) -> StrReference {
        self.extend(input.chars().map(|c| tf.transform(c)))
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        let start = self.end();
        self.buffer.push_str(string);
        let end = self.end();
        StrReference(start, end)
    }

    #[inline]
    fn end(&self) -> StrBound {
        StrBound(self.buffer.len())
    }

    pub fn get_builder(&mut self) -> StringBuilder {
        StringBuilder::new(self)
    }
}

/// A helper type to safely build a string in the buffer from multiple pieces.
///
/// This takes an exclusive reference to the buffer and keeps track of the start
/// of the string being built. This guarantees that upon finishing, the string
/// has valid bounds and nothing else was written to the buffer in the meantime.
pub struct StringBuilder<'buffer> {
    buffer: &'buffer mut Buffer,
    start: StrBound,
}

impl<'buffer> StringBuilder<'buffer> {
    pub fn new(buffer: &'buffer mut Buffer) -> Self {
        let start = buffer.end();
        StringBuilder { buffer, start }
    }

    pub fn extend_from_within(&mut self, reference: &StrReference) -> StrReference {
        self.buffer.extend_from_within(reference)
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        self.buffer.push_str(string)
    }

    pub fn push_char(&mut self, ch: char) {
        self.buffer.buffer.push(ch);
    }

    pub fn transform_and_push(&mut self, input: &str, tf: TextTransform) -> StrReference {
        self.buffer.transform_and_push(input, tf)
    }

    pub fn finish(self) -> StrReference {
        StrReference(self.start, self.buffer.end())
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeListBuilder<'arena, 'source>(Option<InhabitedNodeList<'arena, 'source>>);

#[derive(Debug)]
struct InhabitedNodeList<'arena, 'source> {
    head: NonNull<NodeListElement<'arena, 'source>>,
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
        // We need to work with raw pointers here, because we want *two* mutable references
        // to the last element of the list.
        let new_tail = NonNull::from(node_ref);
        match &mut self.0 {
            None => {
                self.0 = Some(InhabitedNodeList {
                    head: new_tail,
                    tail: new_tail,
                });
            }
            Some(InhabitedNodeList { tail, .. }) => {
                // Update the tail to point to the new node.
                unsafe {
                    tail.as_mut().next = Some(new_tail);
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
            Some(mut list) if list.head == list.tail => {
                SingletonOrList::Singleton(unsafe { list.head.as_mut() })
            }
            _ => SingletonOrList::List(self.finish()),
        }
    }

    /// Finish building the list and return it.
    /// This method consumes the builder.
    pub fn finish(self) -> NodeList<'arena, 'source> {
        NodeList(self.0.map(|mut list| unsafe { list.head.as_mut() }))
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
        first.next = Some(NonNull::from(second));
        NodeList(Some(first))
    }

    pub fn iter(&'arena self) -> NodeListIterator<'arena, 'source> {
        NodeListIterator {
            current: self.0.as_deref(),
        }
    }
}

impl<'arena, 'source> IntoIterator for NodeList<'arena, 'source> {
    type Item = NodeRef<'arena, 'source>;
    type IntoIter = NodeListIntoIter<'arena, 'source>;

    /// Iterate over the list manually.
    ///
    /// This iterator cannot be used with a for loop, because the .next() method
    /// requires a reference to the arena. This is useful when you want to use
    /// a mutable reference to the arena within the loop body.
    fn into_iter(self) -> NodeListIntoIter<'arena, 'source> {
        NodeListIntoIter { current: self.0 }
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
                self.current = element.next.map(|next| unsafe { next.as_ref() });
                Some(&element.node)
            }
        }
    }
}

pub struct NodeListIntoIter<'arena, 'source> {
    current: Option<NodeRef<'arena, 'source>>,
}

impl<'arena, 'source> Iterator for NodeListIntoIter<'arena, 'source> {
    type Item = NodeRef<'arena, 'source>;
    fn next(&mut self) -> Option<NodeRef<'arena, 'source>> {
        match self.current.take() {
            None => None,
            Some(reference) => {
                // Ownership of the next reference is transferred to the iterator.
                // This ensures that returned elements can be added to new lists,
                // without having a "next" reference that points to an element in the old list.
                self.current = reference.next.take().map(|mut r| unsafe { r.as_mut() });
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
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.extend("Hello, world!".chars());
        assert_eq!(str_ref.as_str(&buffer), "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let mut buffer = Buffer::new(0);
        let str_ref = buffer.push_str("Hello, world!");
        assert_eq!(str_ref.as_str(&buffer), "Hello, world!");
    }

    #[test]
    fn buffer_manual_reference() {
        let mut buffer = Buffer::new(0);
        let mut builder = buffer.get_builder();
        assert_eq!(builder.start.0, 0);
        builder.push_char('H');
        builder.push_char('i');
        builder.push_char('↩'); // This is a multi-byte character.
        let str_ref = builder.finish();
        assert_eq!(str_ref.1 .0, 5);
        assert_eq!(str_ref.as_str(&buffer), "Hi↩");
    }

    struct CycleParticipant<'a> {
        val: i32,
        next: Option<&'a mut CycleParticipant<'a>>,
    }

    #[test]
    fn basic_arena() {
        let arena = Bump::new();

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
