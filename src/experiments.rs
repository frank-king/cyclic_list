use ghost_cell::{GhostCell, GhostToken};
use static_rc::StaticRc;
use std::borrow::BorrowMut;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::Deref;

pub struct List<'id, T> {
    links: [Option<NodePtr<'id, T>>; 2],
    #[cfg(feature = "length")]
    len: usize,
}

struct Node<'id, T> {
    links: [Option<NodePtr<'id, T>>; 2],
    elem: T,
}

type NodePtr<'id, T> = Half<GhostCell<'id, Node<'id, T>>>;

type Half<T> = StaticRc<T, 2, 4>;
type Full<T> = StaticRc<T, 4, 4>;

impl<'id, T> Node<'id, T> {
    const NEXT: usize = 0;
    const PREV: usize = 1;
    fn next(&self) -> Option<&NodePtr<'id, T>> {
        self.links[Self::NEXT].as_ref()
    }
    fn take_next(&mut self) -> Option<NodePtr<'id, T>> {
        self.links[Self::NEXT].take()
    }
    fn prev(&self) -> Option<&NodePtr<'id, T>> {
        self.links[Self::PREV].as_ref()
    }
    fn take_prev(&mut self) -> Option<NodePtr<'id, T>> {
        self.links[Self::PREV].take()
    }
    fn new(elem: T) -> Self {
        let links = [None, None];
        Self { elem, links }
    }
}

impl<'id, T> Default for List<'id, T> {
    fn default() -> Self {
        Self {
            links: [None, None],
            #[cfg(feature = "length")]
            len: 0,
        }
    }
}

impl<'id, T> List<'id, T> {
    const HEAD: usize = 0;
    const TAIL: usize = 1;

    /*
    fn for_each_at_side(&mut self, side: usize, token: &'id mut GhostToken<'id>, mut f: impl FnMut(&mut T)) {
        let mut current = &mut self.links[side];
        loop {
            let node = match current.take() {
                Some(node) => {
                    let (left, right) = Half::split(node);
                    let node = left.deref().borrow_mut(token);
                    let next = node.next();
                    f(&node.elem);
                    current.replace(Half::join(left, right));
                    current = &mut next;
                }
                None => break,
            };
            current
        }
        while let Some(node) = current {
            let (left, right) = Half::split(node.take());
        }
        if let Some(node) = self.links[side].take() {
            let (left, right) = Half::split(node);
            f(left.deref(), right.deref());
            self.links[side] = Some(Half::join(left, right));
        }

    }
     */
    fn head(&self) -> Option<&NodePtr<'id, T>> {
        self.links[Self::HEAD].as_ref()
    }
    /*
    fn split_head(&mut self, f: impl FnOnce(&NodeRef<'id, T>, &NodeRef<'id, T>)) {
        self.for_each_nodes(Self::HEAD, f);
    }

     */
    fn tail(&self) -> Option<&NodePtr<'id, T>> {
        self.links[Self::TAIL].as_ref()
    }
    /*
    fn split_tail(&mut self, f: impl FnOnce(&NodeRef<'id, T>, &NodeRef<'id, T>)) {
        self.for_each_nodes(Self::TAIL, f);
    }

     */
    fn push_at(&mut self, side: usize, elem: T, token: &mut GhostToken<'id>) {
        debug_assert!(side < 2);
        #[cfg(feature = "length")]
        {
            self.len += 1;
        }
        let oppo = 1 - side;
        let (left, right) = Full::split(Full::new(GhostCell::new(Node::new(elem))));
        match self.links[side].take() {
            Some(this_side) => {
                this_side.deref().borrow_mut(token).links[oppo] = Some(left);
                right.deref().borrow_mut(token).links[side] = Some(this_side);
            }
            None => self.links[oppo] = Some(left),
        }
        self.links[side] = Some(right);
    }
    fn pop_at(&mut self, side: usize, token: &mut GhostToken<'id>) -> Option<T> {
        debug_assert!(side < 2);
        #[cfg(feature = "length")]
        {
            self.len -= 1;
        }
        let oppo = 1 - side;
        let right = self.links[side].take()?;
        let left = match right.deref().borrow_mut(token).links[side].take() {
            Some(this_side) => {
                let left = this_side.deref().borrow_mut(token).links[oppo]
                    .take()
                    .unwrap();
                self.links[side] = Some(this_side);
                left
            }
            None => self.links[oppo].take().unwrap(),
        };
        Some(Full::into_box(Full::join(left, right)).into_inner().elem)
    }
}

impl<'id, T> List<'id, T> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn is_empty(&self) -> bool {
        self.head().is_none()
    }
    #[cfg(feature = "length")]
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn push_back(&mut self, elem: T, token: &mut GhostToken<'id>) {
        self.push_at(Self::TAIL, elem, token);
    }
    pub fn pop_back(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        self.pop_at(Self::TAIL, token)
    }
    pub fn push_front(&mut self, elem: T, token: &mut GhostToken<'id>) {
        self.push_at(Self::HEAD, elem, token);
    }
    pub fn pop_front(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        self.pop_at(Self::HEAD, token)
    }
    pub fn iter<'iter>(&'iter self, token: &'iter GhostToken<'id>) -> Iter<'id, 'iter, T> {
        Iter {
            head: self.head(),
            tail: self.tail(),
            #[cfg(feature = "length")]
            len: self.len(),
            token,
        }
    }
    pub fn for_each(&self, token: &GhostToken<'id>, mut f: impl FnMut(&T)) {
        self.iter(token).for_each(f)
    }
    pub fn for_each_mut(&self, token: &mut GhostToken<'id>, mut f: impl FnMut(&mut T)) {
        let mut current = self.head();
        while let Some(node) = current {
            let node = node.deref().borrow_mut(token);
            f(&mut node.elem);
            current = node.next();
        }
    }
    pub fn rfor_each_mut(&mut self, token: &mut GhostToken<'id>, mut f: impl FnMut(&mut T)) {
        let mut current = self.tail();
        while let Some(node) = current {
            let node = node.deref().borrow_mut(token);
            f(&mut node.elem);
            current = node.prev();
        }
    }
}

struct Iter<'id, 'iter, T> {
    head: Option<&'iter NodePtr<'id, T>>,
    tail: Option<&'iter NodePtr<'id, T>>,
    #[cfg(feature = "length")]
    len: usize,
    token: &'iter GhostToken<'id>,
}

impl<'id, 'iter, T> Iterator for Iter<'id, 'iter, T> {
    type Item = &'iter T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.head?;
        self.head = current.deref().borrow(self.token).next();
        Some(&current.deref().borrow(self.token).elem)
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

impl<'id, 'iter, T> ExactSizeIterator for Iter<'id, 'iter, T> {}

impl<'id, 'iter, T> DoubleEndedIterator for Iter<'id, 'iter, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.tail?;
        self.tail = current.deref().borrow(self.token).prev();
        Some(&current.deref().borrow(self.token).elem)
    }
}

#[cfg(test)]
mod tests {
    use crate::experiments::List;
    use ghost_cell::GhostToken;

    #[test]
    fn list_push_pop() {
        GhostToken::new(|mut token| {
            let mut list = List::new();
            assert!(list.is_empty());
            list.push_back(1, &mut token);
            list.push_front(2, &mut token);
            assert!(!list.is_empty());
            assert_eq!(list.pop_back(&mut token), Some(2));
            assert_eq!(list.pop_front(&mut token), Some(1));
            assert!(list.is_empty());
        })
    }
}
