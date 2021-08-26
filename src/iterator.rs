use crate::cursor::{Cursor, CursorMut};

pub struct Iter<'a, T: 'a> {
    cursor: Cursor<'a, T>,
}

impl<'a, T: 'a> Iter<'a, T> {
    pub(crate) fn new(cursor: Cursor<'a, T>) -> Self {
        Self { cursor }
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.cursor.current();
        self.cursor.move_next();
        current
    }

    #[cfg(feature = "length")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.cursor.len();
        (len, Some(len))
    }
}

#[cfg(feature = "length")]
impl<'a, T: 'a> ExactSizeIterator for Iter<'a, T> {}

pub struct IterMut<'a, T: 'a> {
    cursor: CursorMut<'a, T>,
}

impl<'a, T: 'a> IterMut<'a, T> {
    pub(crate) fn new(cursor: CursorMut<'a, T>) -> Self {
        Self { cursor }
    }
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.cursor.current_mut();
        self.cursor.move_next();
        current
    }
}
