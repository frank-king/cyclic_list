use crate::cursor::{Cursor, CursorMut};
use crate::iterator::{IntoIter, Iter, IterMut};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

pub struct List<T> {
    ghost: Box<Node<Erased>>,
    #[cfg(feature = "length")]
    pub(crate) len: usize,
    _marker: PhantomData<Box<Node<T>>>,
}

#[repr(C)]
pub(crate) struct Node<T> {
    pub(crate) next: NonNull<Node<T>>,
    pub(crate) prev: NonNull<Node<T>>,
    pub(crate) element: T,
}

struct Erased;

pub(crate) struct DetachedNodes<T> {
    pub(crate) start: NonNull<Node<T>>,
    pub(crate) end: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    pub(crate) len: usize,
}

impl<T> List<T> {
    pub(crate) fn ghost_node(&self) -> NonNull<Node<T>> {
        NonNull::from(self.ghost.as_ref()).cast()
    }
    pub(crate) fn ghost_node_next(&self) -> NonNull<Node<T>> {
        // SAFETY: `ghost.next` is always valid (either `ghost` itself, or the first element
        // in the cyclic_list).
        NonNull::from(unsafe { self.ghost_node().as_ref().next.as_ref() }).cast()
    }
    pub(crate) fn ghost_node_prev(&self) -> NonNull<Node<T>> {
        // SAFETY: `ghost.prev` is always valid (either `ghost` itself, or the last element
        // in the cyclic_list).
        NonNull::from(unsafe { self.ghost_node().as_ref().prev.as_ref() }).cast()
    }
    pub(crate) unsafe fn detach_nodes(
        &mut self,
        mut start: NonNull<Node<T>>,
        mut end: NonNull<Node<T>>,
        #[cfg(feature = "length")] len: usize,
    ) -> DetachedNodes<T> {
        let mut existing_prev = start.as_ref().prev;
        let mut existing_next = end.as_ref().next;
        existing_prev.as_mut().next = existing_next;
        existing_next.as_mut().prev = existing_prev;
        DetachedNodes {
            start,
            end,
            #[cfg(feature = "length")]
            len,
        }
    }
    pub(crate) unsafe fn detach_node(&mut self, mut node: NonNull<Node<T>>) -> Box<Node<T>> {
        let node = Box::from_raw(node.as_ptr());
        let (mut prev, mut next) = (node.prev, node.next);
        prev.as_mut().next = next;
        next.as_mut().prev = prev;
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        node
    }
    pub(crate) unsafe fn attach_nodes(
        &mut self,
        mut existing_prev: NonNull<Node<T>>,
        mut existing_next: NonNull<Node<T>>,
        mut detached: DetachedNodes<T>,
    ) {
        existing_prev.as_mut().next = detached.start;
        existing_next.as_mut().prev = detached.end;
        detached.start.as_mut().prev = existing_prev;
        detached.end.as_mut().next = existing_next;
        #[cfg(feature = "length")]
        {
            self.len += detached.len;
        }
    }
    pub(crate) unsafe fn from_detached(detached: DetachedNodes<T>) -> Self {
        let mut list = List::new();
        list.attach_nodes(list.ghost_node(), list.ghost_node(), detached);
        list
    }
    pub(crate) fn into_detached(mut self) -> Option<DetachedNodes<T>> {
        if self.is_empty() {
            return None;
        }
        let start = self.ghost_node_next();
        let end = self.ghost_node_prev();
        unsafe {
            Some(self.detach_nodes(
                start,
                end,
                #[cfg(feature = "length")]
                self.len,
            ))
        }
    }
}

// Ensure that `List` and its read-only iterators are covariant in their type parameters.
#[allow(dead_code)]
fn assert_covariance() {
    fn a<'a>(x: List<&'static str>) -> List<&'a str> {
        x
    }
    fn b<'i, 'a>(x: Iter<'i, &'static str>) -> Iter<'i, &'a str> {
        x
    }
    fn c<'a>(x: IntoIter<&'static str>) -> IntoIter<&'a str> {
        x
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        let ghost = new_ghost();
        #[cfg(feature = "length")]
        let len = 0;
        let _marker = PhantomData;
        Self {
            ghost,
            #[cfg(feature = "length")]
            len,
            _marker,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ghost_node_next() == self.ghost_node()
    }

    #[cfg(feature = "length")]
    pub fn len(&self) -> usize {
        self.len
    }

    #[cfg(not(feature = "length"))]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn clear(&mut self) {
        while let Some(_) = self.pop_front() {}
    }

    pub fn push_front(&mut self, elt: T) {
        self.cursor_front_mut().insert(elt);
    }

    pub fn push_back(&mut self, elt: T) {
        self.cursor_back_mut().insert(elt);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.cursor_front_mut().remove()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.cursor_back_mut().backspace()
    }

    pub fn cursor_front(&self) -> Cursor<'_, T> {
        Cursor::new(
            self,
            self.ghost_node_next(),
            #[cfg(feature = "length")]
            0,
        )
    }

    pub fn cursor_back(&self) -> Cursor<'_, T> {
        Cursor::new(
            self,
            self.ghost_node(),
            #[cfg(feature = "length")]
            self.len,
        )
    }

    pub fn cursor_front_mut(&mut self) -> CursorMut<'_, T> {
        CursorMut::new(
            self,
            self.ghost_node_next(),
            #[cfg(feature = "length")]
            0,
        )
    }

    pub fn cursor_back_mut(&mut self) -> CursorMut<'_, T> {
        CursorMut::new(
            self,
            self.ghost_node(),
            #[cfg(feature = "length")]
            self.len,
        )
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }

    pub fn split_off(&mut self, at: usize) -> Option<List<T>> {
        let mut cursor_mut = self.cursor_front_mut();
        cursor_mut
            .seek_forward(at)
            .expect("Cannot split at a nonexistent node");
        cursor_mut.split()
    }

    pub fn splice_at(&mut self, at: usize, other: List<T>) {
        let mut cursor_mut = self.cursor_front_mut();
        cursor_mut
            .seek_forward(at)
            .expect("Cannot split at a nonexistent node");
    }
}

impl<T: Debug> Debug for List<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Node<T> {
    pub(crate) fn new_detached(element: T) -> NonNull<Node<T>> {
        // SAFETY:
        // - `node.element` is manually written, so it is safe;
        // - `node.prev` and `node.next` is dangling, but need unsafe blocks for dereference,
        //   so it is also safe.
        NonNull::from(unsafe {
            let mut node = Box::<Node<T>>::leak(Box::new(MaybeUninit::uninit().assume_init()));
            std::ptr::write(&mut node.element, element);
            node
        })
    }

    pub(crate) fn into_element(self: Box<Self>) -> T {
        self.element
    }
}

impl<T> DetachedNodes<T> {
    pub(crate) fn from_single(node: NonNull<Node<T>>) -> Self {
        Self {
            start: node,
            end: node,
            #[cfg(feature = "length")]
            len: 1,
        }
    }
}

fn new_ghost() -> Box<Node<Erased>> {
    let ghost_ptr = Node::new_detached(Erased);
    // SAFETY:
    // - `ghost.next`, `ghost.prev` is initialized immediately after creating `ghost`.
    // - `ghost.element` is never read, so it is erased out.
    let mut ghost = unsafe { Box::from_raw(ghost_ptr.as_ptr()) };
    ghost.next = ghost_ptr;
    ghost.prev = ghost_ptr;
    ghost
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::list::List;
    use std::cell::RefCell;

    #[test]
    fn list_create() {
        let mut list = List::<i32>::new();
        assert!(list.is_empty());
        list.push_back(1);
        assert!(!list.is_empty());
        assert_eq!(list.pop_back(), Some(1));
        assert!(list.is_empty());
    }

    #[test]
    fn list_drop() {
        #[derive(Debug)]
        struct DropChecker<'a, T: Copy> {
            value: T,
            dropped: &'a RefCell<Vec<T>>,
        }
        impl<'a, T: Copy> DropChecker<'a, T> {
            fn new(value: T, dropped: &'a RefCell<Vec<T>>) -> Self {
                Self { value, dropped }
            }
        }
        impl<'a, T: Copy> Drop for DropChecker<'a, T> {
            fn drop(&mut self) {
                self.dropped.borrow_mut().push(self.value);
            }
        }
        let dropped = RefCell::new(Vec::<i32>::new());
        let mut list = List::<DropChecker<i32>>::new();
        list.push_back(DropChecker::new(1, &dropped));
        list.push_back(DropChecker::new(2, &dropped));
        list.push_back(DropChecker::new(3, &dropped));
        drop(list);
        assert_eq!(dropped.borrow().as_slice(), &[1, 2, 3]);
    }
}
