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

    fn merge_sort<F>(&mut self, mut less: F)
    where
        F: FnMut(&T, &T) -> bool,
    {
        if self.is_empty() || self.front_node() == self.back_node() {
            return;
        }
        unsafe {
            self.sort_range(self.front_node(), self.ghost_node(), &mut less);
        }
    }

    unsafe fn sort_range<F>(
        &mut self,
        mut start: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        less: &mut F,
    ) -> NonNull<Node<T>>
    where
        F: FnMut(&T, &T) -> bool,
    {
        let mut mid = self.mid_of_range(start, end);

        if start != mid && start.as_ref().next != mid {
            start = self.sort_range(start, mid, less);
        }
        if mid != end && mid.as_ref().next != end {
            mid = self.sort_range(mid, end, less);
        }

        if start != mid && mid != end {
            start = self.merge_range(start, mid, end, less);
        }
        start
    }

    unsafe fn merge_range<F>(
        &mut self,
        start: NonNull<Node<T>>,
        mid: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        less: &mut F,
    ) -> NonNull<Node<T>>
    where
        F: FnMut(&T, &T) -> bool,
    {
        let (before_start, before_mid, before_end) =
            (start.as_ref().prev, mid.as_ref().prev, end.as_ref().prev);
        let (mut first, mut second, mut result) = (start, mid, before_start);
        while first != mid && second != end {
            let this = if less(&first.as_ref().element, &second.as_ref().element) {
                &mut first
            } else {
                &mut second
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
        before_start.as_ref().next
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

    /// Sort the list.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n* * log(*n*)) time and *O*(1) memory.
    ///
    /// # Current Implementation
    ///
    /// The current algorithm is done by a naive merge sort. There is no extra
    /// temporary storage during merging.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    /// let mut list = List::from_iter([5, 2, 4, 3, 1]);
    ///
    /// list.sort();
    ///
    /// assert_eq!(list.into_vec(), vec![1, 2, 3, 4, 5]);
    /// ```
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.merge_sort(|a, b| a.lt(&b));
    }

    /// Sort the list with a comparator function.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// The comparator function must define a total ordering for the
    /// elements in the list. If the ordering is not total, the order
    /// of the elements is unspecified. An order is a total order if
    /// it is (for all `a`, `b` and `c`):
    /// - total and antisymmetric: exactly one of `a < b`, `a == b`
    ///   or `a > b` is true, and
    /// - transitive, `a < b` and `b < c` implies `a < c`. The same
    /// must hold for both `==` and `>`.
    ///
    /// For example, while [`f64`] doesn’t implement [`Ord`] because
    /// `NaN != NaN`, we can use `partial_cmp` as our sort function
    /// when we know the slice doesn’t contain a `NaN`.
    /// ```
    /// use cyclic_list::List;
    /// let mut floats = List::from([5f64, 4.0, 1.0, 3.0, 2.0]);
    /// floats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    /// assert_eq!(floats.into_vec(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    /// ```
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n* * log(*n*)) time and *O*(1) memory.
    ///
    /// # Current Implementation
    ///
    /// The current algorithm is done by a naive merge sort. There is no extra
    /// temporary storage during merging.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// let mut v = List::from([5, 4, 1, 3, 2]);
    /// v.sort_by(|a, b| a.cmp(b));
    /// assert_eq!(v.to_vec(), vec![1, 2, 3, 4, 5]);
    ///
    /// // reverse sorting
    /// v.sort_by(|a, b| b.cmp(a));
    /// assert_eq!(v.to_vec(), vec![5, 4, 3, 2, 1]);
    /// ```
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.merge_sort(|a, b| compare(a, b) == Ordering::Less)
    }

    /// Sorts the list with a key extraction function.
    ///
    /// This sort is stable (i.e., does not reorder equal elements)
    /// and *O*(*m* \* *n* \* log(*n*)) worst-case, where the
    /// key function is *O*(*m*).
    ///
    /// For expensive key functions (e.g. functions that are not simple
    /// property accesses or basic operations),
    /// [`sort_by_cached_key`](List::sort_by_cached_key) is likely to be
    /// significantly faster, as it does not recompute element keys.
    ///
    /// # Complexity
    ///
    /// This operation should compute in *O*(*n* * log(*n*)) time and *O*(1) memory.
    ///
    /// # Current Implementation
    ///
    /// The current algorithm is done by a naive merge sort. There is no extra
    /// temporary storage during merging.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// let mut v = List::from([-5i32, 4, 1, -3, 2]);
    ///
    /// v.sort_by_key(|k| k.abs());
    /// assert_eq!(v.into_vec(), vec![1, 2, -3, 4, -5]);
    /// ```
    pub fn sort_by_key<K, F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.merge_sort(|a, b| f(a).lt(&f(b)));
    }

    /// TODO
    pub fn sort_by_cached_key<K, F>(&mut self, _f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        unimplemented!()
    }
}
