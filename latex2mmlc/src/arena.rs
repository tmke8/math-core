use std::marker::PhantomData;
use std::ptr::NonNull;

use bumpalo::Bump;

use crate::ast::Node;

#[derive(Debug)]
pub struct NodeListElement<'arena, 'source> {
    pub node: Node<'arena, 'source>,
    next: Option<NonNull<NodeListElement<'arena, 'source>>>,
}

pub type NodeRef<'arena, 'source> = &'arena mut NodeListElement<'arena, 'source>;

pub struct Arena<'source> {
    bump: Bump,
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
}

impl Default for Arena<'_> {
    fn default() -> Self {
        Self::new()
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
    fn list_into_iter() {
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
