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
///
/// The `List` contains:
/// - a pointer `ghost` that points to the ghost node;
/// - a length field `len` indicating the length of the list. It can be disabled by
///   disabling the `length` feature in your `Cargo.toml`:
/// ```text
/// [dependencies]
/// cyclic_list = { default-features = false }
/// ```
///
/// # Naming Conventions
///
/// - `front..=back`: a closed range of list nodes, both inclusive;
/// - `start..end`: a half-open range of list nodes, left inclusive and right
///   exclusive (probably the ghost node).
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

/// Nodes fragment detached from a list, used in list splitting or
/// splicing.
///
/// When detached from a list, reading of `front.prev` and `back.next`
/// is invalid.
pub(crate) struct DetachedNodes<T> {
    pub(crate) front: NonNull<Node<T>>,
    pub(crate) back: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    pub(crate) len: usize,
    _marker: PhantomData<Box<Node<T>>>,
}

// private methods
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

    pub(crate) unsafe fn connect(
        &mut self,
        mut prev: NonNull<Node<T>>,
        mut next: NonNull<Node<T>>,
    ) {
        prev.as_mut().next = next;
        next.as_mut().prev = prev;
    }

    /// Detach a single node `node` from the list, and return it as a box.
    ///
    /// It is unsafe because it does not check whether `node` belongs to the list.
    ///
    /// If the `node` does not belong to the list, this function call will make
    /// the list ill-formed.
    pub(crate) unsafe fn detach_node(&mut self, node: NonNull<Node<T>>) -> Box<Node<T>> {
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        let node = Box::from_raw(node.as_ptr());
        self.connect(node.prev, node.next);
        node
    }

    /// Attach a single node `node` to the list, between `prev` and `next`.
    ///
    /// It is unsafe because it does not check whether `prev` and `next` belongs
    /// to the list, or whether the `prev` and `next` is adjacent (only in
    /// `#[cfg(debug_assertions)]`).
    ///
    /// If the `prev` and `next` does not belong to the list, or they are not
    /// adjacent nodes, this function call will make the list ill-formed.
    pub(crate) unsafe fn attach_node(
        &mut self,
        prev: NonNull<Node<T>>,
        next: NonNull<Node<T>>,
        node: NonNull<Node<T>>,
    ) {
        #[cfg(debug_assertions)]
        assert_adjacent(prev, next);
        self.connect(prev, node);
        self.connect(node, next);
        #[cfg(feature = "length")]
        {
            self.len += 1;
        }
        #[cfg(debug_assertions)]
        {
            assert_adjacent(prev, node);
            assert_adjacent(node, next);
        }
    }

    /// Detach a range of nodes `front..=back` from the list, and return the detached
    /// nodes.
    ///
    /// It is unsafe because it does not check whether `front..=back` is a valid range
    /// (i.e. `front` must **NOT** be at the right of `back`), or whether it belongs
    /// to the list.
    ///
    /// If `front..=back` is not a valid range or it does not belong to the list,
    /// this function call will make the list ill-formed.
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
        self.connect(front.as_ref().prev, back.as_ref().next);
        DetachedNodes::new(
            front,
            back,
            #[cfg(feature = "length")]
            len,
        )
    }

    /// Attach a range of detached nodes to the list, between `prev` and `next`.
    ///
    /// It is unsafe because it does not check whether `prev` and `next` belongs
    /// to the list, or whether the `prev` and `next` is adjacent (only in
    /// `#[cfg(debug_assertions)]`).
    ///
    /// If the `prev` and `next` does not belong to the list, or they are not
    /// adjacent nodes, this function call will make the list ill-formed.
    pub(crate) unsafe fn attach_nodes(
        &mut self,
        prev: NonNull<Node<T>>,
        next: NonNull<Node<T>>,
        detached: DetachedNodes<T>,
    ) {
        #[cfg(debug_assertions)]
        assert_adjacent(prev, next);
        self.connect(prev, detached.front);
        self.connect(detached.back, next);
        #[cfg(feature = "length")]
        {
            self.len += detached.len;
        }
        #[cfg(debug_assertions)]
        {
            assert_adjacent(prev, detached.front);
            assert_adjacent(detached.back, next);
        }
    }

    /// Detach all nodes from the list, and return the detached nodes, or return
    /// `None` if the list is empty.
    ///
    /// It is safe because `self.front_node()..=self.back_node()` is a valid range.
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

    /// Construct a list from detached nodes.
    ///
    /// It is safe because the detached nodes is guaranteed to be a valid range
    /// when construction.
    pub(crate) fn from_detached(detached: DetachedNodes<T>) -> Self {
        let mut list = List::new();
        unsafe {
            list.attach_nodes(list.ghost_node(), list.ghost_node(), detached);
        }
        list
    }

    /// Like [`List::detach_all_nodes`], but consume the list.
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
    /// # Complexity
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
    /// # Complexity
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
    /// # Complexity
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
        while self.pop_front().is_some() {}
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
    /// # Complexity
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
    /// # Complexity
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
        if self.is_empty() {
            return None;
        }
        self.cursor_start_mut().remove()
    }

    /// Appends an element to the back of a list.
    ///
    /// # Complexity
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
    /// # Complexity
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
        if self.is_empty() {
            return None;
        }
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
        #[cfg(feature = "length")]
        assert!(
            at <= self.len,
            "Cannot create cursor at a nonexistent index"
        );
        let mut cursor = self.cursor_start();
        cursor
            .try_seek_to(at)
            .expect("Cannot create cursor at a nonexistent index");
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
        #[cfg(feature = "length")]
        assert!(
            at <= self.len,
            "Cannot create cursor at a nonexistent index"
        );

        let mut cursor = self.cursor_start_mut();
        cursor
            .try_seek_to(at)
            .expect("Cannot create cursor at a nonexistent index");
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

    /// Provides a forward iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_back(0);
    /// list.push_back(1);
    /// list.push_back(2);
    ///
    /// let mut iter = list.iter();
    /// assert_eq!(iter.next(), Some(&0));
    /// assert_eq!(iter.next(), Some(&1));
    /// assert_eq!(iter.next(), Some(&2));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }

    /// Provides a forward iterator with mutable references.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_back(0);
    /// list.push_back(1);
    /// list.push_back(2);
    ///
    /// for element in list.iter_mut() {
    ///     *element += 10;
    /// }
    ///
    /// let mut iter = list.iter();
    /// assert_eq!(iter.next(), Some(&10));
    /// assert_eq!(iter.next(), Some(&11));
    /// assert_eq!(iter.next(), Some(&12));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }

    /// Moves all elements from `other` to the end of the list.
    ///
    /// This reuses all the nodes from `other` and moves them into `self`. After
    /// this operation, `other` becomes empty.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(1) time and *O*(1) memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list1 = List::new();
    /// list1.push_back('a');
    ///
    /// let mut list2 = List::new();
    /// list2.push_back('b');
    /// list2.push_back('c');
    ///
    /// list1.append(&mut list2);
    ///
    /// let mut iter = list1.iter();
    /// assert_eq!(iter.next(), Some(&'a'));
    /// assert_eq!(iter.next(), Some(&'b'));
    /// assert_eq!(iter.next(), Some(&'c'));
    /// assert!(iter.next().is_none());
    ///
    /// assert!(list2.is_empty());
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        if let Some(detached) = other.detach_all_nodes() {
            // `self.back_node()` and `self.ghost_node()` are valid
            // nodes in the list and they are adjacent, so it is safe.
            unsafe { self.attach_nodes(self.back_node(), self.ghost_node(), detached) }
        }
    }

    /// Moves all elements from `other` to the begin of the list.
    /// This reuses all the nodes from `other` and moves them into `self`. After
    /// this operation, `other` becomes empty.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(1) time and *O*(1) memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list1 = List::new();
    /// list1.push_back('a');
    ///
    /// let mut list2 = List::new();
    /// list2.push_back('b');
    /// list2.push_back('c');
    ///
    /// list2.prepend(&mut list1);
    ///
    /// let mut iter = list2.iter();
    /// assert_eq!(iter.next(), Some(&'a'));
    /// assert_eq!(iter.next(), Some(&'b'));
    /// assert_eq!(iter.next(), Some(&'c'));
    /// assert!(iter.next().is_none());
    ///
    /// assert!(list1.is_empty());
    /// ```
    pub fn prepend(&mut self, other: &mut Self) {
        if let Some(detached) = other.detach_all_nodes() {
            // `self.ghost_node()` and `self.front_node()` are valid
            // nodes in the list and they are adjacent, so it is safe.
            unsafe { self.attach_nodes(self.ghost_node(), self.front_node(), detached) }
        }
    }

    /// Splits the list into two at the given index. Returns everything after
    /// the given index (inclusive).
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Panics
    ///
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
    /// let mut split = list.split_off(2);
    ///
    /// assert_eq!(split.pop_front(), Some(1));
    /// assert_eq!(split.pop_front(), None);
    /// ```
    pub fn split_off(&mut self, at: usize) -> List<T> {
        #[cfg(feature = "length")]
        assert!(at <= self.len, "Cannot split off at a nonexistent index");
        #[cfg(feature = "length")]
        if at == self.len {
            return List::new();
        }
        self.cursor_mut(at).split().unwrap_or_default()
    }

    /// Removes the element at the given index and returns it.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Panics
    ///
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

    /// Adds an element at the given index in the list.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n*) time.
    ///
    /// # Panics
    ///
    /// Panics if `at >= len`
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    ///
    /// list.insert(2, 4);
    /// list.insert(4, 5);
    ///
    /// assert_eq!(Vec::from_iter(list), vec![1, 2, 4, 3, 5]);
    /// ```
    pub fn insert(&mut self, at: usize, elm: T) {
        #[cfg(feature = "length")]
        assert!(
            at <= self.len,
            "Cannot insert at an index outside of the list bounds"
        );

        self.cursor_mut(at).insert(elm);
    }

    /// Splices another list at the given index.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n*) time.
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

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Node<T> {
    /// Create a detached node with given element.
    pub(crate) fn new_detached(element: T) -> NonNull<Node<T>> {
        // SAFETY:
        // - `node.element` is manually written, so it is safe;
        // - `node.prev` and `node.next` is dangling, but need unsafe blocks for dereference,
        //   so it is also safe.
        NonNull::from(unsafe {
            // `node.prev` and `node.next` will not be read, so it is ok to be
            // uninitialized. `node.element` is initialized manually by `ptr::write`.
            #[allow(invalid_value, clippy::uninit_assumed_init)]
            let node = Box::<Node<T>>::leak(Box::new(MaybeUninit::uninit().assume_init()));
            std::ptr::write(&mut node.element, element);
            node
        })
    }
}

impl<T> DetachedNodes<T> {
    /// If is unsafe because it must be guaranteed that `front..=back` is
    /// a valid range and its length must be equal to `len` (with
    /// `#[cfg(feature = "length")]`).
    unsafe fn new(
        front: NonNull<Node<T>>,
        back: NonNull<Node<T>>,
        #[cfg(feature = "length")] len: usize,
    ) -> Self {
        let _marker = PhantomData;
        #[cfg(feature = "length")]
        debug_assert!(len > 0, "Cannot detach nodes of length 0");
        Self {
            front,
            back,
            #[cfg(feature = "length")]
            len,
            _marker,
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

#[cfg(debug_assertions)]
fn assert_adjacent<T>(prev: NonNull<Node<T>>, next: NonNull<Node<T>>) {
    unsafe {
        assert_eq!(prev.as_ref().next, next);
        assert_eq!(next.as_ref().prev, prev);
    }
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
    use crate::list::List;
    use std::cell::RefCell;
    use std::fmt::Debug;
    use std::iter::FromIterator;

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
        let mut list = List::new();
        list.push_back(DropChecker::new(1, &dropped));
        list.push_back(DropChecker::new(2, &dropped));
        list.push_back(DropChecker::new(3, &dropped));
        drop(list);
        assert_eq!(dropped.borrow().as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn list_push_and_pop() {
        let mut list = List::new();
        assert!(list.is_empty());
        #[cfg(feature = "length")]
        assert_eq!(list.len(), 0);

        assert_eq!(list.front(), None);
        assert_eq!(list.back(), None);
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.pop_back(), None);

        list.push_back(1);
        assert_eq!(list.back(), Some(&1));
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_back(), None);
        assert!(list.is_empty());
        #[cfg(feature = "length")]
        assert_eq!(list.len(), 0);

        list.push_front(1);
        list.push_front(2);
        list.push_back(3);
        assert_eq!(list.back(), Some(&3));
        assert_eq!(list.front(), Some(&2));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_back(), Some(3));

        assert_eq!(list.front(), Some(&1));
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.front(), None);
        assert_eq!(list.back(), None);
        assert!(list.is_empty());
        #[cfg(feature = "length")]
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn list_insert_and_remove() {
        fn list_eq<T, I>(list: &List<T>, expected: I)
        where
            T: Debug + Clone + Eq,
            I: IntoIterator<Item = T>,
        {
            assert_eq!(
                Vec::from_iter(list.iter().cloned()),
                Vec::from_iter(expected)
            );
        }

        let mut list = List::from_iter(0..10);
        list.insert(5, 10);
        list_eq(&list, (0..5).chain(Some(10)).chain(5..10));

        assert_eq!(list.remove(10), 9);
        assert_eq!(list.back(), Some(&8));
        list_eq(&list, (0..5).chain(Some(10)).chain(5..9));

        list.insert(0, 11);
        assert_eq!(list.front(), Some(&11));
        list_eq(&list, (11..=11).chain((0..5).chain(Some(10)).chain(5..9)));

        assert_eq!(list.remove(0), 11);
        assert_eq!(list.front(), Some(&0));
        list_eq(&list, (0..5).chain(Some(10)).chain(5..9));

        list.insert(10, 12);
        assert_eq!(list.back(), Some(&12));
        list_eq(&list, (0..5).chain(Some(10)).chain(5..9).chain(Some(12)));
    }

    #[test]
    fn list_split_and_append() {
        fn test_list_split_and_append_and_prepend<T, I1, I2, I3>(
            list: I1,
            other: I2,
            at: usize,
            appended: I3,
        ) where
            T: Clone + Eq + Debug,
            I1: IntoIterator<Item = T>,
            I2: IntoIterator<Item = T>,
            I3: IntoIterator<Item = T>,
        {
            // Construct the lists with iterators.
            let mut list = List::from_iter(list);
            let other = List::from_iter(other);
            let appended = List::from_iter(appended);

            let cloned = list.clone();
            let mut other_cloned = other.clone();

            // Test append
            list.append(&mut other_cloned);
            assert!(other_cloned.is_empty());
            assert_eq!(list, appended);
            #[cfg(feature = "length")]
            assert_eq!(list.len(), cloned.len() + other.len());

            // Test split
            let split = list.split_off(at);
            assert_eq!(list, cloned);
            assert_eq!(split, other.clone());
            #[cfg(feature = "length")]
            assert_eq!(list.len(), cloned.len());

            let (mut list, other) = (other, list);
            let cloned = list.clone();
            let mut other_cloned = other.clone();

            // Test prepend
            list.prepend(&mut other_cloned);
            assert!(other_cloned.is_empty());
            assert_eq!(list, appended);
            #[cfg(feature = "length")]
            assert_eq!(list.len(), cloned.len() + other.len());

            let split = list.split_off(at);
            assert_eq!(list, other);
            assert_eq!(split, cloned);
        }
        test_list_split_and_append_and_prepend(0..5, 5..7, 5, 0..7);
        test_list_split_and_append_and_prepend(0..5, None, 5, 0..5);
        test_list_split_and_append_and_prepend(0..5, 5..6, 5, 0..6);
        test_list_split_and_append_and_prepend(0..1, 1..3, 1, 0..3);
        test_list_split_and_append_and_prepend(0..1, None, 1, 0..1);
        test_list_split_and_append_and_prepend(0..1, 1..2, 1, 0..2);
        test_list_split_and_append_and_prepend(None, 0..2, 0, 0..2);
        test_list_split_and_append_and_prepend::<i32, _, _, _>(None, None, 0, None);
        test_list_split_and_append_and_prepend(None, 0..1, 0, 0..1);
    }

    #[test]
    fn list_splice() {
        fn test_list_splice<T, I1, I2, I3>(list: I1, other: I2, at: usize, spliced: I3)
        where
            T: Clone + Eq + Debug,
            I1: IntoIterator<Item = T>,
            I2: IntoIterator<Item = T>,
            I3: IntoIterator<Item = T>,
        {
            let mut list = List::from_iter(list);
            let other = List::from_iter(other);
            let spliced = List::from_iter(spliced);

            list.splice_at(at, other.clone());
            assert_eq!(list, spliced);
            #[cfg(feature = "length")]
            assert_eq!(list.len(), spliced.len());
        }
        test_list_splice(0..5, 5..7, 5, 0..7);
        test_list_splice(0..5, 5..7, 2, (0..2).chain(5..7).chain(2..5));
        test_list_splice(0..5, 5..7, 0, (5..7).chain(0..5));
        test_list_splice(0..5, Some(5), 5, 0..6);
        test_list_splice(0..5, Some(5), 2, (0..2).chain(Some(5)).chain(2..5));
        test_list_splice(0..5, Some(5), 0, Some(5).into_iter().chain(0..5));
        test_list_splice(Some(0), 1..3, 1, 0..3);
        test_list_splice(Some(0), 1..3, 0, (1..3).chain(Some(0)));
        test_list_splice(None, 0..2, 0, 0..2);
        test_list_splice(None, Some(0), 0, Some(0));
        test_list_splice::<i32, _, _, _>(None, None, 0, None);
    }

    #[cfg(feature = "length")]
    #[test]
    fn list_len() {
        let mut list = List::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        list.push_back(1);
        assert_eq!(list.len(), 1);

        list.pop_front();
        assert_eq!(list.len(), 0);

        list.append(&mut List::from_iter(0..5));
        assert_eq!(list.len(), 5);

        list.remove(3);
        assert_eq!(list.len(), 4);

        list.splice_at(3, List::from_iter(5..7));
        assert_eq!(list.len(), 6);

        let other = list.split_off(4);
        assert_eq!(list.len(), 4);
        assert_eq!(other.len(), 2);

        list.prepend(&mut List::from_iter(7..10));
        assert_eq!(list.len(), 7);

        list.clear();
        assert_eq!(list.len(), 0);
    }
}
