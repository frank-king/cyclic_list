use crate::list::{connect, Node};
use crate::List;
use std::ptr::NonNull;

const INSERTION_SORT_THRESHOLD: usize = 8;

pub fn merge_sort<T, F>(list: &mut List<T>, mut less: F)
where
    F: FnMut(&T, &T) -> bool,
{
    let (start, end) = (list.front_node(), list.ghost_node());
    #[cfg(feature = "length")]
    if list.len() < 2 {
    } else if list.len() <= INSERTION_SORT_THRESHOLD {
        unsafe { insertion_sort_range(start, end, &mut less) };
    } else {
        unsafe { merge_sort_range(start, end, &mut less) };
    }

    #[cfg(not(feature = "length"))]
    if !list.is_empty() || start != list.back_node() {
        unsafe { merge_sort_range(start, end, &mut less) };
    }
}

unsafe fn mid_of_range<T>(
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

unsafe fn merge_sort_range<T, F>(
    mut start: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    less: &mut F,
) -> NonNull<Node<T>>
where
    F: FnMut(&T, &T) -> bool,
{
    let (mut mid, len) = mid_of_range(start, end);
    if len <= INSERTION_SORT_THRESHOLD {
        return insertion_sort_range(start, end, less);
    }

    if start != mid && start.as_ref().next != mid {
        start = merge_sort_range(start, mid, less);
    }
    if mid != end && mid.as_ref().next != end {
        mid = merge_sort_range(mid, end, less);
    }

    if start != mid && mid != end {
        start = merge_range(start, mid, end, less);
    }
    start
}

unsafe fn merge_range<T, F>(
    mut start: NonNull<Node<T>>,
    mid: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    less: &mut F,
) -> NonNull<Node<T>>
where
    F: FnMut(&T, &T) -> bool,
{
    // This algorithm first logically partitions the range into
    // two sub-range, both of which are internal sorted:
    // - merged range: `start..mid`,
    // - unmerged range: `mid..end`.
    //
    // Then merge the nodes in the unmerged range one by one
    // into the merged range.
    let (mut merged, merged_back, mut to_merge) = (start, mid.as_ref().prev, mid);
    // If the back of merged range <= the front of unmerged range,
    // it is fully sorted, the algorithm stops here.
    while to_merge != end && less(&to_merge.as_ref().element, &merged_back.as_ref().element) {
        // Find a position of `merged` in the merged range,
        // where the element of the current node to merge < `*merged`.
        while merged != to_merge && !less(&to_merge.as_ref().element, &merged.as_ref().element) {
            merged = merged.as_ref().next;
        }
        if merged == to_merge {
            break;
        }

        // Find a sub-range `to_merge..next_to_merge` in the unmerged range,
        // where all the element in it is < `*merged`.
        let mut next_to_merge = to_merge.as_ref().next;
        while next_to_merge != end
            && less(&next_to_merge.as_ref().element, &merged.as_ref().element)
        {
            next_to_merge = next_to_merge.as_ref().next;
        }
        if merged == start {
            start = to_merge;
        }
        // Move the sub-range `to_merged..next_to_range` to the
        // node before `merged`.
        move_nodes(to_merge, next_to_merge.as_ref().prev, merged);
        to_merge = next_to_merge;
    }
    start
}

unsafe fn insertion_sort_range<T, F>(
    mut start: NonNull<Node<T>>,
    end: NonNull<Node<T>>,
    less: &mut F,
) -> NonNull<Node<T>>
where
    F: FnMut(&T, &T) -> bool,
{
    let (mut sorted_back, mut to_sort) = (start, start.as_ref().next);
    loop {
        // If the back of sorted range <= the current node to sort,
        // then it is already sorted. Move on to sort the next node.
        while to_sort != end && !less(&to_sort.as_ref().element, &sorted_back.as_ref().element) {
            sorted_back = to_sort;
            to_sort = to_sort.as_ref().next;
        }
        if to_sort == end {
            break;
        }
        // Find a position of `sorted` in the sorted range,
        // where the element of the current node to sort < `*sorted`.
        let mut sorted = start;
        while sorted != to_sort && !less(&to_sort.as_ref().element, &sorted.as_ref().element) {
            sorted = sorted.as_ref().next;
        }
        if sorted == start {
            start = to_sort;
        }
        let next = to_sort.as_ref().next;
        // move the node `to_sort` to the node before `sorted`.
        move_node(std::mem::replace(&mut to_sort, next), sorted);
    }
    start
}

unsafe fn move_node<T>(from: NonNull<Node<T>>, to: NonNull<Node<T>>) {
    move_nodes(from, from, to);
}

unsafe fn move_nodes<T>(
    from_front: NonNull<Node<T>>,
    from_back: NonNull<Node<T>>,
    to: NonNull<Node<T>>,
) {
    connect(from_front.as_ref().prev, from_back.as_ref().next);
    connect(to.as_ref().prev, from_front);
    connect(from_back, to);
}
