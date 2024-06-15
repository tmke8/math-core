use crate::ast::Node;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct NodeReference(usize);

impl NodeReference {
    /// Convert a reference to a node by looking it up in the arena.
    #[inline]
    pub fn as_node<'arena, 'source>(&self, arena: &'arena Arena<'source>) -> &'arena Node<'source> {
        arena.lookup(*self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StrReference(usize, usize);

impl StrReference {
    pub fn new(start: usize, end: usize) -> Self {
        StrReference(start, end)
    }

    #[inline]
    pub fn as_str<'buffer>(&self, buffer: &'buffer Buffer) -> &'buffer str {
        buffer.get_str(*self)
    }
}

#[derive(Debug)]
pub struct NodeListElement<'source> {
    pub node: Node<'source>,
    pub next: Option<NodeReference>,
}

#[derive(Debug)]
pub struct Arena<'source> {
    nodes: Vec<NodeListElement<'source>>,
}

impl<'source> Arena<'source> {
    pub fn new() -> Self {
        Arena { nodes: Vec::new() }
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
        NodeReference(index)
    }

    fn lookup(&self, reference: NodeReference) -> &Node<'source> {
        &self.get_raw(reference).node
    }

    fn get_raw<'arena>(&'arena self, reference: NodeReference) -> &'arena NodeListElement<'source> {
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        unsafe { self.nodes.get(reference.0).unwrap_unchecked() }
    }

    fn get_raw_mut<'arena>(
        &'arena mut self,
        reference: NodeReference,
    ) -> &'arena mut NodeListElement<'source> {
        // safety: we only give out valid NodeReferences and don't expose delete functionality
        unsafe { self.nodes.get_unchecked_mut(reference.0) }
    }

    pub fn lookup_mut<'arena>(
        &'arena mut self,
        reference: NodeReference,
    ) -> &'arena mut Node<'source> {
        &mut self.get_raw_mut(reference).node
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Buffer(String);

impl Buffer {
    pub fn new() -> Self {
        Buffer(String::new())
    }

    pub fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) -> StrReference {
        let start = self.0.len();
        self.0.extend(iter);
        let end = self.0.len();
        StrReference(start, end)
    }

    pub fn push_str(&mut self, string: &str) -> StrReference {
        let start = self.0.len();
        self.0.push_str(string);
        let end = self.0.len();
        StrReference(start, end)
    }

    pub fn push(&mut self, ch: char) {
        self.0.push(ch);
    }

    fn get_str(&self, reference: StrReference) -> &str {
        // &self.0[Range::<usize>::from(reference)]
        unsafe { self.0.get_unchecked(reference.0..reference.1) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
struct InhabitedNodeList {
    head: NodeReference,
    tail: NodeReference,
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct NodeList(Option<InhabitedNodeList>);

impl NodeList {
    pub fn new() -> Self {
        NodeList(None)
    }

    pub fn push<'source>(&mut self, arena: &mut Arena<'source>, node: Node<'source>) {
        // Add node to the arena and get a reference to it.
        let new_tail = arena.push(node);
        self.push_ref(arena, new_tail)
    }

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

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

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

    pub fn iter<'arena, 'source>(
        &self,
        arena: &'arena Arena<'source>,
    ) -> NodeListIterator<'arena, 'source> {
        NodeListIterator {
            arena,
            current: self.get_head(),
        }
    }

    /// Iterate over the list manually.
    ///
    /// This iterator cannot be used with a for loop, because the .next() method
    /// requires a reference to the arena. This is useful when you want to use
    /// a mutable reference to the arena within the loop body.
    pub fn iter_manually(&self) -> NodeListManualIterator {
        NodeListManualIterator {
            current: self.get_head(),
        }
    }

    fn get_head(&self) -> Option<NodeReference> {
        self.0.as_ref().map(|list| list.head)
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
        assert!(matches!(
            arena.lookup(reference),
            Node::Space("Hello, world!")
        ));
    }

    #[test]
    fn list() {
        let mut arena = Arena::new();
        let mut list = NodeList::new();
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
        let mut list = NodeList::new();
        list.push(&mut arena, Node::Space("Hello, world!"));
        assert!(list.is_singleton().is_some());
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
        assert!(iter.next().is_none(), "Empty list should return None");
    }

    #[test]
    fn buffer_extend() {
        let mut arena = Buffer::new();
        let str_ref = arena.extend("Hello, world!".chars());
        assert_eq!(arena.get_str(str_ref), "Hello, world!");
    }

    #[test]
    fn buffer_push_str() {
        let mut arena = Buffer::new();
        let str_ref = arena.push_str("Hello, world!");
        assert_eq!(arena.get_str(str_ref), "Hello, world!");
    }
}
