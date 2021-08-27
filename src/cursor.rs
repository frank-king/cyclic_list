use crate::list::{List, Node};
use std::ptr::NonNull;

pub struct Cursor<'a, T: 'a> {
    index: usize,
    current: NonNull<Node<T>>,
    pub(crate) list: &'a List<T>,
}

pub struct CursorMut<'a, T: 'a> {
    index: usize,
    current: NonNull<Node<T>>,
    pub(crate) list: &'a mut List<T>,
}

macro_rules! impl_cursor {
    ($CURSOR:ident) => {
        impl<'a, T: 'a> $CURSOR<'a, T> {
            fn is_ghost(&self, ptr: NonNull<Node<T>>) -> bool {
                ptr == self.list.ghost()
            }
            fn next(&self) -> NonNull<Node<T>> {
                // TODO: SAFETY
                unsafe { self.current.as_ref().next }
            }
            fn prev(&self) -> NonNull<Node<T>> {
                // TODO: SAFETY
                unsafe { self.current.as_ref().prev }
            }
        }

        impl<'a, T: 'a> $CURSOR<'a, T> {
            pub fn index(&self) -> usize {
                self.index
            }

            pub fn move_next(&mut self) -> bool {
                if !self.is_ghost(self.current) {
                    // TODO: index
                    self.current = self.next();
                    return true;
                }
                false
            }

            pub fn move_prev(&mut self) -> bool {
                if !self.is_ghost(self.current) {
                    // TODO: index
                    self.current = self.prev();
                    return true;
                }
                false
            }

            pub fn seek_forward(&mut self, step: usize) -> Result<(), usize> {
                for i in 0..step {
                    if !self.move_next() {
                        return Err(i);
                    }
                }
                Ok(())
            }

            pub fn seek_backward(&mut self, step: usize) -> Result<(), usize> {
                for i in 0..step {
                    if !self.move_prev() {
                        return Err(i);
                    }
                }
                Ok(())
            }

            pub fn current(&self) -> Option<&'a T> {
                if self.is_ghost(self.current) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.current.as_ref().element) }
            }

            pub fn peek_next(&self) -> Option<&'a T> {
                if self.is_ghost(self.next()) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.next().as_ref().element) }
            }

            pub fn peek_prev(&self) -> Option<&'a T> {
                if self.is_ghost(self.prev()) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.prev().as_ref().element) }
            }
        }
    };
}

impl_cursor!(CursorMut);
impl_cursor!(Cursor);

impl<'a, T: 'a> Cursor<'a, T> {
    pub(crate) fn new(index: usize, list: &'a List<T>, current: NonNull<Node<T>>) -> Self {
        Self {
            index,
            current,
            list,
        }
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub(crate) fn new(index: usize, list: &'a mut List<T>, current: NonNull<Node<T>>) -> Self {
        Self {
            index,
            current,
            list,
        }
    }
    fn next_mut(&mut self) -> &mut Node<T> {
        // TODO: SAFETY
        unsafe { self.current.as_mut().next.as_mut() }
    }
    fn prev_mut(&mut self) -> &mut Node<T> {
        // TODO: SAFETY
        unsafe { self.current.as_mut().prev.as_mut() }
    }
    fn insert(&mut self, mut prev: NonNull<Node<T>>, mut next: NonNull<Node<T>>, item: T) {
        let node = Node::new(next, prev, item);
        // TODO: SAFETY
        unsafe {
            self.list.splice_nodes(
                prev,
                next,
                node,
                node,
                #[cfg(feature = "length")]
                1,
            );
        }
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub fn current_mut(&mut self) -> Option<&'a mut T> {
        if self.is_ghost(self.current) {
            return None;
        }
        // TODO: SAFETY
        unsafe { Some(&mut self.current.as_mut().element) }
    }
    pub fn insert_front(&mut self, item: T) {
        self.insert(self.list.ghost(), self.list.ghost_next(), item)
    }

    pub fn insert_back(&mut self, item: T) {
        self.insert(self.list.ghost_prev(), self.list.ghost(), item)
    }

    pub fn insert_after(&mut self, item: T) {
        self.insert(self.current, self.next(), item);
        self.move_next();
    }

    pub fn insert_before(&mut self, item: T) {
        self.insert(self.prev(), self.current, item);
    }

    pub fn remove_current(&mut self) -> Option<T> {
        if self.is_ghost(self.current) {
            return None;
        }
        #[cfg(feature = "length")]
        {
            self.list.len -= 1;
        }
        self.next_mut().prev = self.prev();
        self.prev_mut().next = self.next();
        // TODO: SAFETY
        let node = unsafe { Box::from_raw(self.current.as_ptr()) };
        self.current = self.next();
        // TODO: index
        Some(Node::into_element(node))
    }

    pub fn as_cursor(&self) -> Cursor<'_, T> {
        Cursor::new(self.index, self.list, self.current)
    }

    pub fn split_after(&mut self) -> Option<List<T>> {
        if self.is_ghost(self.current) || self.is_ghost(self.next()) {
            return None;
        }
        let split_start = self.next();
        let split_end = self.list.ghost_prev();
        // TODO: SAFETY
        unsafe {
            self.current.as_mut().next = self.list.ghost();
            Some(List::from_splice(
                split_start,
                split_end,
                #[cfg(feature = "length")]
                {
                    self.list.len - self.index
                },
            ))
        }
    }

    pub fn split_before(&mut self) -> Option<List<T>> {
        if self.is_ghost(self.current) || self.is_ghost(self.prev()) {
            return None;
        }
        let split_start = self.prev();
        let split_end = self.list.ghost_next();
        // TODO: SAFETY
        unsafe {
            self.current.as_mut().prev = self.list.ghost();
            Some(List::from_splice(
                split_start,
                split_end,
                #[cfg(feature = "length")]
                self.index,
            ))
        }
    }
}
