use crate::cursor::{Cursor, CursorMut};
use crate::iterator::{Iter, IterMut};
use crate::list::List;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

impl<T: PartialEq> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other)
    }
}

impl<T: Eq> Eq for List<T> {}

impl<T: PartialOrd> PartialOrd for List<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other)
    }
}

impl<T: Ord> Ord for List<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other)
    }
}

impl<T: Clone> Clone for List<T> {
    fn clone(&self) -> Self {
        self.iter().cloned().collect()
    }

    fn clone_from(&mut self, other: &Self) {
        let mut iter_other = other.iter();
        let mut cursor_mut = self.cursor_front_mut();
        for elem_other in iter_other {
            // FIXME incorrect cursor moves
            match cursor_mut.current_mut() {
                None => cursor_mut.insert_before(elem_other.clone()),
                Some(elem) => elem.clone_from(elem_other),
            }
        }
        cursor_mut.split_after();
    }
}

impl<T: Hash> Hash for List<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut len = 0_usize;
        for elt in self {
            elt.hash(state);
            len += 1;
        }
        len.hash(state);
    }
}

unsafe impl<T: Send> Send for List<T> {}

unsafe impl<T: Sync> Sync for List<T> {}

unsafe impl<T: Sync> Send for Iter<'_, T> {}

unsafe impl<T: Sync> Sync for Iter<'_, T> {}

unsafe impl<T: Send> Send for IterMut<'_, T> {}

unsafe impl<T: Sync> Sync for IterMut<'_, T> {}

unsafe impl<T: Sync> Send for Cursor<'_, T> {}

unsafe impl<T: Sync> Sync for Cursor<'_, T> {}

unsafe impl<T: Send> Send for CursorMut<'_, T> {}

unsafe impl<T: Sync> Sync for CursorMut<'_, T> {}

impl<T> List<T> {
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq<T>,
    {
        self.iter().any(|e| e == x)
    }

    pub fn sort(&mut self)
    where
        T: Ord,
    {
        unimplemented!()
    }

    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        unimplemented!()
    }

    pub fn sort_by_key<K, F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        unimplemented!()
    }
}
