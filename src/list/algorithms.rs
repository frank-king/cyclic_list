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
    const INSERTION_SORT_THRESHOLD: usize = 8;

    unsafe fn mid_of_range(
        &self,
        mut start: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
    ) -> (NonNull<Node<T>>, usize) {
        let mut mid = start;
        let mut len = 0;
        while start != end {
            len += 1;
            start = start.as_ref().next;
            if start != end {
                len += 1;
                start = start.as_ref().next;
                mid = mid.as_ref().next;
            }
        }
        (mid, len)
    }

    fn merge_sort<F>(&mut self, mut less: F)
    where
        F: FnMut(&T, &T) -> bool,
    {
        let (start, end) = (self.front_node(), self.ghost_node());
        #[cfg(feature = "length")]
        if self.len() < 2 {
        } else if self.len() <= Self::INSERTION_SORT_THRESHOLD {
            unsafe { self.insertion_sort_range(start, end, &mut less) };
        } else {
            unsafe { self.merge_sort_range(start, end, &mut less) };
        }

        #[cfg(not(feature = "length"))]
        {
            if self.is_empty() || start == self.back_node() {
                return;
            } else {
                unsafe { self.merge_sort_range(start, end, &mut less) };
            }
        }
    }

    unsafe fn merge_sort_range<F>(
        &mut self,
        mut start: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        less: &mut F,
    ) -> NonNull<Node<T>>
    where
        F: FnMut(&T, &T) -> bool,
    {
        let (mut mid, len) = self.mid_of_range(start, end);
        if len <= Self::INSERTION_SORT_THRESHOLD {
            return self.insertion_sort_range(start, end, less);
        }

        if start != mid && start.as_ref().next != mid {
            start = self.merge_sort_range(start, mid, less);
        }
        if mid != end && mid.as_ref().next != end {
            mid = self.merge_sort_range(mid, end, less);
        }

        if start != mid && mid != end {
            start = self.merge_range(start, mid, end, less);
        }
        start
    }

    unsafe fn merge_range<F>(
        &mut self,
        mut start: NonNull<Node<T>>,
        mid: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        less: &mut F,
    ) -> NonNull<Node<T>>
    where
        F: FnMut(&T, &T) -> bool,
    {
        let (mut merged, merged_back, mut to_merge) = (start, mid.as_ref().prev, mid);
        while to_merge != end && less(&to_merge.as_ref().element, &merged_back.as_ref().element) {
            while merged != to_merge && !less(&to_merge.as_ref().element, &merged.as_ref().element)
            {
                merged = merged.as_ref().next;
            }
            if merged == to_merge {
                break;
            }

            let mut next_to_merge = to_merge.as_ref().next;
            while next_to_merge != end
                && less(&next_to_merge.as_ref().element, &merged.as_ref().element)
            {
                next_to_merge = next_to_merge.as_ref().next;
            }
            if merged == start {
                start = to_merge;
            }
            self.move_nodes(to_merge, next_to_merge.as_ref().prev, merged);
            to_merge = next_to_merge;
        }
        start
    }

    unsafe fn insertion_sort_range<F>(
        &mut self,
        mut start: NonNull<Node<T>>,
        end: NonNull<Node<T>>,
        less: &mut F,
    ) -> NonNull<Node<T>>
    where
        F: FnMut(&T, &T) -> bool,
    {
        let (mut sorted_back, mut to_sort) = (start, start.as_ref().next);
        loop {
            while to_sort != end && !less(&to_sort.as_ref().element, &sorted_back.as_ref().element)
            {
                sorted_back = to_sort;
                to_sort = to_sort.as_ref().next;
            }
            if to_sort == end {
                break;
            }
            let mut sorted = start;
            while sorted != to_sort && !less(&to_sort.as_ref().element, &sorted.as_ref().element) {
                sorted = sorted.as_ref().next;
            }
            if sorted == start {
                start = to_sort;
            }
            let next = to_sort.as_ref().next;
            self.move_node(std::mem::replace(&mut to_sort, next), sorted);
        }
        start
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
        self.merge_sort(|a, b| a.lt(b));
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
