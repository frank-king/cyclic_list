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
        let iter_other = other.iter();
        let mut cursor_mut = self.cursor_start_mut();
        for elem_other in iter_other {
            // FIXME incorrect cursor moves
            match cursor_mut.current_mut() {
                None => cursor_mut.insert(elem_other.clone()),
                Some(elem) => elem.clone_from(elem_other),
            }
            cursor_mut.move_next_cyclic();
        }
        cursor_mut.split();
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

impl<T> List<T> {
    /// Returns `true` if the `List` contains an element equal to the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    ///
    /// let mut list = List::new();
    ///
    /// list.push_back(0);
    /// list.push_back(1);
    /// list.push_back(2);
    ///
    /// assert_eq!(list.contains(&0), true);
    /// assert_eq!(list.contains(&10), false);
    /// ```
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq<T>,
    {
        self.iter().any(|e| e == x)
    }

    /// TODO
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        unimplemented!()
    }

    /// TODO
    pub fn sort_by<F>(&mut self, _compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        unimplemented!()
    }

    /// TODO
    pub fn sort_by_key<K, F>(&mut self, _f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        unimplemented!()
    }
}
