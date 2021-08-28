use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

use crate::list::cursor::{Cursor, CursorMut};
use crate::{IntoIter, Iter, IterMut};

pub mod cursor;
pub mod iterator;

mod algorithms;

/// The `List` is a doubly-linked list with owned nodes, implemented as a cyclic list.
/// It allows inserting, removing elements at any given position in constant time.
/// In compromise, accessing or mutating elements at any position take *O*(*n*) time.
pub struct List<T> {
    ghost: Box<Node<Erased>>,
    #[cfg(feature = "length")]
    /// the length of the list
    pub(crate) len: usize,
    _marker: PhantomData<Box<Node<T>>>,
}

#[repr(C)]
pub(crate) struct Node<T> {
    pub(crate) next: NonNull<Node<T>>,
    pub(crate) prev: NonNull<Node<T>>,
    pub(crate) element: T,
}

#[derive(Default)]
struct Erased;

pub(crate) struct DetachedNodes<T> {
    pub(crate) front: NonNull<Node<T>>,
    pub(crate) back: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    pub(crate) len: usize,
}

impl<T> List<T> {
    pub(crate) fn ghost_node(&self) -> NonNull<Node<T>> {
        NonNull::from(self.ghost.as_ref()).cast()
    }
    pub(crate) fn front_node(&self) -> NonNull<Node<T>> {
        // SAFETY: `ghost.next` is always valid (either `ghost` itself, or the first element
        // in the cyclic_list).
        NonNull::from(unsafe { self.ghost_node().as_ref().next.as_ref() }).cast()
    }
    pub(crate) fn back_node(&self) -> NonNull<Node<T>> {
        // SAFETY: `ghost.prev` is always valid (either `ghost` itself, or the last element
        // in the cyclic_list).
        NonNull::from(unsafe { self.ghost_node().as_ref().prev.as_ref() }).cast()
    }

    pub(crate) unsafe fn detach_node(&mut self, node: NonNull<Node<T>>) -> Box<Node<T>> {
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        let node = Box::from_raw(node.as_ptr());
        let (mut prev, mut next) = (node.prev, node.next);
        prev.as_mut().next = next;
        next.as_mut().prev = prev;
        node
    }
    pub(crate) unsafe fn detach_nodes(
        &mut self,
        front: NonNull<Node<T>>,
        back: NonNull<Node<T>>,
        #[cfg(feature = "length")] len: usize,
    ) -> DetachedNodes<T> {
        #[cfg(feature = "length")]
        {
            self.len -= len;
        }
        let (mut prev, mut next) = (front.as_ref().prev, back.as_ref().next);
        prev.as_mut().next = next;
        next.as_mut().prev = prev;
        DetachedNodes::new(
            front,
            back,
            #[cfg(feature = "length")]
            len,
        )
    }
    pub(crate) fn detach_all_nodes(&mut self) -> Option<DetachedNodes<T>> {
        if self.is_empty() {
            return None;
        }
        unsafe {
            Some(self.detach_nodes(
                self.front_node(),
                self.back_node(),
                #[cfg(feature = "length")]
                self.len,
            ))
        }
    }

    pub(crate) unsafe fn attach_node(
        &mut self,
        mut prev: NonNull<Node<T>>,
        mut next: NonNull<Node<T>>,
        mut node: NonNull<Node<T>>,
    ) {
        prev.as_mut().next = node;
        next.as_mut().prev = node;
        node.as_mut().prev = prev;
        node.as_mut().next = next;
        #[cfg(feature = "length")]
        {
            self.len += 1;
        }
    }
    pub(crate) unsafe fn attach_nodes(
        &mut self,
        mut prev: NonNull<Node<T>>,
        mut next: NonNull<Node<T>>,
        mut detached: DetachedNodes<T>,
    ) {
        prev.as_mut().next = detached.front;
        next.as_mut().prev = detached.back;
        detached.front.as_mut().prev = prev;
        detached.back.as_mut().next = next;
        #[cfg(feature = "length")]
        {
            self.len += detached.len;
        }
    }
    pub(crate) fn from_detached(detached: DetachedNodes<T>) -> Self {
        let mut list = List::new();
        // TODO: SAFETY
        unsafe {
            list.attach_nodes(list.ghost_node(), list.ghost_node(), detached);
        }
        list
    }
    pub(crate) fn into_detached(mut self) -> Option<DetachedNodes<T>> {
        self.detach_all_nodes()
    }
}

impl<T> List<T> {
    /// Create an empty `List`
    ///
    /// # Examples
    /// ```
    /// use cyclic_list::List;
    /// let list: List<u32> = List::new();
    /// ```
    #[inline]
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

    /// Returns `true` if the `List` is empty.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert!(list.is_empty());
    ///
    /// list.push_front("foo");
    /// assert!(!list.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.front_node() == self.ghost_node()
    }

    /// Returns the length of the `List`. Enabled by `feature = "length"`.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// #![cfg(feature = "length")]
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_front(2);
    /// assert_eq!(list.len(), 1);
    ///
    /// list.push_front(1);
    /// assert_eq!(list.len(), 2);
    ///
    /// list.push_back(3);
    /// assert_eq!(list.len(), 3);
    /// ```
    #[cfg(feature = "length")]
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Removes all elements from the `List`.
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_front(2);
    /// list.push_front(1);
    /// #[cfg(feature = "length")]
    /// assert_eq!(list.len(), 2);
    /// assert_eq!(list.front(), Some(&1));
    ///
    /// list.clear();
    /// #[cfg(feature = "length")]
    /// assert_eq!(list.len(), 0);
    /// assert_eq!(list.front(), None);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        while let Some(_) = self.pop_front() {}
    }

    /// Provides a reference to the front element, or `None` if the list is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert_eq!(list.front(), None);
    ///
    /// list.push_front(1);
    /// assert_eq!(list.front(), Some(&1));
    /// ```
    #[inline]
    pub fn front(&self) -> Option<&T> {
        self.cursor_start().current()
    }

    /// Provides a mutable reference to the front element, or `None` if the list
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert_eq!(list.front(), None);
    ///
    /// list.push_front(1);
    /// assert_eq!(list.front(), Some(&1));
    ///
    /// if let Some(x) = list.front_mut() {
    ///     *x = 5;
    /// }
    /// assert_eq!(list.front(), Some(&5));
    /// ```
    #[inline]
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.cursor_start_mut().current_mut()
    }

    /// Provides a reference to the back element, or `None` if the list is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert_eq!(list.back(), None);
    ///
    /// list.push_back(1);
    /// assert_eq!(list.back(), Some(&1));
    /// ```
    #[inline]
    pub fn back(&self) -> Option<&T> {
        self.cursor_end().previous()
    }

    /// Provides a mutable reference to the back element, or `None` if the list
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert_eq!(list.back(), None);
    ///
    /// list.push_back(1);
    /// assert_eq!(list.back(), Some(&1));
    ///
    /// if let Some(x) = list.back_mut() {
    ///     *x = 5;
    /// }
    /// assert_eq!(list.back(), Some(&5));
    /// ```
    #[inline]
    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.cursor_end_mut().previous_mut()
    }

    /// Adds an element first in the list.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_front(2);
    /// assert_eq!(list.front().unwrap(), &2);
    ///
    /// list.push_front(1);
    /// assert_eq!(list.front().unwrap(), &1);
    /// ```
    pub fn push_front(&mut self, elt: T) {
        self.cursor_start_mut().insert(elt);
    }

    /// Removes the first element and returns it, or `None` if the list is
    /// empty.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert_eq!(list.pop_front(), None);
    ///
    /// list.push_front(1);
    /// list.push_front(3);
    /// assert_eq!(list.pop_front(), Some(3));
    /// assert_eq!(list.pop_front(), Some(1));
    /// assert_eq!(list.pop_front(), None);
    /// ```
    pub fn pop_front(&mut self) -> Option<T> {
        self.cursor_start_mut().remove()
    }

    /// Appends an element to the back of a list.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// list.push_back(1);
    /// list.push_back(3);
    /// assert_eq!(list.back().unwrap(), &3);
    /// ```
    pub fn push_back(&mut self, elt: T) {
        self.cursor_end_mut().insert(elt);
    }

    /// Removes the last element from a list and returns it, or `None` if
    /// it is empty.
    ///
    /// This operation should compute in *O*(1) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    /// assert_eq!(list.pop_back(), None);
    /// list.push_back(1);
    /// list.push_back(3);
    /// assert_eq!(list.pop_back(), Some(3));
    /// ```
    pub fn pop_back(&mut self) -> Option<T> {
        self.cursor_end_mut().backspace()
    }

    /// Provides a cursor at the node with given index.
    ///
    /// By convention, the cursor is pointing to the "ghost" node if `at == len`.
    ///
    /// # Panics
    ///
    /// Panics if `at > len`
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let list = List::from_iter([1, 2, 3]);
    /// assert_eq!(list.cursor(1).current(), Some(&2));
    /// assert_eq!(list.cursor(3).current(), None);
    /// ```
    pub fn cursor(&self, at: usize) -> Cursor<'_, T> {
        let mut cursor = self.cursor_start();
        cursor
            .seek_to(at)
            .expect("Cannot create cursor at unexpected place");
        cursor
    }

    /// Provides a cursor at the first node.
    ///
    /// The cursor is pointing to the "ghost" node if the list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let list = List::from_iter([1, 2, 3]);
    /// let cursor = list.cursor_start();
    /// assert_eq!(cursor.current(), Some(&1));
    /// ```
    pub fn cursor_start(&self) -> Cursor<'_, T> {
        Cursor::new(
            self,
            self.front_node(),
            #[cfg(feature = "length")]
            0,
        )
    }

    /// Provides a cursor at the ghost node.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let list = List::from_iter([1, 2, 3]);
    /// let cursor = list.cursor_end();
    /// assert_eq!(cursor.current(), None);
    /// assert_eq!(cursor.previous(), Some(&3));
    /// ```
    pub fn cursor_end(&self) -> Cursor<'_, T> {
        Cursor::new(
            self,
            self.ghost_node(),
            #[cfg(feature = "length")]
            self.len,
        )
    }

    /// Provides a cursor with editing operations at the node with given index.
    ///
    /// By convention, the cursor is pointing to the "ghost" node if `at == len`.
    ///
    /// # Panics
    ///
    /// Panics if `at > len`
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    /// let mut cursor = list.cursor_mut(1);
    ///
    /// if let Some(x) = cursor.current_mut() {
    ///     *x *= 5;
    /// }
    /// assert_eq!(cursor.current(), Some(&10));
    /// assert_eq!(list.cursor_mut(3).current_mut(), None);
    /// ```
    pub fn cursor_mut(&mut self, at: usize) -> CursorMut<'_, T> {
        let mut cursor = self.cursor_start_mut();
        cursor
            .seek_to(at)
            .expect("Cannot create cursor at unexpected place");
        cursor
    }

    /// Provides a cursor with editing operations at the first node.
    ///
    /// The cursor is pointing to the "ghost" node if the list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    /// let mut cursor = list.cursor_start_mut();
    ///
    /// if let Some(x) = cursor.current_mut() {
    ///     *x *= 5;
    /// }
    /// assert_eq!(cursor.current(), Some(&5));
    /// ```
    pub fn cursor_start_mut(&mut self) -> CursorMut<'_, T> {
        CursorMut::new(
            self,
            self.front_node(),
            #[cfg(feature = "length")]
            0,
        )
    }

    /// Provides a cursor with editing operations at the ghost node.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    /// let mut cursor = list.cursor_end_mut();
    ///
    /// if let Some(x) = cursor.previous_mut() {
    ///     *x *= 5;
    /// }
    /// assert_eq!(cursor.previous(), Some(&15));
    /// ```
    pub fn cursor_end_mut(&mut self) -> CursorMut<'_, T> {
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

    pub fn append(&mut self, other: &mut Self) {
        if let Some(detached) = other.detach_all_nodes() {
            // TODO: SAFETY
            unsafe { self.attach_nodes(self.back_node(), self.ghost_node(), detached) }
        }
    }

    pub fn prepend(&mut self, other: &mut Self) {
        if let Some(detached) = other.detach_all_nodes() {
            // TODO: SAFETY
            unsafe { self.attach_nodes(self.ghost_node(), self.front_node(), detached) }
        }
    }

    /// Splits the list into two at the given index. Returns everything after the given index,
    /// including the index; or `None` if `at == len`.
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Panics
    /// Panics if `at > len`
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_front(1);
    /// list.push_front(2);
    /// list.push_front(3);
    ///
    /// let mut split = list.split_off(2).unwrap();
    ///
    /// assert_eq!(split.pop_front(), Some(1));
    /// assert_eq!(split.pop_front(), None);
    /// ```
    pub fn split_off(&mut self, at: usize) -> Option<List<T>> {
        #[cfg(feature = "length")]
        assert!(at <= self.len, "Cannot split off at a nonexistent index");
        #[cfg(feature = "length")]
        if at == self.len {
            return None;
        }
        self.cursor_mut(at).split()
    }

    /// Removes the element at the given index and returns it.
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Panics
    /// Panics if `at >= len`
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_front(1);
    /// list.push_front(2);
    /// list.push_front(3);
    ///
    /// assert_eq!(list.remove(1), 2);
    /// assert_eq!(list.remove(0), 3);
    /// assert_eq!(list.remove(0), 1);
    /// ```
    pub fn remove(&mut self, at: usize) -> T {
        #[cfg(feature = "length")]
        assert!(
            at < self.len,
            "Cannot remove at an index outside of the list bounds"
        );

        self.cursor_mut(at)
            .remove()
            .expect("Cannot remove at an index outside of the list bounds")
    }

    /// Splices another list at the given index.
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Panics
    /// Panics if `at > len`
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    ///
    /// let other = List::from_iter([4, 5, 6]);
    ///
    /// list.splice_at(2, other);
    ///
    /// assert_eq!(Vec::from_iter(list), vec![1, 2, 4, 5, 6, 3]);
    /// ```
    pub fn splice_at(&mut self, at: usize, other: Self) {
        #[cfg(feature = "length")]
        assert!(at <= self.len, "Cannot split at a nonexistent node");
        let mut cursor_mut = self.cursor_start_mut();
        cursor_mut
            .seek_forward(at)
            .expect("Cannot splice at a nonexistent node");
        cursor_mut.splice(other);
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
            #[allow(invalid_value)]
            let node = Box::<Node<T>>::leak(Box::new(MaybeUninit::uninit().assume_init()));
            std::ptr::write(&mut node.element, element);
            node
        })
    }

    pub(crate) fn into_element(self: Box<Self>) -> T {
        self.element
    }
}

impl<T> DetachedNodes<T> {
    fn new(
        front: NonNull<Node<T>>,
        back: NonNull<Node<T>>,
        #[cfg(feature = "length")] len: usize,
    ) -> Self {
        #[cfg(feature = "length")]
        debug_assert!(len > 0, "Cannot detach nodes of length 0");
        Self {
            front,
            back,
            #[cfg(feature = "length")]
            len,
        }
    }
}

fn new_ghost() -> Box<Node<Erased>> {
    let ghost_ptr = Node::new_detached(Erased::default());
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

unsafe impl<T: Send> Send for List<T> {}

unsafe impl<T: Sync> Sync for List<T> {}

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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::list::List;

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
