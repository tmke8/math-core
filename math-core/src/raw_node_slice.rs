use crate::mathml_renderer::arena::FrozenArena;

use super::Node;

#[derive(Debug)]
pub struct RawNodeSlice {
    ptr: *const &'static Node<'static>,
    len: usize,
}

impl RawNodeSlice {
    pub fn from_slice(slice: &[&Node<'_>]) -> Self {
        Self {
            ptr: unsafe {
                std::mem::transmute::<*const &Node<'_>, *const &'static Node<'static>>(
                    slice.as_ptr(),
                )
            },
            len: slice.len(),
        }
    }

    /// Turn the raw pointer within `RawNodeSlice` into a slice of references to nodes.
    /// This method requires a reference to a `FrozenArena` to ensure the nodes are valid.
    /// We check at runtime whether the slice is contained within the arena, ensuring safety.
    pub fn lift<'arena>(
        &self,
        arena: &'arena FrozenArena,
    ) -> Option<&'arena [&'arena Node<'arena>]> {
        let ptr = unsafe {
            std::mem::transmute::<*const &'static Node<'static>, *const &'arena Node<'arena>>(
                self.ptr,
            )
        };
        let slice = unsafe { std::slice::from_raw_parts(ptr, self.len) };
        arena.contains_slice(slice).then(|| {
            // SAFETY: Slice and references are guaranteed to be valid for the lifetime of the arena.
            unsafe {
                std::mem::transmute::<&[&'arena Node<'arena>], &'arena [&'arena Node<'arena>]>(
                    slice,
                )
            }
        })
    }
}

// Safety: While `RawNodeSlice` contains a raw pointer, it does not allow mutation of the underlying
// data. In order to dereference the pointer, one requires a valid reference to a `FrozenArena`,
// which contains the pointer.
unsafe impl Send for RawNodeSlice {}
unsafe impl Sync for RawNodeSlice {}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::mathml_renderer::arena::Arena;

    use super::*;

    #[test]
    fn raw_node_slice_test() {
        let arena = Arena::new();
        let node1 = arena.push(Node::HardcodedMathML("Node 1"));
        let node2 = arena.push(Node::HardcodedMathML("Node 2"));
        let slice = arena.push_slice(&[node1, node2]);

        let raw_slice = RawNodeSlice::from_slice(slice);
        let arena = arena.freeze();

        thread::spawn(move || {
            let lifted = raw_slice.lift(&arena).unwrap();
            assert_eq!(lifted.len(), 2);
            assert!(matches!(lifted[0], &Node::HardcodedMathML("Node 1")));
            assert!(matches!(lifted[1], &Node::HardcodedMathML("Node 2")));
        })
        .join()
        .unwrap();
    }
}
