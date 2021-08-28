use crate::list::{List, Node};
use std::ptr::NonNull;

#[derive(Clone)]
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

            unsafe fn seek_forward_fast(&mut self, steps: usize) {
                #[cfg(feature = "length")]
                {
                    self.index += steps;
                }
                (0..steps).for_each(|_| self.current = self.next_node());
            }

            unsafe fn seek_backward_fast(&mut self, steps: usize) {
                #[cfg(feature = "length")]
                {
                    self.index -= steps;
                }
                (0..steps).for_each(|_| self.current = self.prev_node());
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

            pub fn move_next(&mut self) -> Result<(), &'static str> {
                if !self.is_ghost_node(self.current) {
                    self.move_next_cyclic();
                    return Ok(());
                }
                Err("`move_next` across ghost boundary")
            }

            pub fn move_prev(&mut self) -> Result<(), &'static str> {
                if !self.is_ghost_node(self.prev_node()) {
                    self.move_prev_cyclic();
                    return Ok(());
                }
                Err("`move_prev` across ghost boundary")
            }

            // TODO: use `Iterator::advance_by` once stabled
            pub fn seek_forward(&mut self, steps: usize) -> Result<(), usize> {
                (0..steps).try_for_each(|i| self.move_next().map_err(|_| i))
            }

            pub fn seek_backward(&mut self, steps: usize) -> Result<(), usize> {
                (0..steps).try_for_each(|i| self.move_prev().map_err(|_| i))
            }

            pub fn seek_to(&mut self, target: usize) -> Result<(), usize> {
                #[cfg(not(feature = "length"))]
                {
                    let current = self.current;
                    self.set_start();
                    if self.seek_forward(target).is_err() {
                        self.current = current;
                    }
                }
                #[cfg(feature = "length")]
                {
                    if target == self.index {
                        return Ok(());
                    }
                    let len = self.list.len();
                    match target {
                        target if target > len => return Err(target - len),
                        0 => self.set_start(),
                        target if target == len => self.set_end(),
                        _ => unsafe {
                            // current=c, target=t, ghost=#
                            if target > self.index {
                                // target is at the right side of current: [   c----->t   #]
                                if target - self.index <= len - target {
                                    // target is near the right side of current: [    c-->t     #]
                                    self.seek_forward_fast(target - self.index);
                                } else {
                                    // target is far from the right side of current: [ c     t<--#]
                                    self.set_end();
                                    self.seek_backward_fast(len - target);
                                }
                            } else {
                                // target is at the left side of current: [   t<-----c   #]
                                if self.index - target <= target {
                                    // target is near the left side of current: [    t<--c     #]
                                    self.seek_backward_fast(self.index - target);
                                } else {
                                    // target is far from the left side of current: [-->t      c #]
                                    self.set_start();
                                    self.seek_forward_fast(target);
                                }
                            }
                        },
                    }
                }
                Ok(())
            }

            pub fn set_start(&mut self) {
                #[cfg(feature = "length")]
                {
                    self.index = 0;
                }
                self.current = self.list.front_node();
            }

            pub fn set_end(&mut self) {
                #[cfg(feature = "length")]
                {
                    self.index = self.list.len();
                }
                self.current = self.list.ghost_node();
            }

            pub fn current(&self) -> Option<&'a T> {
                if self.is_ghost_node(self.current) {
                    return None;
                }
                // TODO: SAFETY
                unsafe { Some(&self.current.as_ref().element) }
            }

            pub fn previous(&self) -> Option<&'a T> {
                if self.is_ghost_node(self.prev_node()) {
                    return None;
                }
                // TODO: SAFETY
                Some(unsafe { &self.prev_node().as_ref().element })
            }

            pub fn view(&self) -> &List<T> {
                self.list
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

    unsafe fn insert_before(&mut self, next: NonNull<Node<T>>, item: T) -> NonNull<Node<T>> {
        let node = Node::new_detached(item);
        self.list.attach_node(next.as_ref().prev, next, node);
        node
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub fn previous_mut(&mut self) -> Option<&'a mut T> {
        if self.is_ghost_node(self.prev_node()) {
            return None;
        }
        // TODO: SAFETY
        Some(unsafe { &mut self.prev_node().as_mut().element })
    }

    pub fn as_cursor(&self) -> Cursor<'_, T> {
        Cursor::new(
            self.list,
            self.current,
            #[cfg(feature = "length")]
            self.index,
        )
    }

    pub fn into_cursor(self) -> Cursor<'a, T> {
        Cursor::new(
            self.list,
            self.current,
            #[cfg(feature = "length")]
            self.index,
        )
    }
}

impl<'a, T: 'a> CursorMut<'a, T> {
    pub fn current_mut(&mut self) -> Option<&'a mut T> {
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

    pub fn append(&mut self, item: T) {
        unsafe {
            self.insert_before(self.current, item);
        }
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

    pub fn split(&mut self) -> Option<List<T>> {
        if self.is_ghost_node(self.current) {
            return None;
        }
        #[cfg(feature = "length")]
        let len = self.list.len - self.index;
        // TODO: SAFETY
        unsafe {
            Some(List::from_detached(self.list.detach_nodes(
                self.current,
                self.list.back_node(),
                #[cfg(feature = "length")]
                len,
            )))
        }
    }

    pub fn splice(&mut self, other: List<T>) {
        if let Some(detached) = other.into_detached() {
            // TODO: SAFETY
            unsafe {
                self.list
                    .attach_nodes(self.prev_node(), self.current, detached);
            }
        }
    }

    pub fn splice_with<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().for_each(|item| self.insert(item));
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
