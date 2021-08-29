use crate::list::cursor::{
    Cursor, CursorBackIter, CursorBackIterMut, CursorIter, CursorIterMut, CursorMut,
};
use crate::list::{List, Node};
use std::fmt;
use std::iter::{FromIterator, FusedIterator};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// An iterator over the elements of a `List`.
///
/// It uses a pair of nodes `start..end` to represent a half-open subrange
/// of the list, where `start` is inclusive and `end` is not.
///
/// Though the `Iter` does not hold a reference from the list,
/// it actually *borrows* (immutably) from the list, so a phantom
/// marker of `&'a List<T>` is added to protect the list from being
/// write.
///
/// # Examples
///
/// ```compile_fail
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let mut list = List::from_iter([1, 2, 3]);
/// let mut iter = list.iter();
///
/// // Won't compile, because list is already borrowed immutably.
/// list.push_back(4);
/// println!("{:?}", iter.next());
/// ```
#[derive(Clone)]
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
        Self {
            start,
            end,
            #[cfg(feature = "length")]
            len,
            _marker,
        }
    }
}

impl<'a, T: fmt::Debug + 'a> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("Iter");
        // SAFETY: `start..end` is always a valid range of a list,
        // and it is not empty here, so it is safe.
        let mut ptr = self.start;
        while ptr != self.end {
            let current = unsafe { ptr.as_ref() };
            f.field(&current.element);
            ptr = current.next;
        }
        f.finish()
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    /// Return `*start` and reset the iterating range to `(start.next)..end`,
    /// or return `None` if `start..end` is already empty.
    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // SAFETY: `start..end` is always a valid range of a list,
        // and it is not empty here, so it is safe.
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
    /// Reset the iterating range to `start..(end.prev)` and return `*end`,
    /// or return `None` if `start..end` is already empty.
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // SAFETY: `start..end` is always a valid range of a list,
        // and it is not empty here, so it is safe.
        self.end = unsafe { self.end.as_ref().prev };
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

/// A mutable iterator over the elements of a `List`.
///
/// `start..end` denotes a subrange of the list.
///
/// Though the `IterMut` does not hold a reference from the list,
/// it actually *borrows* (mutably) from the list, so a phantom
/// marker of `&'a mut List<T>` is added to protect the list from
/// begin read.
///
/// # Examples
///
/// `List` is not readable after an `IterMut` is created.
/// ```compile_fail
/// use cyclic_list::List;
/// use std::iter::FromIterator;
///
/// let mut list = List::from_iter([1, 2, 3]);
/// let mut iter = list.iter_mut();
/// println!("{:?}", list.back());
/// println!("{:?}", iter.next());
/// ```
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

impl<'a, T: fmt::Debug + 'a> fmt::Debug for IterMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("IterMut");
        // SAFETY: `start..end` is always a valid range of a list,
        // and it is not empty here, so it is safe.
        let mut ptr = self.start;
        while ptr != self.end {
            let current = unsafe { ptr.as_ref() };
            f.field(&current.element);
            ptr = current.next;
        }
        f.finish()
    }
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    /// Return `*start` and reset the iterating range to `(start.next)..end`,
    /// or return `None` if `start..end` is already empty.
    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // SAFETY: `start..end` is always a valid range of a list,
        // and it is not empty here, so it is safe.
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
    /// Reset the iterating range to `start..(end.prev)` and return `*end`,
    /// or return `None` if `start..end` is already empty.
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }
        // SAFETY: `start..end` is always a valid range of a list,
        // and it is not empty here, so it is safe.
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

/// An owning iterator over the elements of a `List`.
///
/// This `struct` is created by the [`into_iter`] method on [`List`]
/// (provided by the `IntoIterator` trait). See its documentation for more.
///
/// [`into_iter`]: List::into_iter
pub struct IntoIter<T> {
    list: List<T>,
}

impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoIter")
            .field("list", &self.list)
            .finish()
    }
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

/// Convert the cursor to an iterator, which is cyclic and not fused.
impl<'a, T: 'a> IntoIterator for Cursor<'a, T> {
    type Item = &'a T;
    type IntoIter = CursorIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        CursorIter { cursor: self }
    }
}

/// Convert the cursor to an mutable iterator, which is cyclic
/// and not fused.
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

#[cfg(test)]
mod tests {
    use crate::List;
    use std::fmt::Debug;
    use std::iter::FromIterator;

    #[test]
    fn test_iter() {
        macro_rules! test_iter {
            ($FN:ident, $ITER:ident $(, $REV:ident)?) => {
                fn $FN<T, I>(input: I, mid: usize)
                where
                    T: Eq + Debug + Clone,
                    I: IntoIterator<Item = T>,
                {
                    #[allow(unused_mut)]
                    let mut vec = Vec::from_iter(input);
                    #[allow(unused_mut)]
                    let mut list = List::from_iter(vec.clone());
                    let len = vec.len();
                    let mut iter = list.$ITER() $( .$REV() )?;
                    for (i, item) in vec.$ITER() $( .$REV() )?.enumerate() {
                        assert_eq!(iter.next(), Some(item));
                        #[cfg(feature = "length")]
                        assert_eq!(iter.len(), len - i - 1);
                    }
                    assert_eq!(iter.next(), None);
                    assert_eq!(iter.next(), None);
                    assert_eq!(iter.next_back(), None);
                    #[cfg(feature = "length")]
                    assert_eq!(iter.len(), 0);

                    let mut iter = list.$ITER() $( .$REV() )?;
                    for (i, item) in vec.$ITER() $( .$REV() )? .take(mid).enumerate() {
                        assert_eq!(iter.next(), Some(item));
                        #[cfg(feature = "length")]
                        assert_eq!(iter.len(), len - i - 1);
                    }
                    let mut iter = iter.rev();
                    for (i, item) in vec.$ITER() $( .$REV() )? .skip(mid).rev().enumerate() {
                        assert_eq!(iter.next(), Some(item));
                        #[cfg(feature = "length")]
                        assert_eq!(iter.len(), len - mid - i - 1);
                    }
                    assert_eq!(iter.next(), None);
                    assert_eq!(iter.next(), None);
                    assert_eq!(iter.next_back(), None);
                    #[cfg(feature = "length")]
                    assert_eq!(iter.len(), 0);
                }
            };
        }
        test_iter!(test_iter, iter);
        test_iter!(test_iter_mut, iter_mut);
        test_iter!(test_back_iter, iter, rev);
        test_iter!(test_back_iter_mut, iter_mut, rev);

        fn test_case<T, I>(input: I, mid: usize)
        where
            T: Eq + Debug + Clone,
            I: IntoIterator<Item = T> + Clone,
        {
            test_iter(input.clone(), mid);
            test_iter_mut(input.clone(), mid);
            test_back_iter(input.clone(), mid);
            test_back_iter_mut(input.clone(), mid);
        }
        test_case(0..10, 10);
        test_case(0..10, 8);
        test_case(0..10, 5);
        test_case(0..10, 2);
        test_case(0..10, 0);
        test_case(0..2, 2);
        test_case(0..2, 1);
        test_case(0..2, 0);
        test_case(0..1, 1);
        test_case(0..1, 0);
        test_case(0..0, 0);
    }
}
