use crate::list::cursor::{
    Cursor, CursorBackIter, CursorBackIterMut, CursorIter, CursorIterMut, CursorMut,
};
use crate::list::{List, Node};
use std::iter::{FromIterator, FusedIterator};
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct Iter<'a, T: 'a> {
    start: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    len: usize,
    _marker: PhantomData<&'a List<T>>,
}

impl<'a, T: 'a> Iter<'a, T> {
    pub(crate) fn new(list: &'a List<T>) -> Self {
        let start = list.front_node();
        let end = list.ghost_node();
        let _marker = PhantomData;
        #[cfg(feature = "length")]
        let len = list.len();
        eprintln!("len: {}", len);
        Self {
            start,
            end,
            #[cfg(feature = "length")]
            len,
            _marker,
        }
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // TODO: SAFETY
        let current = unsafe { self.start.as_ref() };
        self.start = current.next;
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        Some(&current.element)
    }

    #[cfg(feature = "length")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next_back()
    }
}

impl<'a, T: 'a> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // TODO: SAFETY
        self.end = unsafe { self.end.as_ref().prev };
        // TODO: SAFETY
        let current = unsafe { self.end.as_ref() };
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        Some(&current.element)
    }
}

#[cfg(feature = "length")]
impl<'a, T: 'a> ExactSizeIterator for Iter<'a, T> {}

impl<'a, T: 'a> FusedIterator for Iter<'a, T> {}

pub struct IterMut<'a, T: 'a> {
    start: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    len: usize,
    _marker: PhantomData<&'a mut List<T>>,
}

impl<'a, T: 'a> IterMut<'a, T> {
    pub(crate) fn new(list: &'a mut List<T>) -> Self {
        let start = list.front_node();
        let end = list.ghost_node();
        let _marker = PhantomData;
        #[cfg(feature = "length")]
        let len = list.len();
        Self {
            start,
            end,
            #[cfg(feature = "length")]
            len,
            _marker,
        }
    }
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // TODO: SAFETY
        let current = unsafe { self.start.as_mut() };
        self.start = current.next;
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        Some(&mut current.element)
    }

    #[cfg(feature = "length")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next_back()
    }
}

#[cfg(feature = "length")]
impl<'a, T: 'a> ExactSizeIterator for IterMut<'a, T> {}

impl<'a, T: 'a> FusedIterator for IterMut<'a, T> {}

impl<'a, T: 'a> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // TODO: SAFETY
        self.end = unsafe { self.end.as_ref().prev };
        // TODO: SAFETY
        let current = unsafe { self.end.as_mut() };
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        Some(&mut current.element)
    }
}

pub struct IntoIter<T> {
    list: List<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front()
    }

    #[cfg(feature = "length")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.list.len;
        (len, Some(len))
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next_back()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.list.pop_back()
    }
}

#[cfg(feature = "length")]
impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> FusedIterator for IntoIter<T> {}

impl<T> IntoIterator for List<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { list: self }
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut List<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> FromIterator<T> for List<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut list = List::new();
        list.extend(iter);
        list
    }
}

impl<T> Extend<T> for List<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|item| self.push_back(item));
    }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for List<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().copied())
    }
}

impl<'a, T: 'a> Iterator for CursorIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.cursor.current();
        self.cursor.move_next_cyclic();
        current
    }
}

impl<'a, T: 'a> Iterator for CursorIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.cursor.current_mut();
        self.cursor.move_next_cyclic();
        current
    }
}

impl<'a, T: 'a> Iterator for CursorBackIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.move_prev_cyclic();
        self.cursor.current()
    }
}

impl<'a, T: 'a> Iterator for CursorBackIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.move_prev_cyclic();
        self.cursor.current_mut()
    }
}

impl<'a, T: 'a> IntoIterator for Cursor<'a, T> {
    type Item = &'a T;
    type IntoIter = CursorIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        CursorIter { cursor: self }
    }
}

impl<'a, T: 'a> IntoIterator for CursorMut<'a, T> {
    type Item = &'a mut T;
    type IntoIter = CursorIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        CursorIterMut { cursor: self }
    }
}

unsafe impl<T: Sync> Send for Iter<'_, T> {}

unsafe impl<T: Sync> Sync for Iter<'_, T> {}

unsafe impl<T: Send> Send for IterMut<'_, T> {}

unsafe impl<T: Sync> Sync for IterMut<'_, T> {}
