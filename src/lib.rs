//! This crate provides a doubly-linked list with owned nodes, implemented as a
//! cyclic list.
//!
//! The [`List`] allows inserting, removing elements at any given position in
//! constant time. In compromise, accessing or mutating elements at any position
//! take *O*(*n*) time.
//!
//! Here is a quick example showing hwo the list works.
//!
//! ```
//! use cyclic_list::List;
//! use std::iter::FromIterator;
//!
//! let mut list = List::from_iter([1, 2, 3, 4]);
//!
//! let mut cursor = list.cursor_start_mut();
//!
//! cursor.insert(0); // insert 0 at the beginning of the list
//! assert_eq!(cursor.current(), Some(&1));
//! assert_eq!(cursor.view(), &List::from_iter([0, 1, 2, 3, 4]));
//!
//! cursor.seek_to(3); // move the cursor to position 3, and removes it.
//! assert_eq!(cursor.remove(), Some(3));
//! assert_eq!(cursor.view(), &List::from_iter([0, 1, 2, 4]));
//!
//! cursor.push_front(5); // pushing front to the list is also allowed
//! assert_eq!(cursor.view(), &List::from_iter([5, 0, 1, 2, 4]));
//! ```
//!
//! # Memory Layout
//!
//! The memory layout of the list is like the following graph:
//! ```text
//!          ┌─────────────────────────────────────────────────────────────────────┐
//!          ↓                                                     (Ghost) Node N  │
//!    ╔═══════════╗           ╔═══════════╗                        ┌───────────┐  │
//!    ║   next    ║ ────────→ ║   next    ║ ────────→ ┄┄ ────────→ │   next    │ ─┘
//!    ╟───────────╢           ╟───────────╢     Node 2, 3, ...     ├───────────┤
//! ┌─ ║   prev    ║ ←──────── ║   prev    ║ ←──────── ┄┄ ←──────── │   prev    │
//! │  ╟───────────╢           ╟───────────╢                        ├───────────┤
//! │  ║ payload T ║           ║ payload T ║                        ┊No payload ┊
//! │  ╚═══════════╝           ╚═══════════╝                        └╌╌╌╌╌╌╌╌╌╌╌┘
//! │      Node 0                  Node 1                               ↑   ↑
//! └───────────────────────────────────────────────────────────────────┘   │
//! ╔═══════════╗                                                           │
//! ║   ghost   ║ ──────────────────────────────────────────────────────────┘
//! ╟───────────╢
//! ║   (len)   ║
//! ╚═══════════╝
//!     List
//! ```
//! The `List` contains:
//! - a pointer `ghost` that points to the ghost node;
//! - a length field `len` indicating the length of the list. It can be disabled by
//!   disabling the `length` feature in your `Cargo.toml`:
//! ```text
//! [dependencies]
//! cyclic_list = { default-features = false }
//! ```
//!
//! Each node of the list `List<T>` is allocated on heap, which contains:
//! - the `next` pointer that points to the next element (or the ghost node if it
//!   is the last element in the list);
//! - the `prev` pointer that points to the previous element (or the ghost node if
//!   it is the first element in the list);
//! - the actual payload `T` that depends on the element type of the list, except
//!   the ghost node.
//!
//! Note that the ghost node has *NO* payload to save memory.
//!
//! Initially, there is a ghost node in an empty list, of which the `next` and `prev`
//! pointer point to itself.
//!
//! As elements are inserted into the list, `ghost.next` points to the first element,
//! and `ghost.prev` points to the last element of the list.
//!
//! In convention, in a list with length *n*, the nodes are indexed by 0, 1, ...,
//! *n* - 1, and the ghost node is always indexed by *n*. (In an empty list, the
//! ghost nodes is indexed by 0, which is equal to its length 0).
//!
//! # Iteration
//!
//! Iterating over a list is by the [`Iter`] and [`IterMut`] iterators. These are
//! double-ended iterators and iterate the list like an array (fused and non-cyclic).
//! [`IterMut`] provides mutability of the elements (but not the linked structure of
//! the list).
//!
//! ## Examples
//!
//! ```
//! use cyclic_list::List;
//! use std::iter::FromIterator;
//!
//! let mut list = List::from_iter([1, 2, 3]);
//! let mut iter = list.iter();
//! assert_eq!(iter.next(), Some(&1));
//! assert_eq!(iter.next(), Some(&2));
//! assert_eq!(iter.next(), Some(&3));
//! assert_eq!(iter.next(), None);
//! assert_eq!(iter.next(), None); // Fused and non-cyclic
//!
//! list.iter_mut().for_each(|item| *item *= 2);
//! assert_eq!(Vec::from_iter(list), vec![2, 4, 6]);
//! ```
//!
//! # Cursor Views
//!
//! Beside iteration, the cursors [`Cursor`] and [`CursorMut`] provide more
//! flexible ways of viewing a list.
//!
//! As the names suggest, they are like cursors and can move forward or backward
//! over the list. In a list with length *n*, there are *n* + 1 valid locations
//! for the cursor, indexed by 0, 1, ..., *n*, where *n* is the ghost node of the
//! list.
//!
//! Cursors can also be used as iterators, but are cyclic and not fused.
//!
//! **Warning**: Though cursor iterators have methods `rev`, they **DO NOT** behave
//! as double-ended iterators. Instead, they create a new iterator that reverses
//! the moving direction of the cursor.
//!
//! ## Examples
//!
//! ```
//! use cyclic_list::List;
//! use std::iter::FromIterator;
//!
//! let list = List::from_iter([1, 2, 3]);
//! // Create a cursor iterator
//! let mut cursor_iter = list.cursor_start().into_iter();
//! assert_eq!(cursor_iter.next(), Some(&1));
//! assert_eq!(cursor_iter.next(), Some(&2));
//! assert_eq!(cursor_iter.next(), Some(&3));
//! assert_eq!(cursor_iter.next(), None);
//! assert_eq!(cursor_iter.next(), Some(&1)); // Not fused and cyclic
//!
//! // Create a cursor back iterator which reverses the moving direction
//! // of the cursor
//! let mut cursor_iter = cursor_iter.rev();
//! assert_eq!(cursor_iter.next(), Some(&1)); // Iterate in reversed direction
//! assert_eq!(cursor_iter.next(), None); // Pass through the ghost node boundary
//! assert_eq!(cursor_iter.next(), Some(&3)); // Reaches the ghost node
//! ```
//!
//! # Cursor Mutations
//!
//! [`CursorMut`] provides many useful ways to mutate the list in any position.
//! - [`insert`]: insert a new item at the cursor;
//! - [`remove`]: remove the item at the cursor;
//! - [`backspace`]: remove the item before the cursor;
//! - [`split`]: split the list into a new one, from the cursor position to the end;
//! - [`splice`]: splice another list before the cursor position;
//!
//! ## Examples
//!
//! ```
//! use cyclic_list::List;
//! use std::iter::FromIterator;
//!
//! let mut list = List::from_iter([1, 2, 3, 4]);
//!
//! let mut cursor = list.cursor_start_mut();
//!
//! cursor.insert(5); // becomes [5, 1, 2, 3, 4], points to 1
//! assert_eq!(cursor.current(), Some(&1));
//!
//! assert!(cursor.seek_forward(2).is_ok());
//! assert_eq!(cursor.remove(), Some(3)); // becomes [5, 1, 2, 4], points to 4
//! assert_eq!(cursor.current(), Some(&4));
//!
//! assert_eq!(cursor.backspace(), Some(2)); // becomes [5, 1, 4], points to 4
//! assert_eq!(cursor.current(), Some(&4));
//!
//! assert_eq!(Vec::from_iter(list), vec![5, 1, 4]);
//! ```
//!
//! See more functions in [`CursorMut`].
//!
//! # Algorithms
//!
//! TODO
//!
//! [`List`]: crate::List
//! [`Iter`]: crate::Iter
//! [`IterMut`]: crate::IterMut
//! [`Cursor`]: crate::list::cursor::Cursor
//! [`CursorMut`]: crate::list::cursor::CursorMut
//! [`CursorIter`]: crate::list::cursor::CursorIter
//! [`CursorIterMut`]: crate::list::cursor::CursorIterMut
//! [`CursorBackIter`]: crate::list::cursor::CursorBackIter
//! [`CursorBackIterMut`]: crate::list::cursor::CursorBackIterMut
//! [`insert`]: crate::list::cursor::CursorMut::insert
//! [`append`]: crate::list::cursor::CursorMut::append
//! [`remove`]: crate::list::cursor::CursorMut::remove
//! [`backspace`]: crate::list::cursor::CursorMut::backspace
//! [`split`]: crate::list::cursor::CursorMut::split
//! [`splice`]: crate::list::cursor::CursorMut::splice

#[doc(inline)]
pub use list::iterator::{IntoIter, Iter, IterMut};
#[doc(inline)]
pub use list::List;

pub mod list;

mod experiments;
