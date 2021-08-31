use crate::list::{List, Node};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ptr::NonNull;

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
    unsafe fn mid_of_range(
        &self,
        mut start: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
    ) -> NonNull<Node<T>> {
        let mut mid = start;
        while start != end {
            start = start.as_ref().next;
            if start != end {
                start = start.as_ref().next;
                mid = mid.as_ref().next;
            }
        }
        mid
    }

    unsafe fn sort_range<F>(
        &mut self,
        mut start: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        compare: &mut F,
    ) -> NonNull<Node<T>>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let mut mid = self.mid_of_range(start, end);

        if start != mid && start.as_ref().next != mid {
            start = self.sort_range(start, mid, compare);
        }
        if mid != end && mid.as_ref().next != end {
            mid = self.sort_range(mid, end, compare);
        }

        if start != mid && mid != end {
            self.merge_range(&mut start, mid, end, compare);
        }
        start
    }

    unsafe fn merge_range<F>(
        &mut self,
        start: &mut NonNull<Node<T>>,
        mid: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        compare: &mut F,
    ) where
        F: FnMut(&T, &T) -> Ordering,
    {
        let (before_start, before_mid, before_end) =
            (start.as_ref().prev, mid.as_ref().prev, end.as_ref().prev);
        let (mut first, mut second, mut result) = (*start, mid, before_start);
        while first != mid && second != end {
            let this = match compare(&first.as_ref().element, &second.as_ref().element) {
                std::cmp::Ordering::Greater => &mut second,
                _ => &mut first,
            };
            self.connect(result, *this);
            result = std::mem::replace(this, this.as_ref().next);
        }
        if let Some((rem_front, rem_back)) = match (first != mid, second != end) {
            (true, false) => Some((first, before_mid)),
            (false, true) => Some((second, before_end)),
            _ => None,
        } {
            self.connect(result, rem_front);
            result = rem_back;
        };
        self.connect(result, end);
        *start = before_start.as_ref().next;
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

    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    /// let mut list = List::from_iter([5, 2, 4, 3, 1]);
    ///
    /// list.sort();
    ///
    /// assert_eq!(Vec::from_iter(list), Vec::from_iter([1, 2, 3, 4, 5]));
    /// List::<i32>::new().sort();
    /// ```
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.sort_by(T::cmp);
    }

    /// TODO
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        if self.is_empty() || self.front_node() == self.back_node() {
            return;
        }
        unsafe {
            self.sort_range(self.front_node(), self.ghost_node(), &mut compare);
        }
    }

    /// TODO
    pub fn sort_by_key<K, F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.sort_by(|lhs, rhs| f(lhs).cmp(&f(rhs)));
    }
}
