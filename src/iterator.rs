use crate::list::{List, Node};
use std::iter::{FromIterator, FusedIterator};
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct Iter<'a, T: 'a> {
    begin: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    len: usize,
    _marker: PhantomData<&'a List<T>>,
}

impl<'a, T: 'a> Iter<'a, T> {
    pub(crate) fn new(list: &'a List<T>) -> Self {
        let begin = list.ghost_next();
        let end = list.ghost();
        let _marker = PhantomData;
        #[cfg(feature = "length")]
        let len = list.len();
        Self {
            begin,
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
        if self.begin == self.end {
            return None;
        }
        // TODO: SAFETY
        let current = unsafe { self.begin.as_ref() };
        self.begin = current.next;
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
        if self.begin == self.end {
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
    begin: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    #[cfg(feature = "length")]
    len: usize,
    _marker: PhantomData<&'a mut List<T>>,
}

impl<'a, T: 'a> IterMut<'a, T> {
    pub(crate) fn new(list: &'a mut List<T>) -> Self {
        let begin = list.ghost_next();
        let end = list.ghost();
        let _marker = PhantomData;
        #[cfg(feature = "length")]
        let len = list.len();
        Self {
            begin,
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
        if self.begin == self.end {
            return None;
        }
        // TODO: SAFETY
        let current = unsafe { self.begin.as_mut() };
        self.begin = current.next;
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
        if self.begin == self.end {
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
