use crate::list::algorithms::drain::{Drain, DrainFilter};
use crate::list::List;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

mod drain;
mod sort;

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

    /// Creates a draining iterator that removes and yields all
    /// the elements in the list.
    ///
    /// When the iterator is dropped, all elements are removed
    /// from the list, even if the iterator was not fully consumed.
    /// If the iterator is not dropped (with mem::forget for example),
    /// it is unspecified how many elements are removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut v = List::from_iter([1, 2, 3]);
    /// let u: Vec<_> = v.drain().collect();
    ///
    /// assert!(v.is_empty());
    /// assert_eq!(u, &[1, 2, 3]);
    /// ```
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain::new(self)
    }

    /// Creates an iterator which uses a closure to determine
    /// if an element should be removed.
    ///
    /// If the closure returns true, then the element is removed
    /// and yielded. If the closure returns false, the element
    /// will remain in the list and will not be yielded by the
    /// iterator.
    ///
    /// Note that `drain_filter` lets you mutate every element
    /// in the filter closure, regardless of whether you choose
    /// to keep or remove it.
    ///
    /// # Examples
    ///
    /// Splitting a list into evens and odds, reusing the original
    /// list:
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let mut numbers = List::<u32>::new();
    /// numbers.extend(&[1, 2, 3, 4, 5, 6, 8, 9, 11, 13, 14, 15]);
    ///
    /// let evens = numbers.drain_filter(|x| *x % 2 == 0).collect::<List<_>>();
    /// let odds = numbers;
    ///
    /// assert_eq!(Vec::from_iter(evens), vec![2, 4, 6, 8, 14]);
    /// assert_eq!(Vec::from_iter(odds), vec![1, 3, 5, 9, 11, 13, 15]);
    /// ```
    pub fn drain_filter<F>(&mut self, f: F) -> DrainFilter<'_, T, F>
    where
        F: FnMut(&mut T) -> bool,
    {
        DrainFilter::new(self, f)
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
        sort::merge_sort(self, |a, b| a.lt(b));
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
    /// when we know the list doesn’t contain a `NaN`.
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
        sort::merge_sort(self, |a, b| compare(a, b) == Ordering::Less)
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
        sort::merge_sort(self, |a, b| f(a).lt(&f(b)));
    }

    /// TODO
    pub fn sort_by_cached_key<K, F>(&mut self, _f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        unimplemented!()
    }

    /// Checks if the elements of this list are sorted.
    ///
    /// That is, for each element `a` and its following element `b`,
    /// `a <= b` must hold. If the list yields exactly zero or one
    /// element, true is returned.
    ///
    /// Note that if `T` is only `PartialOrd`, but not `Ord`, the
    /// above definition implies that this function returns false
    /// if any two consecutive items are not comparable.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// let empty = List::<u32>::new();
    ///
    /// assert!(List::from_iter([1, 2, 2, 9]).is_sorted());
    /// assert!(!List::from_iter([1, 3, 2, 4]).is_sorted());
    /// assert!(List::from_iter([0]).is_sorted());
    /// assert!(empty.is_sorted());
    /// assert!(!List::from_iter([0.0, 1.0, f32::NAN]).is_sorted());
    /// ```
    pub fn is_sorted(&self) -> bool
    where
        T: PartialOrd,
    {
        self.is_sorted_by(T::partial_cmp)
    }

    /// Checks if the elements of this list are sorted using the
    /// given comparator function.
    ///
    /// Instead of using `PartialOrd::partial_cmp`, this function
    /// uses the given compare function to determine the ordering
    /// of two elements. Apart from that, it’s equivalent to
    /// [`is_sorted`]; see its documentation for more information.
    ///
    /// [`is_sorted`]: List::is_sorted
    // FIXME: use `Iterator::is_sorted_by` once stabled.
    pub fn is_sorted_by<F>(&self, compare: F) -> bool
    where
        F: FnMut(&T, &T) -> Option<Ordering>,
    {
        #[inline]
        fn check<'a, T: Copy + 'a>(
            last: &'a mut T,
            mut compare: impl FnMut(T, T) -> Option<Ordering> + 'a,
        ) -> impl FnMut(T) -> bool + 'a {
            move |curr| {
                if let Some(Ordering::Greater) | None = compare(*last, curr) {
                    return false;
                }
                *last = curr;
                true
            }
        }

        let mut iter = self.iter();
        let mut last = match iter.next() {
            Some(e) => e,
            None => return true,
        };

        iter.all(check(&mut last, compare))
    }

    /// Checks if the elements of this list are sorted using the given
    /// key extraction function.
    ///
    /// Instead of comparing the list’s elements directly, this function
    /// compares the keys of the elements, as determined by `f`. Apart
    /// from that, it’s equivalent to [`is_sorted`]; see its documentation
    /// for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use cyclic_list::List;
    /// use std::iter::FromIterator;
    ///
    /// assert!(List::from_iter(["c", "bb", "aaa"]).is_sorted_by_key(|s| s.len()));
    /// assert!(!List::from_iter([-2i32, -1, 0, 3]).is_sorted_by_key(|n| n.abs()));
    /// ```
    ///
    /// [`is_sorted`]: List::is_sorted
    // FIXME: use `Iterator::is_sorted_by_key` once stabled.
    pub fn is_sorted_by_key<F, K>(&self, mut f: F) -> bool
    where
        F: FnMut(&T) -> K,
        K: PartialOrd,
    {
        self.is_sorted_by(|a, b| f(a).partial_cmp(&f(b)))
    }
}
