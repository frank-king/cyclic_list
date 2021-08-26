use crate::list::{List, Node};
use std::ops::Range;
use std::ptr::NonNull;

pub struct Cursor<'a, T: 'a> {
    #[cfg(feature = "length")]
    index: usize,
    current: NonNull<Node<T>>,
    list: &'a List<T>,
}

pub struct CursorMut<'a, T: 'a> {
    #[cfg(feature = "length")]
    index: usize,
    current: NonNull<Node<T>>,
    list: &'a mut List<T>,
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
            #[cfg(feature = "length")]
            pub fn len(&self) -> usize {
                self.list.len()
            }

            #[cfg(feature = "length")]
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
    pub(crate) fn new(
        #[cfg(feature = "length")] index: usize,
        list: &'a List<T>,
        current: NonNull<Node<T>>,
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
        #[cfg(feature = "length")] index: usize,
        list: &'a mut List<T>,
        current: NonNull<Node<T>>,
    ) -> Self {
        Self {
            #[cfg(feature = "length")]
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
        let node = NonNull::from(Box::leak(node));
        // TODO: SAFETY
        unsafe {
            next.as_mut().prev = node;
            prev.as_mut().next = node;
        }
        if self.is_ghost(self.current) {
            self.current = self.next();
        }
        #[cfg(feature = "length")]
        {
            self.list.len += 1;
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
}

pub struct CursorRange<'a, T: 'a> {
    #[cfg(feature = "length")]
    range: Range<usize>,
    begin: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    list: &'a List<T>,
}

impl<'a, T: 'a> CursorRange<'a, T> {
    fn new(
        #[cfg(feature = "length")] range: Range<usize>,
        begin: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        list: &'a List<T>,
    ) -> Self {
        Self {
            #[cfg(feature = "length")]
            range,
            begin,
            end,
            list,
        }
    }
}

pub struct CursorRangeMut<'a, T: 'a> {
    #[cfg(feature = "length")]
    range: Range<usize>,
    begin: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    list: &'a mut List<T>,
}

impl<'a, T: 'a> CursorRangeMut<'a, T> {
    fn new(
        #[cfg(feature = "length")] range: Range<usize>,
        begin: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        list: &'a mut List<T>,
    ) -> Self {
        Self {
            #[cfg(feature = "length")]
            range,
            begin,
            end,
            list,
        }
    }
}

macro_rules! impl_cursor_range {
    ($CURSOR_RANGE:ident, $CURSOR:ident) => {
        impl<'a, T: 'a> $CURSOR_RANGE<'a, T> {
            pub fn begin<'b>(&'b mut self) -> $CURSOR<'b, T>
            where
                'a: 'b,
            {
                $CURSOR::new(
                    #[cfg(feature = "length")]
                    self.range.start,
                    self.list,
                    self.begin,
                )
            }

            pub fn end<'b>(&'b mut self) -> $CURSOR<'b, T>
            where
                'a: 'b,
            {
                $CURSOR::new(
                    #[cfg(feature = "length")]
                    self.range.end,
                    self.list,
                    self.end,
                )
            }

            pub fn into_begin(self) -> $CURSOR<'a, T> {
                $CURSOR::new(
                    #[cfg(feature = "length")]
                    self.range.start,
                    self.list,
                    self.end,
                )
            }

            pub fn into_end(self) -> $CURSOR<'a, T> {
                $CURSOR::new(
                    #[cfg(feature = "length")]
                    self.range.end,
                    self.list,
                    self.end,
                )
            }
        }
    };
}

impl_cursor_range!(CursorRange, Cursor);
impl_cursor_range!(CursorRangeMut, CursorMut);

impl<'a, T: 'a> CursorRangeMut<'a, T> {
    pub fn insert_before(&mut self, item: T) {
        self.begin().insert_before(item)
    }

    pub fn insert_after(&mut self, item: T) {
        self.end().insert_before(item)
    }
}
