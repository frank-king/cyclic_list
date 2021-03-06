use crate::list::{List, Node};
#[cfg(feature = "length")]
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::ptr::NonNull;

/// A cursor over a [`List`].
///
/// A `Cursor` is like an iterator, except that it can freely seek back-and-forth.
///
/// In a list with length *n*, there are *n* + 1 valid locations for the cursor,
/// indexed by 0, 1, ..., *n*, where *n* is the ghost node of the list.
///
/// # Examples
///
/// Here is a simple example showing how the cursors work. (The ghost node of the
/// list is denoted by `#`).
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// // Create a list: [ A B C D #]
/// let list = List::from_iter(['A', 'B', 'C', 'D']);
///
/// // Create a cursor at start: [|A B C D #] (index = 0)
/// let mut cursor = list.cursor_start();
/// assert_eq!(cursor.current(), Some(&'A'));
///
/// // Move cursor forward: [ A|B C D #] (index = 1)
/// assert!(cursor.move_next().is_ok());
/// assert_eq!(cursor.current(), Some(&'B'));
///
/// // Move the cursor to the end: [ A B C D|#] (index = 4)
/// cursor.move_to_end();
/// assert_eq!(cursor.current(), None);
///
/// // Move cursor backward: [ A B C|D #] (index = 3)
/// assert!(cursor.move_prev().is_ok());
/// assert_eq!(cursor.current(), Some(&'D'));
///
/// // Create a cursor in the end and move forward: [ A B C D|#] (index = 4)
/// cursor.move_to_end();
/// assert!(cursor.move_next().is_err());
///
/// // Move cursor forward, cyclically: [|A B C D #] (index = 0)
/// cursor.move_next_cyclic();
/// assert_eq!(cursor.current(), Some(&'A'));
/// ```
#[derive(Clone)]
pub struct Cursor<'a, T: 'a> {
    #[cfg(feature = "length")]
    index: usize,
    pub(crate) current: NonNull<Node<T>>,
    pub(crate) list: &'a List<T>,
}

/// Compare cursors by its position.
///
/// Only cursors belong to the same list and have the same positions
/// are considered equal.
///
/// # Examples
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let list = List::from_iter([1, 2, 3]);
/// let cursor1 = list.cursor_start();
/// let mut cursor2 = cursor1.clone();
/// // The same list, and the same position.
/// assert_eq!(cursor1, cursor2);
///
/// cursor2.move_next_cyclic();
/// // The same list, but different positions.
/// assert_ne!(cursor1, cursor2);
///
/// let another_list = list.clone();
/// let cursor3 = another_list.cursor_start();
/// // Different list, different positions.
/// assert_ne!(cursor1, cursor3);
/// ```
impl<'a, T: 'a> PartialEq for Cursor<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.same_list_with(other) && self.current == other.current
    }
}

impl<'a, T: 'a> Eq for Cursor<'a, T> {}

/// Compare cursors by its position.
///
/// Only cursors belong to the same list can compare, so it is `PartialOrd`
/// but not `Ord`.
///
/// # Examples
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let list = List::from_iter([1, 2, 3]);
/// let cursor1 = list.cursor_start();
/// let mut cursor2 = cursor1.clone();
/// cursor2.move_next_cyclic();
/// // They belong to the same list, can compare.
/// assert!(cursor1 < cursor2);
///
/// let another_list = list.clone();
/// let cursor3 = another_list.cursor_end();
/// // They belong to different lists, cannot compare.
/// assert_eq!(cursor1.partial_cmp(&cursor3), None);
/// ```
#[cfg(feature = "length")]
impl<'a, T: 'a> PartialOrd for Cursor<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if !self.same_list_with(other) {
            return None;
        }
        Some(self.index().cmp(&other.index()))
    }
}

/// A cursor over a [`List`] with editing operations.
///
/// A `CursorMut` is like an iterator, except that it can freely seek back-and-forth,
/// and can safely mutate the list during iteration. This is because the lifetime of
/// its yielded references is tied to its own lifetime, instead of just the underlying
/// list. This means cursors cannot yield multiple elements at once.
///
/// For convenience, [`CursorMut::view`] provides a function to temporarily borrow
/// the list and returns an immutable reference whose lifetime is shorter than the
/// iterator. See the documents for details.
///
/// In a list with length *n*, there are *n* + 1 valid locations for the cursor,
/// indexed by 0, 1, ..., *n*, where *n* is the ghost node of the list.
///
/// # Examples
///
/// ```compile_fail
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let mut list = List::from_iter([1, 2, 3]);
/// let mut cursor = list.cursor_start_mut();
/// println!("{:?}", list.back());
/// println!("{:?}", cursor.current());
/// ```
pub struct CursorMut<'a, T: 'a> {
    #[cfg(feature = "length")]
    index: usize,
    pub(crate) current: NonNull<Node<T>>,
    pub(crate) list: &'a mut List<T>,
}

macro_rules! impl_cursor {
    ($CURSOR:ident) => {
        // Private methods
        impl<'a, T: 'a> $CURSOR<'a, T> {
            pub(crate) fn is_ghost_node(&self) -> bool {
                self.current == self.list.ghost_node()
            }
            pub(crate) fn is_front_node(&self) -> bool {
                self.prev_node() == self.list.ghost_node()
            }
            pub(crate) fn next_node(&self) -> NonNull<Node<T>> {
                // SAFETY: `current.next` is always valid since it is a cyclic list.
                unsafe { self.current.as_ref().next }
            }
            pub(crate) fn prev_node(&self) -> NonNull<Node<T>> {
                // SAFETY: `current.prev` is always valid since it is a cyclic list.
                unsafe { self.current.as_ref().prev }
            }

            /// Move forward the cursor by given steps, without checking whether
            /// it will pass through the ghost node.
            ///
            /// It is unsafe because if the moving passes through the ghost node,
            /// the index will be invalid.
            #[cfg(feature = "length")]
            unsafe fn seek_forward_fast(&mut self, steps: usize) {
                self.index = self.index.saturating_add(steps);
                (0..steps).for_each(|_| self.current = self.next_node());
            }

            /// Move backward the cursor by given steps, without checking whether
            /// it will pass through the ghost node.
            ///
            /// It is unsafe because if the moving passes through the ghost node,
            /// the index will be invalid.
            #[cfg(feature = "length")]
            unsafe fn seek_backward_fast(&mut self, steps: usize) {
                self.index = self.index.saturating_sub(steps);
                (0..steps).for_each(|_| self.current = self.prev_node());
            }
        }

        /// Public methods of cursor moving or locating
        impl<'a, T: 'a> $CURSOR<'a, T> {
            #[cfg(feature = "length")]
            /// Return the index of the cursor
            pub fn index(&self) -> usize {
                self.index
            }

            /// Returns `true` if the `List` is empty. See [`List::is_empty`].
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(1) time.
            pub fn is_empty(&self) -> bool {
                self.list.is_empty()
            }

            /// Move the cursor to the next position, where passing
            /// through the ghost node is allowed.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*1*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_end();
            ///
            /// // The cursor is at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            /// cursor.move_next_cyclic();
            ///
            /// // The cursor is now at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            /// ```
            pub fn move_next_cyclic(&mut self) {
                if self.is_empty() {
                    return;
                }
                #[cfg(feature = "length")]
                if self.is_ghost_node() {
                    self.index = 0;
                } else {
                    self.index += 1;
                }
                self.current = self.next_node();
            }

            /// Move the cursor to the previous position, where passing
            /// through the ghost node is allowed.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*1*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_start();
            ///
            /// // The cursor is at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            /// cursor.move_prev_cyclic();
            ///
            /// // The cursor is now at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            /// ```
            pub fn move_prev_cyclic(&mut self) {
                if self.is_empty() {
                    return;
                }
                #[cfg(feature = "length")]
                if self.is_front_node() {
                    self.index = self.list.len();
                } else {
                    self.index -= 1;
                }
                self.current = self.prev_node();
            }

            /// Move the cursor to the next position, or return an error
            /// when passing through the ghost node is happened.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*1*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_end();
            ///
            /// // The cursor is at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            ///
            /// // Forbid to move passing through the ghost node
            /// assert!(cursor.move_next().is_err());
            ///
            /// // the cursor is still at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            /// ```
            pub fn move_next(&mut self) -> Result<(), &'static str> {
                if !self.is_empty() && !self.is_ghost_node() {
                    self.move_next_cyclic();
                    return Ok(());
                }
                Err("`move_next` across ghost boundary")
            }

            /// Move the cursor to the previous position, or return an error
            /// when passing through the ghost node is happened.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*1*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_start();
            ///
            /// // The cursor is at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            ///
            /// // Forbid to move passing through the ghost node
            /// assert!(cursor.move_prev().is_err());
            ///
            /// // The cursor is stiil at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            /// ```
            pub fn move_prev(&mut self) -> Result<(), &'static str> {
                if !self.is_empty() && !self.is_front_node() {
                    self.move_prev_cyclic();
                    return Ok(());
                }
                Err("`move_prev` across ghost boundary")
            }

            /// Move forward the cursor by given steps, or return an error
            /// which tells the actual steps it has moved, when passing through
            /// the ghost node is happened.
            ///
            /// If an error occurs, the cursor will stay at the ghost node.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*n*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_start();
            ///
            /// // The cursor is at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            ///
            /// // Forbid to move passing through the ghost node
            /// assert_eq!(cursor.seek_forward(5), Err(3));
            ///
            /// // the cursor is now at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            /// ```
            pub fn seek_forward(&mut self, steps: usize) -> Result<(), usize> {
                (0..steps).try_for_each(|i| self.move_next().map_err(|_| i))
            }

            /// Move backward the cursor by given steps, or return an error
            /// which tells the actual steps it has moved, when passing through
            /// the ghost node is happened.
            ///
            /// If an error occurs, the cursor will stay at the first node.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*n*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_end();
            ///
            /// // the cursor is at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            ///
            /// // Forbid to move passing through the ghost node
            /// assert_eq!(cursor.seek_backward(5), Err(3));
            ///
            /// // the cursor is now at the ghost node
            /// assert_eq!(cursor.current(), Some(&1));
            /// ```
            pub fn seek_backward(&mut self, steps: usize) -> Result<(), usize> {
                (0..steps).try_for_each(|i| self.move_prev().map_err(|_| i))
            }

            /// Move the cursor to the given position `target`, or return the `target`
            /// as an error when `target > len`.
            ///
            /// If an error occurs, the cursor will stay put.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*n*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_start();
            ///
            /// // The cursor is at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            ///
            /// // Move cursor to a valid place (at the third node)
            /// assert!(cursor.try_seek_to(2).is_ok());
            /// assert_eq!(cursor.current(), Some(&3));
            ///
            /// // Forbid to move to a invalid place
            /// assert_eq!(cursor.try_seek_to(5), Err(5));
            ///
            /// // The cursor is still at the third node
            /// assert_eq!(cursor.current(), Some(&3));
            /// ```
            pub fn try_seek_to(&mut self, target: usize) -> Result<(), usize> {
                #[cfg(not(feature = "length"))]
                {
                    let current = self.current;
                    self.move_to_start();
                    if self.seek_forward(target).is_err() {
                        self.current = current;
                        return Err(target);
                    }
                }
                #[cfg(feature = "length")]
                {
                    if target == self.index {
                        return Ok(());
                    }
                    let len = self.list.len();
                    match target {
                        target if target > len => return Err(target),
                        0 => self.move_to_start(),
                        target if target == len => self.move_to_end(),
                        _ => unsafe {
                            // current=c, target=t, ghost=#
                            if target > self.index {
                                // target is at the right side of current: [   c----->t   #]
                                if target - self.index <= len - target {
                                    // target is near the right side of current: [    c-->t     #]
                                    self.seek_forward_fast(target - self.index);
                                } else {
                                    // target is far from the right side of current: [ c     t<--#]
                                    self.move_to_end();
                                    self.seek_backward_fast(len - target);
                                }
                            } else {
                                // target is at the left side of current: [   t<-----c   #]
                                if self.index - target <= target {
                                    // target is near the left side of current: [    t<--c     #]
                                    self.seek_backward_fast(self.index - target);
                                } else {
                                    // target is far from the left side of current: [-->t      c #]
                                    self.move_to_start();
                                    self.seek_forward_fast(target);
                                }
                            }
                        },
                    }
                }
                Ok(())
            }

            /// Move the cursor to the given position `target`.
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*n*) time.
            ///
            /// # Panics
            ///
            /// Panics if `target > len`.
            ///
            /// # Examples
            ///
            /// ```should_panic
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_start();
            ///
            /// // Panics if moving to a invalid place
            /// cursor.seek_to(5);
            /// ```
            pub fn seek_to(&mut self, target: usize) {
                self.try_seek_to(target)
                    .expect("Cannot seek to nonexistent place");
            }

            /// Set the cursor to the start of the list (i.e. the first node).
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*1*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_end();
            ///
            /// // The cursor is at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            /// cursor.move_to_start();
            ///
            /// // The cursor is now at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            /// ```
            #[inline]
            pub fn move_to_start(&mut self) {
                #[cfg(feature = "length")]
                {
                    self.index = 0;
                }
                self.current = self.list.front_node();
            }

            /// Set the cursor to the end of the list (i.e. the ghost node).
            ///
            /// # Complexity
            ///
            /// This operation should compute in *O*(*1*) time.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// let mut cursor = list.cursor_start();
            ///
            /// // The cursor is at the first node
            /// assert_eq!(cursor.current(), Some(&1));
            /// cursor.move_to_end();
            ///
            /// // The cursor is now at the ghost node
            /// assert_eq!(cursor.previous(), Some(&3));
            /// ```
            #[inline]
            pub fn move_to_end(&mut self) {
                #[cfg(feature = "length")]
                {
                    self.index = self.list.len();
                }
                self.current = self.list.ghost_node();
            }

            /// Return an immutable reference of current node of the cursor,
            /// or return `None` if it is located at the first node.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// assert_eq!(list.cursor(0).current(), Some(&1));
            /// assert_eq!(list.cursor(1).current(), Some(&2));
            /// assert_eq!(list.cursor(2).current(), Some(&3));
            /// assert_eq!(list.cursor(3).current(), None);
            /// ```
            pub fn current(&self) -> Option<&'a T> {
                if self.is_ghost_node() {
                    return None;
                }
                // SAFETY: it is safe because non-ghost nodes must hold a
                // valid element.
                unsafe { Some(&self.current.as_ref().element) }
            }

            /// Return an immutable reference of previous node of the cursor,
            /// or return `None` if it is located at the first node.
            ///
            /// This is useful where using the cursor as a reversed cursor.
            /// See [`CursorBackIter`] for details.
            ///
            /// # Examples
            ///
            /// ```
            /// use cyclic_list::List;
            /// use std::iter::FromIterator;
            ///
            /// let list = List::from_iter([1, 2, 3]);
            /// assert_eq!(list.cursor(0).previous(), None);
            /// assert_eq!(list.cursor(1).previous(), Some(&1));
            /// assert_eq!(list.cursor(2).previous(), Some(&2));
            /// assert_eq!(list.cursor(3).previous(), Some(&3));
            /// ```
            pub fn previous(&self) -> Option<&'a T> {
                if self.is_front_node() {
                    return None;
                }
                // SAFETY: it is safe because the previous node of a non-first node
                // is never a ghost node, and non-ghost nodes must hold a valid element.
                Some(unsafe { &self.prev_node().as_ref().element })
            }
        }

        impl<'a, T: fmt::Debug + 'a> fmt::Debug for $CURSOR<'a, T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                let mut f = f.debug_struct(stringify!($CURSOR));
                f.field("list", &self.list)
                    .field("current", &self.current());
                #[cfg(feature = "length")]
                f.field("index", &self.index);
                f.finish()
            }
        }
    };
}

impl_cursor!(CursorMut);
impl_cursor!(Cursor);

impl<'a, T: 'a> Cursor<'a, T> {
    pub(crate) fn new(
        list: &'a List<T>,
        current: NonNull<Node<T>>,
        #[cfg(feature = "length")] index: usize,
    ) -> Self {
        Self {
            #[cfg(feature = "length")]
            index,
            current,
            list,
        }
    }

    fn same_list_with(&self, other: &Self) -> bool {
        std::ptr::eq(self.list, other.list)
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub(crate) fn new(
        list: &'a mut List<T>,
        current: NonNull<Node<T>>,
        #[cfg(feature = "length")] index: usize,
    ) -> Self {
        Self {
            #[cfg(feature = "length")]
            index,
            current,
            list,
        }
    }

    /// Insert a new item before the given node `next`.
    ///
    /// It is unsafe because it does not check whether `next` is
    /// belong to the current list that the cursor points to.
    unsafe fn insert_before(&mut self, next: NonNull<Node<T>>, item: T) -> NonNull<Node<T>> {
        let node = Node::new_detached(item);
        self.list.attach_node(next, node);
        node
    }
}

/// Methods that does not change the linking structure of the list.
impl<'a, T: 'a> CursorMut<'a, T> {
    /// Return an mutable reference of current node of the cursor,
    /// or return `None` if it is located at the first node.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    ///
    /// // Create a cursor and mutate the element in the current node.
    /// let mut cursor = list.cursor_mut(0);
    /// *cursor.current_mut().unwrap() *= 5;
    /// assert_eq!(cursor.current(), Some(&5));
    ///
    /// // Cannot mutate the ghost node.
    /// assert!(list.cursor_mut(3).current_mut().is_none());
    /// ```
    pub fn current_mut(&mut self) -> Option<&'a mut T> {
        if self.is_ghost_node() {
            return None;
        }
        // SAFETY: it is safe because non-ghost nodes must hold a
        // valid element.
        unsafe { Some(&mut self.current.as_mut().element) }
    }

    /// Return a mutable reference of previous node of the cursor,
    /// or return `None` if it is located at the first node.
    ///
    /// This is useful where using the cursor as a reversed cursor.
    /// See [`CursorBackIterMut`] for details.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([1, 2, 3]);
    ///
    /// // Create a cursor and mutate the element in the current node.
    /// let mut cursor = list.cursor_mut(3);
    /// *cursor.previous_mut().unwrap() *= 5;
    /// assert_eq!(cursor.previous(), Some(&15));
    ///
    /// // Cannot mutate the ghost node.
    /// assert!(list.cursor_mut(0).previous_mut().is_none());
    /// ```
    pub fn previous_mut(&mut self) -> Option<&'a mut T> {
        if self.is_front_node() {
            return None;
        }
        // SAFETY: it is safe because the previous node of a non-first node
        // is never a ghost node, and non-ghost nodes must hold a valid element.
        Some(unsafe { &mut self.prev_node().as_mut().element })
    }

    /// Re-borrow the mutable cursor as a short-lived immutable one.
    pub fn as_cursor(&self) -> Cursor<'_, T> {
        Cursor::new(
            self.list,
            self.current,
            #[cfg(feature = "length")]
            self.index,
        )
    }

    /// Convert the mutable cursor to an immutable one.
    pub fn into_cursor(self) -> Cursor<'a, T> {
        Cursor::new(
            self.list,
            self.current,
            #[cfg(feature = "length")]
            self.index,
        )
    }

    /// Temporarily view the list via an immutable reference.
    ///
    /// This is useful where the list is not able to read while a
    /// mutable cursor is created and being used. This method
    /// provides an ability of temporarily reading the list.
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
    /// // Temporarily view the list
    /// assert_eq!(cursor.view().back(), Some(&3));
    ///
    /// // Won't compile because list is already borrowed mutably.
    /// // assert_eq!(list.back(), Some(&3));
    ///
    /// cursor.insert(4);
    /// assert_eq!(Vec::from_iter(list), vec![4, 1, 2, 3]);
    /// ```
    pub fn view(&self) -> &List<T> {
        self.list
    }
}

/// Methods that might change the linking structure of the list.
impl<'a, T: 'a> CursorMut<'a, T> {
    /// Add an element first in the list.
    ///
    /// It is the same as [`List::push_front`], except it avoids
    /// another mutable borrow of the list while the mutable cursor
    /// is being used.
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
    /// cursor.insert(4);
    /// cursor.push_front(0);
    /// // Won't compile because list is already mutably borrowed,
    /// // and the cursor is used later.
    /// // list.push_front(0);
    ///
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 5);
    /// assert_eq!(cursor.previous(), Some(&4));
    ///
    /// assert_eq!(Vec::from_iter(list), vec![0, 1, 2, 3, 4]);
    /// ```
    pub fn push_front(&mut self, item: T) {
        self.list.push_front(item);
        #[cfg(feature = "length")]
        {
            self.index += 1;
        }
    }

    /// Remove the first element and return it, or `None` if the list is
    /// empty.
    ///
    /// It is the same as [`List::pop_front`], except it avoids
    /// another mutable borrow of the list while the mutable cursor
    /// is being used.
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
    /// cursor.insert(4); // becomes [1, 2, 3, 4], points to # (the ghost node)
    /// assert_eq!(cursor.previous(), Some(&4));
    ///
    /// assert_eq!(cursor.pop_front(), Some(1)); // becomes [2, 3, 4], points to #
    /// // Won't compile because list is already mutably borrowed,
    /// // and the cursor is used later.
    /// // assert_eq!(list.pop_front(), Some(1));
    ///
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 3);
    /// assert_eq!(cursor.previous(), Some(&4));
    ///
    /// assert_eq!(Vec::from_iter(list), vec![2, 3, 4]);
    /// ```
    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let is_front = self.is_front_node();
        let item = self.list.pop_front();
        if is_front {
            self.current = self.list.front_node();
        }
        #[cfg(feature = "length")]
        if !is_front {
            self.index -= 1;
        }
        item
    }

    /// Append an element to the back of a list.
    ///
    /// It is the same as [`List::push_back`], except it avoids
    /// another mutable borrow of the list while the mutable cursor
    /// is being used.
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
    /// cursor.insert(0);
    ///
    /// cursor.push_back(4);
    /// // Won't compile because list is already mutably borrowed,
    /// // and the cursor is used later.
    /// // list.push_back(4);
    ///
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 1);
    /// assert_eq!(cursor.current(), Some(&1));
    ///
    /// assert_eq!(Vec::from_iter(list), vec![0, 1, 2, 3, 4]);
    /// ```
    pub fn push_back(&mut self, item: T) {
        self.list.push_back(item)
    }

    /// Remove the last element from a list and return it, or `None` if
    /// it is empty.
    ///
    /// It is the same as [`List::pop_back`], except it avoids
    /// another mutable borrow of the list while the mutable cursor
    /// is being used.
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
    /// cursor.insert(0);
    ///
    /// assert_eq!(cursor.pop_back(), Some(3));
    /// // Won't compile because list is already mutably borrowed,
    /// // and the cursor is used later.
    /// // assert_eq!(list.pop_back(), Some(3));
    ///
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 1);
    /// assert_eq!(cursor.current(), Some(&1));
    ///
    /// assert_eq!(Vec::from_iter(list), vec![0, 1, 2]);
    /// ```
    pub fn pop_back(&mut self) -> Option<T> {
        self.list.pop_back()
    }

    /// Add an element before the cursor position.
    ///
    /// After insertion, the cursor stays put but its `index` becomes
    /// `index + 1`.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(1) time.
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
    /// cursor.insert(4); // becomes [1, 4, 2, 3]
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 2);
    /// assert_eq!(cursor.current(), Some(&2));
    ///
    /// cursor.move_to_end();
    /// cursor.insert(5); // becomes [1, 4, 2, 3, 5]
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 5);
    /// assert_eq!(cursor.previous(), Some(&5));
    ///
    /// assert_eq!(Vec::from_iter(list), vec![1, 4, 2, 3, 5]);
    /// ```
    pub fn insert(&mut self, item: T) {
        // SAFETY: `self.current` is a valid node in the list, so it is safe.
        unsafe { self.insert_before(self.current, item) };
        #[cfg(feature = "length")]
        {
            self.index += 1;
        }
    }

    /// Remove the element at the cursor and return it, or return `None`
    /// if the cursor is at the ghost node. After removal, the cursor
    /// is moved to the next node unless no removing is happened.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*1*) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter(0..10);
    /// let mut cursor = list.cursor_mut(5);
    ///
    /// assert_eq!(cursor.remove(), Some(5)); // becomes [0, 1, 2, 3, 4, 6, 7, 8, 9]
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 5);
    /// assert_eq!(cursor.current(), Some(&6));
    ///
    /// cursor.move_to_start();
    /// assert_eq!(cursor.remove(), Some(0)); // becomes [1, 2, 3, 4, 6, 7, 8, 9]
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 0);
    /// assert_eq!(cursor.current(), Some(&1));
    ///
    /// cursor.move_to_end();
    /// assert_eq!(cursor.remove(), None); // removing at the ghost node returns `None`
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 8);
    /// assert_eq!(cursor.current(), None);
    ///
    /// assert_eq!(Vec::from_iter(list), vec![1, 2, 3, 4, 6, 7, 8, 9]);
    /// ```
    pub fn remove(&mut self) -> Option<T> {
        if self.is_ghost_node() {
            return None;
        }
        // SAFETY: `self.current` is a valid non-ghost node in the list, so it is safe.
        let node = unsafe { self.list.detach_node(self.current) };
        self.current = self.next_node();
        Some(node.element)
    }

    /// Remove the element before the cursor and return it, or return `None` if
    /// the cursor is at the first node. After removal, the cursor is not moved,
    /// but its `index` becomes `index - 1`.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*1*) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter(0..10);
    /// let mut cursor = list.cursor_mut(5);
    ///
    /// assert_eq!(cursor.backspace(), Some(4)); // becomes [0, 1, 2, 3, 5, 6, 7, 8, 9]
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 4);
    /// assert_eq!(cursor.current(), Some(&5));
    ///
    /// cursor.move_to_start();
    /// assert_eq!(cursor.backspace(), None); // backspacing at the first node returns `None`
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 0);
    /// assert_eq!(cursor.current(), Some(&0));
    ///
    /// cursor.move_to_end();
    /// assert_eq!(cursor.backspace(), Some(9)); // becomes [0, 1, 2, 3, 5, 6, 7, 8]
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 8);
    /// assert_eq!(cursor.current(), None);
    ///
    /// assert_eq!(Vec::from_iter(list), vec![0, 1, 2, 3, 5, 6, 7, 8]);
    /// ```
    pub fn backspace(&mut self) -> Option<T> {
        self.move_prev().ok().and_then(|_| self.remove())
    }

    /// Split the list into two after the current element (inclusive). This will
    /// return a new list consisting of everything after the cursor (inclusive),
    /// with the original list retaining everything before (exclusive).
    ///
    /// If the cursor is pointing at the ghost node, `None` will be returned.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*1*) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter(0..10);
    /// let mut cursor = list.cursor_mut(5);
    ///
    /// // Split the list at cursor position (index = 5), and leave
    /// // all the nodes before cursor (exclusive).
    /// let list2 = cursor.split().unwrap();
    /// assert_eq!(cursor.current(), None);
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 5);
    ///
    /// assert_eq!(Vec::from_iter(list2), vec![5, 6, 7, 8, 9]);
    /// assert_eq!(Vec::from_iter(list), vec![0, 1, 2, 3, 4]);
    /// ```
    pub fn split(&mut self) -> Option<List<T>> {
        if self.is_ghost_node() {
            return None;
        }
        #[cfg(feature = "length")]
        let len = self.list.len - self.index;
        // After splitting, the current node is pointing to the ghost node.
        let current = std::mem::replace(&mut self.current, self.list.ghost_node());
        // SAFETY: since current is a non-ghost node, the range from current to
        // the ghost node is a valid range in the list, and thus it is safe.
        unsafe {
            Some(List::from_detached(self.list.detach_nodes(
                current,
                self.list.back_node(),
                #[cfg(feature = "length")]
                len,
            )))
        }
    }

    /// Split the list into two before the current element (exclusive). This will
    /// return a new list consisting of everything before the cursor (exclusive),
    /// with the original list retaining everything after (inclusive).
    ///
    /// If the cursor is pointing at the front node, `None` will be returned.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*1*) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter(0..10);
    /// let mut cursor = list.cursor_mut(5);
    ///
    /// // Split the list at cursor position (index = 5), and leave
    /// // all the nodes after cursor (inclusive).
    /// let list2 = cursor.split_before().unwrap();
    /// assert_eq!(cursor.current(), Some(&5));
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 0);
    ///
    /// assert_eq!(Vec::from_iter(list2), vec![0, 1, 2, 3, 4]);
    /// assert_eq!(Vec::from_iter(list), vec![5, 6, 7, 8, 9]);
    /// ```
    pub fn split_before(&mut self) -> Option<List<T>> {
        if self.is_front_node() {
            return None;
        }
        // After splitting, the current node becomes a front node, so its
        // index becomes 0.
        #[cfg(feature = "length")]
        let len = std::mem::replace(&mut self.index, 0);
        // SAFETY: since current is a non-front node, the range from the front node
        // to the current node is a valid range in the list, and thus it is safe.
        unsafe {
            Some(List::from_detached(self.list.detach_nodes(
                self.list.front_node(),
                self.prev_node(),
                #[cfg(feature = "length")]
                len,
            )))
        }
    }

    /// Splice another list between the current node and its previous node.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*1*) time.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut list = List::from_iter([0, 1, 7, 8, 9]);
    /// let list2 = List::from_iter([2, 3, 4, 5, 6]);
    /// let mut cursor = list.cursor_mut(2);
    ///
    /// // Splice another list at the cursor position.
    /// cursor.splice(list2);
    /// assert_eq!(cursor.current(), Some(&7));
    /// #[cfg(feature = "length")]
    /// assert_eq!(cursor.index(), 7);
    ///
    /// assert_eq!(Vec::from_iter(list), Vec::from_iter(0..10));
    /// ```
    pub fn splice(&mut self, other: List<T>) {
        if let Some(detached) = other.into_detached() {
            #[cfg(feature = "length")]
            {
                self.index += detached.len;
            }
            // SAFETY: `self.current.prev` and `self.current` are valid nodes in the list,
            // and they are adjacent, so it is safe.
            unsafe { self.list.attach_nodes(self.current, detached) };
        }
    }
}

/// `CursorIter` provides an cursor-like iterator that are cyclic
/// and not fused.
///
/// If you are looking for container-like iterators,
/// see [`Iter`] and [`IterMut`] for details.
///
/// # Examples
///
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let list = List::from_iter([1, 2, 3]);
/// // Create a cursor iterator
/// let mut cursor_iter = list.cursor_start().into_iter();
/// assert_eq!(cursor_iter.next(), Some(&1));
/// assert_eq!(cursor_iter.next(), Some(&2));
/// assert_eq!(cursor_iter.next(), Some(&3));
/// assert_eq!(cursor_iter.next(), None);
/// assert_eq!(cursor_iter.next(), Some(&1)); // Not fused and cyclic
///
/// // Convert back to a cursor
/// let mut cursor = cursor_iter.into_cursor();
/// assert_eq!(cursor.current(), Some(&2));
/// ```
///
/// [`Iter`]: crate::list::iterator::Iter
/// [`IterMut`]: crate::list::iterator::IterMut
pub struct CursorIter<'a, T: 'a> {
    pub(crate) cursor: Cursor<'a, T>,
}

/// `CursorIterMut` provides an cursor-like mutable iterator
/// that are cyclic and not fused.
///
/// If you are looking for container-like iterators,
/// see [`Iter`] and [`IterMut`] for details.
///
/// # Examples
///
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let mut list = List::from_iter([1, 2, 3]);
/// // Create a mutable cursor iterator
/// let mut cursor_iter = list.cursor_start_mut().into_iter();
/// *cursor_iter.next().unwrap() *= 5;
/// *cursor_iter.next().unwrap() *= 5;
/// *cursor_iter.next().unwrap() *= 5;
/// assert_eq!(cursor_iter.next(), None);
/// assert_eq!(cursor_iter.next(), Some(&mut 5)); // return back to the first element
/// assert_eq!(cursor_iter.next(), Some(&mut 10));
///
/// // Convert back to a cursor
/// let mut cursor = cursor_iter.into_cursor_mut();
/// assert_eq!(cursor.current(), Some(&15));
/// ```
///
/// [`Iter`]: crate::list::iterator::Iter
/// [`IterMut`]: crate::list::iterator::IterMut
pub struct CursorIterMut<'a, T: 'a> {
    pub(crate) cursor: CursorMut<'a, T>,
}

/// `CursorBackIter` is largely the same asa [`CursorIter`],
/// except that the cursors are moving in an opposite direction.
///
/// # Examples
///
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let list = List::from_iter([1, 2, 3]);
/// // Create a cursor back iterator
/// let mut cursor_iter = list.cursor_end().into_iter().rev();
/// assert_eq!(cursor_iter.next(), Some(&3));
/// assert_eq!(cursor_iter.next(), Some(&2));
/// assert_eq!(cursor_iter.next(), Some(&1));
/// assert_eq!(cursor_iter.next(), None);
/// assert_eq!(cursor_iter.next(), Some(&3)); // Not fused and cyclic
///
/// // Convert back to a cursor
/// let mut cursor = cursor_iter.into_cursor();
/// assert_eq!(cursor.previous(), Some(&2));
/// ```
pub struct CursorBackIter<'a, T: 'a> {
    pub(crate) cursor: Cursor<'a, T>,
}

/// `CursorBackIterMut` is largely the same asa [`CursorIterMut`],
/// except that the cursors are moving in an opposite direction.
///
/// # Examples
///
/// ```
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let mut list = List::from_iter([1, 2, 3]);
/// // Create a mutable cursor back iterator
/// let mut cursor_iter = list.cursor_end_mut().into_iter().rev();
/// *cursor_iter.next().unwrap() *= 5;
/// *cursor_iter.next().unwrap() *= 5;
/// *cursor_iter.next().unwrap() *= 5;
/// assert_eq!(cursor_iter.next(), None);
/// assert_eq!(cursor_iter.next(), Some(&mut 15)); // return back to the first element
/// assert_eq!(cursor_iter.next(), Some(&mut 10));
///
/// // Convert back to a cursor
/// let mut cursor = cursor_iter.into_cursor_mut();
/// assert_eq!(cursor.previous(), Some(&5));
/// ```
pub struct CursorBackIterMut<'a, T: 'a> {
    pub(crate) cursor: CursorMut<'a, T>,
}

impl<'a, T: 'a> CursorIter<'a, T> {
    /// Convert the cursor iterator to a cursor.
    pub fn into_cursor(self) -> Cursor<'a, T> {
        self.cursor
    }
    /// Make a back iterator which reverses the iterating direction.
    pub fn rev(self) -> CursorBackIter<'a, T> {
        CursorBackIter {
            cursor: self.cursor,
        }
    }
    /// Peek the next item being iterated without consume it.
    pub fn peek(&self) -> Option<&'a T> {
        self.cursor.current()
    }
}

impl<'a, T: 'a> CursorIterMut<'a, T> {
    /// Convert the mutable cursor iterator to an immutable cursor
    pub fn into_cursor(self) -> Cursor<'a, T> {
        self.cursor.into_cursor()
    }
    /// Convert the mutable cursor iterator to a mutable cursor.
    pub fn into_cursor_mut(self) -> CursorMut<'a, T> {
        self.cursor
    }
    /// Make a mutable cursor back iterator which reverses the
    /// iterating direction.
    pub fn rev(self) -> CursorBackIterMut<'a, T> {
        CursorBackIterMut {
            cursor: self.cursor,
        }
    }
    /// Peek the next item being iterated (mutably) without consume it.
    pub fn peek(&mut self) -> Option<&'a mut T> {
        self.cursor.current_mut()
    }
}

impl<'a, T: 'a> CursorBackIter<'a, T> {
    /// Convert the cursor back iterator to a cursor.
    pub fn into_cursor(self) -> Cursor<'a, T> {
        self.cursor
    }
    /// Make a normal cursor iterator which recovers the
    /// original iterating direction.
    pub fn rev(self) -> CursorIter<'a, T> {
        CursorIter {
            cursor: self.cursor,
        }
    }
    /// Peek the next item being iterated without consume it.
    ///
    /// Note that the iterating direction is opposite to the
    /// normal cursor iterator.
    pub fn peek(&self) -> Option<&'a T> {
        self.cursor.previous()
    }
}

impl<'a, T: 'a> CursorBackIterMut<'a, T> {
    /// Convert the mutable cursor back iterator to an immutable cursor.
    pub fn into_cursor(self) -> Cursor<'a, T> {
        self.cursor.into_cursor()
    }
    /// Convert the mutable cursor back iterator to a mutable cursor.
    pub fn into_cursor_mut(self) -> CursorMut<'a, T> {
        self.cursor
    }
    /// Make a normal mutable cursor iterator which recovers the
    /// original iterating direction.
    pub fn rev(self) -> CursorIterMut<'a, T> {
        CursorIterMut {
            cursor: self.cursor,
        }
    }
    /// Peek the next item being iterated (mutably) without consume it.
    ///
    /// Note that the iterating direction is opposite to the
    /// normal cursor iterator.
    pub fn peek(&mut self) -> Option<&'a mut T> {
        self.cursor.previous_mut()
    }
}

impl<'a, T: 'a> From<CursorIter<'a, T>> for Cursor<'a, T> {
    fn from(cursor_iter: CursorIter<'a, T>) -> Self {
        cursor_iter.into_cursor()
    }
}

impl<'a, T: 'a> From<CursorIterMut<'a, T>> for CursorMut<'a, T> {
    fn from(cursor_iter: CursorIterMut<'a, T>) -> Self {
        cursor_iter.into_cursor_mut()
    }
}

impl<'a, T: 'a> From<CursorMut<'a, T>> for Cursor<'a, T> {
    fn from(cursor: CursorMut<'a, T>) -> Self {
        cursor.into_cursor()
    }
}

impl<'a, T: 'a> From<CursorIterMut<'a, T>> for CursorIter<'a, T> {
    fn from(cursor_iter: CursorIterMut<'a, T>) -> Self {
        cursor_iter.into_cursor().into_iter()
    }
}

unsafe impl<T: Sync> Send for Cursor<'_, T> {}

unsafe impl<T: Sync> Sync for Cursor<'_, T> {}

unsafe impl<T: Send> Send for CursorMut<'_, T> {}

unsafe impl<T: Sync> Sync for CursorMut<'_, T> {}

unsafe impl<T: Sync> Send for CursorIter<'_, T> {}

unsafe impl<T: Sync> Sync for CursorIter<'_, T> {}

unsafe impl<T: Send> Send for CursorIterMut<'_, T> {}

unsafe impl<T: Sync> Sync for CursorIterMut<'_, T> {}

unsafe impl<T: Sync> Send for CursorBackIter<'_, T> {}

unsafe impl<T: Sync> Sync for CursorBackIter<'_, T> {}

unsafe impl<T: Send> Send for CursorBackIterMut<'_, T> {}

unsafe impl<T: Sync> Sync for CursorBackIterMut<'_, T> {}

#[cfg(test)]
mod tests {
    use crate::list::cursor::{Cursor, CursorMut};
    use crate::List;
    use std::cmp::Ordering;
    use std::fmt::Debug;
    use std::iter::FromIterator;

    #[test]
    fn cursor_read() {
        fn test_cursor_read<T, I>(list: I)
        where
            T: Debug + Eq + Clone,
            I: IntoIterator<Item = T> + Clone,
        {
            let vec = Vec::from_iter(list.clone());
            let len = vec.len();
            let mut list = List::from_iter(list);
            for i in 0..len {
                assert_eq!(list.cursor(i).current(), vec.get(i));
                assert_eq!(list.cursor_mut(i).current(), vec.get(i));
            }
            assert!(list.cursor(len).current().is_none());
            assert!(list.cursor_mut(len).current().is_none());
            for i in 1..=len {
                assert_eq!(list.cursor(i).previous(), vec.get(i - 1));
                assert_eq!(list.cursor_mut(i).previous(), vec.get(i - 1));
            }
            assert!(list.cursor(0).previous().is_none());
            assert!(list.cursor_mut(0).previous().is_none());
        }
        test_cursor_read(0..5);
        test_cursor_read(["123", "abc"]);
        test_cursor_read(Some(0));
        test_cursor_read::<i32, _>(None);
    }

    #[test]
    fn cursor_write() {
        fn test_cursor_write<T, F, I1, I2>(input: I1, f: F, expected: I2)
        where
            T: Debug + Eq + Clone,
            F: FnMut(&mut T) + Clone,
            I1: IntoIterator<Item = T> + Clone,
            I2: IntoIterator<Item = T>,
        {
            let mut vec = Vec::from_iter(expected);
            let len = vec.len();
            let mut list = List::from_iter(input.clone());
            for i in 0..len {
                let mut f = f.clone();
                assert_eq!(
                    list.cursor_mut(i).current_mut().map(|item| {
                        f(item);
                        item
                    }),
                    vec.get_mut(i)
                );
            }
            assert!(list.cursor_mut(len).current_mut().is_none());

            let mut list = List::from_iter(input.clone());
            for i in 1..=len {
                let mut f = f.clone();
                assert_eq!(
                    list.cursor_mut(i).previous_mut().map(|item| {
                        f(item);
                        item
                    }),
                    vec.get_mut(i - 1)
                );
            }
            assert!(list.cursor_mut(0).previous_mut().is_none());
        }
        test_cursor_write(0..5, |i| *i *= 2, [0, 2, 4, 6, 8]);
        test_cursor_write(
            [String::from("123"), String::from("abc")],
            |s| s.push_str("#"),
            [String::from("123#"), String::from("abc#")],
        );
        test_cursor_write(Some(0), |_| {}, Some(0));
        test_cursor_write::<i32, _, _, _>(None, |i| *i *= 2, None);
    }

    #[test]
    fn cursor_insert_and_remove() {
        fn test_cursor_insert_and_remove<T, I1, I2>(input: I1, at: usize, item: T, expected: I2)
        where
            T: Debug + Eq + Clone,
            I1: IntoIterator<Item = T>,
            I2: IntoIterator<Item = T>,
        {
            let input = List::from_iter(input);
            let mut list = input.clone();
            let expected = List::from_iter(expected);
            let mut cursor = list.cursor_mut(at);

            cursor.insert(item.clone());
            #[cfg(feature = "length")]
            assert_eq!(cursor.index(), at + 1);
            assert_eq!(cursor.previous(), Some(&item));
            assert_eq!(cursor.view(), &expected);

            assert_eq!(cursor.backspace(), Some(item.clone()));
            #[cfg(feature = "length")]
            assert_eq!(cursor.index(), at);
            assert_eq!(cursor.view(), &input);

            cursor.insert(item.clone());
            assert!(cursor.move_prev().is_ok());
            #[cfg(feature = "length")]
            assert_eq!(cursor.index(), at);
            assert_eq!(cursor.current(), Some(&item));
            assert_eq!(cursor.view(), &expected);

            assert_eq!(cursor.remove(), Some(item.clone()));
            #[cfg(feature = "length")]
            assert_eq!(cursor.index(), at);
            assert_eq!(cursor.view(), &input);
        }
        test_cursor_insert_and_remove(0..5, 5, 5, 0..6);
        test_cursor_insert_and_remove(0..5, 2, 5, (0..2).chain(Some(5)).chain(2..5));
        test_cursor_insert_and_remove(0..5, 0, 5, (5..=5).chain(0..5));
        test_cursor_insert_and_remove(0..2, 2, 2, [0, 1, 2]);
        test_cursor_insert_and_remove(0..2, 1, 2, [0, 2, 1]);
        test_cursor_insert_and_remove(0..2, 0, 2, [2, 0, 1]);
        test_cursor_insert_and_remove(Some(0), 1, 1, [0, 1]);
        test_cursor_insert_and_remove(Some(0), 0, 1, [1, 0]);
        test_cursor_insert_and_remove(None, 0, 0, Some(0));

        let mut empty = List::<i32>::new();
        let mut cursor = empty.cursor_end_mut();

        #[cfg(feature = "length")]
        assert_eq!(cursor.index(), 0);
        assert!(cursor.remove().is_none());

        #[cfg(feature = "length")]
        assert_eq!(cursor.index(), 0);
        assert!(cursor.backspace().is_none());

        #[cfg(feature = "length")]
        assert_eq!(cursor.index(), 0);
    }

    #[test]
    fn cursor_move() {
        macro_rules! test_cursor_move(
            ($FN:ident, $CURSOR:ident, $CURSOR_START:ident) => {
                fn $FN(
                    len: usize,
                    relative_moves: impl IntoIterator<Item = isize>,
                    invalid_relative_moves: impl IntoIterator<Item = isize>,
                ) {
                    #[allow(unused_mut)]
                    let mut list = List::from_iter(0..len);
                    let mut cursor = list.$CURSOR_START();
                    let mut index = 0;
                    let verify_cursor = |cursor: &$CURSOR<'_, _>, index| {
                        #[cfg(feature = "length")]
                        assert_eq!(cursor.index(), index);
                        if index == len {
                            assert!(cursor.current().is_none());
                        } else {
                            assert_eq!(cursor.current(), Some(&index));
                        }
                    };
                    verify_cursor(&cursor, index);
                    for mv in relative_moves {
                        match mv.cmp(&0) {
                            Ordering::Equal => {
                                assert!(cursor.seek_forward(0).is_ok());
                                verify_cursor(&cursor, index);
                                assert!(cursor.seek_backward(0).is_ok());
                            }
                            Ordering::Less => assert!(cursor.seek_backward(-mv as usize).is_ok()),
                            Ordering::Greater => assert!(cursor.seek_forward(mv as usize).is_ok()),
                        }
                        index = (index as isize + mv) as usize;
                        verify_cursor(&cursor, index);
                        assert!(cursor.try_seek_to(len + 1).is_err());
                        verify_cursor(&cursor, index);
                    }
                    for mv in invalid_relative_moves {
                        match mv.cmp(&0) {
                            Ordering::Equal => {
                                assert!(cursor.seek_forward(0).is_ok());
                                verify_cursor(&cursor, index);
                                assert!(cursor.seek_backward(0).is_ok());
                                verify_cursor(&cursor, index);
                            }
                            Ordering::Less => assert_eq!(cursor.seek_backward(-mv as usize), Err(index)),
                            Ordering::Greater => {
                                assert_eq!(cursor.seek_forward(mv as usize), Err(len - index))
                            }
                        }
                        index = (index as isize + mv).clamp(0, len as isize) as usize;
                        verify_cursor(&cursor, index);
                        assert!(cursor.try_seek_to(len + 1).is_err());
                        verify_cursor(&cursor, index);
                    }
                }
            }
        );
        test_cursor_move!(test_cursor_move, Cursor, cursor_start);
        test_cursor_move!(test_cursor_mut_move, CursorMut, cursor_start_mut);

        fn test_case(
            len: usize,
            relative_moves: impl IntoIterator<Item = isize> + Clone,
            invalid_relative_moves: impl IntoIterator<Item = isize> + Clone,
        ) {
            test_cursor_move(len, relative_moves.clone(), invalid_relative_moves.clone());
            test_cursor_mut_move(len, relative_moves, invalid_relative_moves);
        }

        test_case(10, [5, -3, 8, -10, 0, 5, 5], [10, 1, 0, -11, -5, 0]);
        test_case(3, [1, 2, -1, -1, 0, -1, 3, -3], [10, -20, 0, -4, 5, 0]);
        test_case(1, [1, -1, 1, 0, -1, 1, 0, -1], [3, -5, 0, -1, 2, 0]);
        test_case(0, [0, 0], [2, -3, 0, -1, 1, 0]);
    }

    #[test]
    fn cursor_iter() {
        macro_rules! test_cursor_iter(
            ($FN:ident, $CURSOR_START:ident, $INTO_CURSOR:ident) => {
                fn $FN(len: usize, mid: usize) {
                    #[allow(unused_mut)]
                    let mut list = List::from_iter(0..len);
                    let mut cursor_iter = list.$CURSOR_START().into_iter();
                    for _ in 0..3 {
                        for i in 0..len {
                            assert_eq!(cursor_iter.next().copied(), Some(i));
                        }
                        assert_eq!(cursor_iter.next().copied(), None);
                    }
                    for i in 0..mid {
                        assert_eq!(cursor_iter.next().copied(), Some(i));
                    }

                    let cursor = cursor_iter.$INTO_CURSOR();
                    if mid == len {
                        assert_eq!(cursor.current(), None);
                    } else {
                        assert_eq!(cursor.current(), Some(&mid));
                    }
                    let cursor_iter = cursor.into_iter();

                    let mut cursor_iter = cursor_iter.rev();
                    for i in (0..mid).rev() {
                        assert_eq!(cursor_iter.next().copied(), Some(i));
                    }
                    assert_eq!(cursor_iter.next().copied(), None);
                    for _ in 0..3 {
                        for i in (0..len).rev() {
                            assert_eq!(cursor_iter.next().copied(), Some(i));
                        }
                        assert_eq!(cursor_iter.next().copied(), None);
                    }

                    for i in (mid..len).rev() {
                        assert_eq!(cursor_iter.next().copied(), Some(i));
                    }

                    let cursor = cursor_iter.$INTO_CURSOR();
                    if mid == len {
                        assert_eq!(cursor.current(), None);
                    } else {
                        assert_eq!(cursor.current(), Some(&mid));
                    }

                    let mut cursor_iter = cursor.into_iter();
                    for i in mid..len {
                        assert_eq!(cursor_iter.next().copied(), Some(i));
                    }
                }
            };
        );
        test_cursor_iter!(test_cursor_iter, cursor_start, into_cursor);
        test_cursor_iter!(test_cursor_iter_mut, cursor_start_mut, into_cursor_mut);

        fn test_case(len: usize, mid: usize) {
            test_cursor_iter(len, mid);
            test_cursor_iter_mut(len, mid);
        }

        test_case(10, 10);
        test_case(10, 5);
        test_case(10, 1);
        test_case(10, 0);
        test_case(2, 2);
        test_case(2, 1);
        test_case(2, 0);
        test_case(1, 1);
        test_case(1, 0);
        test_case(0, 0);
    }
}
