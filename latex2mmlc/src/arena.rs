use std::{alloc::Layout, ptr::NonNull};

use bumpalo::{AllocErr, Bump};

use crate::{ast::Node, attribute::TextTransform};

#[derive(Debug)]
pub struct NodeListElement<'arena> {
    pub node: Node<'arena>,
    next: Option<NonNull<NodeListElement<'arena>>>,
}

pub type NodeRef<'arena> = &'arena mut NodeListElement<'arena>;

pub struct Arena {
    bump: Bump,
}

impl Arena {
    pub fn new() -> Self {
        Arena { bump: Bump::new() }
    }

    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub fn push<'arena>(&'arena self, node: Node<'arena>) -> &mut NodeListElement<'arena> {
        // This fails if the bump allocator is out of memory.
        self.bump
            .try_alloc_with(|| NodeListElement { node, next: None })
            .unwrap_or_else(|_| std::process::abort())
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub fn push<'arena>(&'arena self, node: Node<'arena>) -> &mut NodeListElement<'arena> {
        self.bump
            .alloc_with(|| NodeListElement { node, next: None })
    }

    #[inline(always)]
    fn alloc_str(&self, src: &str) -> &str {
        let buffer = self
            .try_alloc_slice_copy(src.as_bytes())
            .unwrap_or_else(|_| std::process::abort());
        unsafe {
            // This is OK, because it already came in as str, so it is guaranteed to be utf8
            std::str::from_utf8_unchecked(buffer)
        }
    }
    #[inline(always)]
    fn try_alloc_slice_copy<T>(&self, src: &[T]) -> Result<&mut [T], AllocErr>
    where
        T: Copy,
    {
        let layout = Layout::for_value(src);
        let dst = self.bump.try_alloc_layout(layout)?.cast::<T>();

        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
            Ok(std::slice::from_raw_parts_mut(dst.as_ptr(), src.len()))
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Buffer(String);

impl Buffer {
    pub fn new(size_hint: usize) -> Self {
        Buffer(String::with_capacity(size_hint))
    }

    pub fn get_builder(&mut self) -> StringBuilder<'_> {
        StringBuilder::new(self)
    }

    fn transform_and_append(&mut self, input: &str, tf: TextTransform) {
        self.0.extend(input.chars().map(|c| tf.transform(c)))
    }
}

/// A helper type to safely build a string in the buffer from multiple pieces.
///
/// It takes an exclusive reference to the buffer and clears everything in the
/// buffer before we start building. This guarantees that upon finishing, the
/// buffer contains only what we wrote to it.
pub struct StringBuilder<'buffer> {
    buffer: &'buffer mut Buffer,
}

impl<'buffer> StringBuilder<'buffer> {
    pub fn new(buffer: &'buffer mut Buffer) -> Self {
        // Clear the buffer before we start building.
        buffer.0.clear();
        StringBuilder { buffer }
    }

    #[inline]
    pub fn push_str(&mut self, src: &str) {
        self.buffer.0.push_str(src)
    }

    pub fn push_char(&mut self, c: char) {
        self.buffer.0.push(c)
    }

    pub fn extend<I: Iterator<Item = char>>(&mut self, iter: I) {
        self.buffer.0.extend(iter)
    }

    #[inline]
    pub fn transform_and_push(&mut self, input: &str, tf: TextTransform) {
        self.buffer.transform_and_append(input, tf)
    }

    pub fn finish(self, arena: &Arena) -> &str {
        arena.alloc_str(&self.buffer.0)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeListBuilder<'arena>(Option<InhabitedNodeList<'arena>>);

#[derive(Debug)]
struct InhabitedNodeList<'arena> {
    head: NonNull<NodeListElement<'arena>>,
    tail: NonNull<NodeListElement<'arena>>,
}

pub enum SingletonOrList<'arena> {
    List(NodeList<'arena>),
    Singleton(NodeRef<'arena>),
}

impl Default for NodeListBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'arena> NodeListBuilder<'arena> {
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
    pub fn push(&mut self, node_ref: NodeRef<'arena>) {
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
    pub fn as_singleton_or_finish(self) -> SingletonOrList<'arena> {
        match self.0 {
            Some(mut list) if list.head == list.tail => {
                SingletonOrList::Singleton(unsafe { list.head.as_mut() })
            }
            _ => SingletonOrList::List(self.finish()),
        }
    }

    /// Finish building the list and return it.
    /// This method consumes the builder.
    pub fn finish(self) -> NodeList<'arena> {
        NodeList(self.0.map(|mut list| unsafe { list.head.as_mut() }))
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeList<'arena>(Option<NodeRef<'arena>>);

impl<'arena> NodeList<'arena> {
    #[inline]
    pub fn empty() -> Self {
        NodeList(None)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn from_two_nodes(first: NodeRef<'arena>, second: NodeRef<'arena>) -> Self {
        first.next = Some(NonNull::from(second));
        NodeList(Some(first))
    }

    pub fn iter(&'arena self) -> NodeListIterator<'arena> {
        NodeListIterator {
            current: self.0.as_deref(),
        }
    }
}

impl<'arena> IntoIterator for NodeList<'arena> {
    type Item = NodeRef<'arena>;
    type IntoIter = NodeListIntoIter<'arena>;

    /// Iterate over the list manually.
    ///
    /// This iterator cannot be used with a for loop, because the .next() method
    /// requires a reference to the arena. This is useful when you want to use
    /// a mutable reference to the arena within the loop body.
    fn into_iter(self) -> NodeListIntoIter<'arena> {
        NodeListIntoIter { current: self.0 }
    }
}

pub struct NodeListIterator<'arena> {
    current: Option<&'arena NodeListElement<'arena>>,
}

impl<'arena> Iterator for NodeListIterator<'arena> {
    type Item = &'arena Node<'arena>;

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

pub struct NodeListIntoIter<'arena> {
    current: Option<NodeRef<'arena>>,
}

impl<'arena> Iterator for NodeListIntoIter<'arena> {
    type Item = NodeRef<'arena>;
    fn next(&mut self) -> Option<NodeRef<'arena>> {
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
        let arena = Arena::new();
        let mut buffer = Buffer::new(0);
        let mut builder = buffer.get_builder();
        builder.extend("Hello, world!".chars());
        let str_ref = builder.finish(&arena);
        assert_eq!(str_ref, "Hello, world!");
    }

    #[test]
    fn buffer_manual_reference() {
        let arena = Arena::new();
        let mut buffer = Buffer::new(0);
        let mut builder = buffer.get_builder();
        assert_eq!(builder.buffer.0.len(), 0);
        builder.push_char('H');
        builder.push_char('i');
        builder.push_char('↩'); // This is a multi-byte character.
        assert_eq!(builder.buffer.0.len(), 5);
        let str_ref = builder.finish(&arena);
        assert_eq!(str_ref.len(), 5);
        assert_eq!(str_ref, "Hi↩");
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
