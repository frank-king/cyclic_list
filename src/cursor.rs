use crate::list::{DetachedNodes, List, Node};
use std::ptr::NonNull;

pub struct Cursor<'a, T: 'a> {
    #[cfg(feature = "length")]
    index: usize,
    pub(crate) current: NonNull<Node<T>>,
    pub(crate) list: &'a List<T>,
}

pub struct CursorMut<'a, T: 'a> {
    #[cfg(feature = "length")]
    index: usize,
    pub(crate) current: NonNull<Node<T>>,
    pub(crate) list: &'a mut List<T>,
}

macro_rules! impl_cursor {
    ($CURSOR:ident) => {
        impl<'a, T: 'a> $CURSOR<'a, T> {
            pub(crate) fn is_ghost_node(&self, ptr: NonNull<Node<T>>) -> bool {
                ptr == self.list.ghost_node()
            }
            pub(crate) fn next_node(&self) -> NonNull<Node<T>> {
                // TODO: SAFETY
                unsafe { self.current.as_ref().next }
            }
            pub(crate) fn prev_node(&self) -> NonNull<Node<T>> {
                // TODO: SAFETY
                unsafe { self.current.as_ref().prev }
            }
        }

        impl<'a, T: 'a> $CURSOR<'a, T> {
            #[cfg(feature = "length")]
            pub fn index(&self) -> usize {
                self.index
            }

            pub fn move_next_cyclic(&mut self) {
                #[cfg(feature = "length")]
                if self.is_ghost_node(self.current) {
                    self.index = 0;
                } else {
                    self.index += 1;
                }
                self.current = self.next_node();
            }

            pub fn move_prev_cyclic(&mut self) {
                #[cfg(feature = "length")]
                if self.is_ghost_node(self.prev_node()) {
                    self.index = self.list.len();
                } else {
                    self.index -= 1;
                }
                self.current = self.prev_node();
            }

            // TODO: use `Iterator::advance_by` once stabled
            pub fn seek_forward(&mut self, step: usize) -> Result<(), usize> {
                for i in 0..step {
                    if self.next().is_none() {
                        return Err(i);
                    }
                }
                Ok(())
            }

            pub fn seek_backward(&mut self, step: usize) -> Result<(), usize> {
                for i in 0..step {
                    if self.prev().is_none() {
                        return Err(i);
                    }
                }
                Ok(())
            }

            pub fn current(&self) -> Option<&'a T> {
                if self.is_ghost_node(self.current) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.current.as_ref().element) }
            }

            pub fn peek_next(&self) -> Option<&'a T> {
                if self.is_ghost_node(self.current) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.current.as_ref().element) }
            }

            pub fn peek_prev(&self) -> Option<&'a T> {
                if self.is_ghost_node(self.prev_node()) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.prev_node().as_ref().element) }
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
    fn next_node_mut(&mut self) -> &mut Node<T> {
        // TODO: SAFETY
        unsafe { self.current.as_mut().next.as_mut() }
    }
    fn prev_node_mut(&mut self) -> &mut Node<T> {
        // TODO: SAFETY
        unsafe { self.current.as_mut().prev.as_mut() }
    }
    unsafe fn insert_before(&mut self, mut next: NonNull<Node<T>>, item: T) -> NonNull<Node<T>> {
        let node = Node::new_detached(item);
        let detached = DetachedNodes::from_single(node);
        self.list.attach_nodes(next.as_ref().prev, next, detached);
        node
    }
}

impl<'a, T: 'a> Cursor<'a, T> {
    pub fn next(&mut self) -> Option<&'a T> {
        let current = self.current();
        self.move_next_cyclic();
        current
    }
    pub fn prev(&mut self) -> Option<&'a T> {
        self.move_prev_cyclic();
        self.current()
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub fn next(&mut self) -> Option<&'a mut T> {
        let current = self.peek_mut();
        self.move_next_cyclic();
        current
    }
    pub fn prev(&mut self) -> Option<&'a mut T> {
        self.move_prev_cyclic();
        self.peek_mut()
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub fn peek_mut(&mut self) -> Option<&'a mut T> {
        if self.is_ghost_node(self.current) {
            return None;
        }
        // TODO: SAFETY
        unsafe { Some(&mut self.current.as_mut().element) }
    }

    pub fn push_front(&mut self, item: T) {
        self.list.push_front(item);
        self.index += 1;
    }

    pub fn push_back(&mut self, item: T) {
        self.list.push_back(item)
    }

    pub fn insert(&mut self, item: T) {
        self.current = unsafe { self.insert_before(self.current, item) }
    }

    pub fn remove(&mut self) -> Option<T> {
        if self.is_ghost_node(self.current) {
            return None;
        }
        // TODO: SAFETY
        let node = unsafe { self.list.detach_node(self.current) };
        self.current = self.next_node();
        Some(Node::into_element(node))
    }

    pub fn backspace(&mut self) -> Option<T> {
        if self.is_ghost_node(self.prev_node()) {
            return None;
        }
        self.move_prev_cyclic();
        self.remove()
    }

    pub fn as_cursor(&self) -> Cursor<'_, T> {
        Cursor::new(self.list, self.current, self.index)
    }

    pub fn into_cursor(self) -> Cursor<'a, T> {
        Cursor::new(self.list, self.current, self.index)
    }

    pub fn split(&mut self) -> Option<List<T>> {
        if self.is_ghost_node(self.current) || self.is_ghost_node(self.next_node()) {
            return None;
        }
        #[cfg(feature = "length")]
        let len = self.list.len - self.index;
        // TODO: SAFETY
        unsafe {
            let detached = self.list.detach_nodes(
                self.next_node(),
                self.list.ghost_node_prev(),
                #[cfg(feature = "length")]
                len,
            );
            Some(List::from_detached(detached))
        }
    }

    pub fn splice(&mut self, other: List<T>) {
        if let Some(detached) = other.into_detached() {
            // TODO: SAFETY
            unsafe {
                self.list
                    .attach_nodes(self.list.ghost_node(), self.current, detached);
            }
        }
    }
}

pub struct CursorIter<'a, T: 'a> {
    pub(crate) cursor: Cursor<'a, T>,
}

pub struct CursorIterMut<'a, T: 'a> {
    pub(crate) cursor: CursorMut<'a, T>,
}

pub struct CursorBackIter<'a, T: 'a> {
    pub(crate) cursor: Cursor<'a, T>,
}

pub struct CursorBackIterMut<'a, T: 'a> {
    pub(crate) cursor: CursorMut<'a, T>,
}

impl<'a, T: 'a> CursorIter<'a, T> {
    pub fn into_cursor(self) -> Cursor<'a, T> {
        self.cursor
    }
    pub fn rev(self) -> CursorBackIter<'a, T> {
        CursorBackIter {
            cursor: self.cursor,
        }
    }
}

impl<'a, T: 'a> CursorIterMut<'a, T> {
    pub fn into_cursor_mut(self) -> CursorMut<'a, T> {
        self.cursor
    }
    pub fn rev(self) -> CursorBackIterMut<'a, T> {
        CursorBackIterMut {
            cursor: self.cursor,
        }
    }
}

impl<'a, T: 'a> CursorBackIter<'a, T> {
    pub fn into_cursor(self) -> Cursor<'a, T> {
        self.cursor
    }
    pub fn rev(self) -> CursorIter<'a, T> {
        CursorIter {
            cursor: self.cursor,
        }
    }
}

impl<'a, T: 'a> CursorBackIterMut<'a, T> {
    pub fn into_cursor_mut(self) -> CursorMut<'a, T> {
        self.cursor
    }
    pub fn rev(self) -> CursorIterMut<'a, T> {
        CursorIterMut {
            cursor: self.cursor,
        }
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
        cursor_iter.into_cursor_mut().into_cursor().into_iter()
    }
}
