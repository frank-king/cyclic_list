use crate::cursor::{Cursor, CursorMut};
use crate::iterator::{Iter, IterMut};
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

impl<T> List<T> {
    fn init_ghost() -> Box<Node<Erased>> {
        let ghost_ptr: NonNull<MaybeUninit<Node<Erased>>> =
            NonNull::from(Box::leak(Box::new(MaybeUninit::uninit())));
        let ghost_ptr: NonNull<Node<Erased>> = ghost_ptr.cast();
        // SAFETY:
        // - `ghost.next`, `ghost.prev` is initialized immediately after creating `ghost`.
        // - `ghost.element` is never read, so it is erased out.
        let mut ghost = unsafe { Box::from_raw(ghost_ptr.as_ptr()) };
        ghost.next = ghost_ptr;
        ghost.prev = ghost_ptr;
        ghost
    }
    pub(crate) fn ghost(&self) -> NonNull<Node<T>> {
        NonNull::from(self.ghost.as_ref()).cast()
    }
    pub(crate) fn ghost_next(&self) -> NonNull<Node<T>> {
        // SAFETY: `ghost.next` is always valid (either `ghost` itself, or the first element
        // in the cyclic_list).
        NonNull::from(unsafe { self.ghost().as_ref().next.as_ref() }).cast()
    }
    pub(crate) fn ghost_prev(&self) -> NonNull<Node<T>> {
        // SAFETY: `ghost.prev` is always valid (either `ghost` itself, or the last element
        // in the cyclic_list).
        NonNull::from(unsafe { self.ghost().as_ref().prev.as_ref() }).cast()
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        let ghost = Self::init_ghost();
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
        self.ghost_next() == self.ghost()
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
        self.cursor_front_mut().insert_before(elt)
    }

    pub fn push_back(&mut self, elt: T) {
        self.cursor_back_mut().insert_after(elt)
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.cursor_front_mut().remove_current()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.cursor_back_mut().remove_current()
    }

    pub fn cursor_front(&self) -> Cursor<'_, T> {
        Cursor::new(
            #[cfg(feature = "length")]
            0,
            self,
            self.ghost_next(),
        )
    }

    pub fn cursor_back(&self) -> Cursor<'_, T> {
        Cursor::new(
            #[cfg(feature = "length")]
            self.len.checked_sub(1).unwrap_or(0),
            self,
            self.ghost_prev(),
        )
    }

    pub fn cursor_front_mut(&mut self) -> CursorMut<'_, T> {
        CursorMut::new(
            #[cfg(feature = "length")]
            0,
            self,
            self.ghost_next(),
        )
    }

    pub fn cursor_back_mut(&mut self) -> CursorMut<'_, T> {
        CursorMut::new(
            #[cfg(feature = "length")]
            self.len.checked_sub(1).unwrap_or(0),
            self,
            self.ghost_prev(),
        )
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self.cursor_front())
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self.cursor_front_mut())
    }
}

impl<T: Debug> Debug for List<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Node<T> {
    pub(crate) fn new(next: NonNull<Node<T>>, prev: NonNull<Node<T>>, element: T) -> Box<Self> {
        Box::new(Node {
            next,
            prev,
            element,
        })
    }

    pub(crate) fn into_element(self: Box<Self>) -> T {
        self.element
    }
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
        println!("start ");
        let mut list = List::<DropChecker<i32>>::new();
        eprintln!("cyclic_list: {:?}", list);
        list.push_back(DropChecker::new(1, &dropped));
        list.push_back(DropChecker::new(2, &dropped));
        list.push_back(DropChecker::new(3, &dropped));
        drop(list);
        assert_eq!(dropped.borrow().as_slice(), &[1, 2, 3]);
    }
}
