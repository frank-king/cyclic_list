use ghost_cell::{GhostCell, GhostToken};
use static_rc::StaticRc;
use std::ops::Deref;

pub struct List<'id, T> {
    links: [Option<NodePtr<'id, T>>; 2],
}

struct Node<'id, T> {
    links: [Option<NodePtr<'id, T>>; 2],
    elem: T,
}

type NodePtr<'id, T> = Half<GhostCell<'id, Node<'id, T>>>;

type Half<T> = StaticRc<T, 1, 2>;
type Full<T> = StaticRc<T, 2, 2>;

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
        let links = [None, None];
        Self { links }
    }
}

impl<'id, T> List<'id, T> {
    const HEAD: usize = 0;
    const TAIL: usize = 1;

    fn head(&self) -> Option<&NodePtr<'id, T>> {
        self.links[Self::HEAD].as_ref()
    }
    fn tail(&self) -> Option<&NodePtr<'id, T>> {
        self.links[Self::TAIL].as_ref()
    }
    fn push_at(&mut self, side: usize, elem: T, token: &mut GhostToken<'id>) {
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
