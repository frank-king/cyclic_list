# Double-Linked List

This crate provides a doubly-linked list with owned nodes, implemented as a
cyclic list.

## Usage

First, add dependency in your `Cargo.toml`:
```toml
[dependencies]
cyclic_list = "0.1"
```

Then enjoy it in your project.

### Examples

```rust
use cyclic_list::List;
use std::iter::FromIterator;

let mut list = List::from_iter([1, 2, 3, 4]);

let mut cursor = list.cursor_start_mut();

cursor.insert(0); // insert 0 at the beginning of the list
assert_eq!(cursor.current(), Some(&1));
assert_eq!(cursor.view(), &List::from_iter([0, 1, 2, 3, 4]));

cursor.seek_to(3); // move the cursor to position 3, and removes it.
assert_eq!(cursor.remove(), Some(3));
assert_eq!(cursor.view(), &List::from_iter([0, 1, 2, 4]));

cursor.push_front(5); // pushing front to the list is also allowed
assert_eq!(cursor.view(), &List::from_iter([5, 0, 1, 2, 4]));
```

## Introduction

The `List` allows inserting, removing elements at any given position in
constant time. In compromise, accessing or mutating elements at any position
take *O*(*n*) time.

### Memory Layout

The memory layout is like the following graph:
```text
         ┌─────────────────────────────────────────────────────────────────────┐
         ↓                                                     (Ghost) Node N  │
   ╔═══════════╗           ╔═══════════╗                        ┌───────────┐  │
   ║   next    ║ ────────→ ║   next    ║ ────────→ ┄┄ ────────→ │   next    │ ─┘
   ╟───────────╢           ╟───────────╢     Node 2, 3, ...     ├───────────┤
┌─ ║   prev    ║ ←──────── ║   prev    ║ ←──────── ┄┄ ←──────── │   prev    │
│  ╟───────────╢           ╟───────────╢                        ├───────────┤
│  ║ payload T ║           ║ payload T ║                        ┊No payload ┊
│  ╚═══════════╝           ╚═══════════╝                        └╌╌╌╌╌╌╌╌╌╌╌┘
│      Node 0                  Node 1                               ↑   ↑
└───────────────────────────────────────────────────────────────────┘   │
╔═══════════╗                                                           │
║   ghost   ║ ──────────────────────────────────────────────────────────┘
╟───────────╢
║   (len)   ║
╚═══════════╝
    List
```
### Iteration

Iterating over a list is by the [`Iter`] and [`IterMut`] iterators. These are
double-ended iterators and iterate the list like an array (fused and non-cyclic).
[`IterMut`] provides mutability of the elements (but not the linked structure of
the list).

#### Examples

```rust
use cyclic_list::List;
use std::iter::FromIterator;

let mut list = List::from_iter([1, 2, 3]);
let mut iter = list.iter();
assert_eq!(iter.next(), Some(&1));
assert_eq!(iter.next(), Some(&2));
assert_eq!(iter.next(), Some(&3));
assert_eq!(iter.next(), None);
assert_eq!(iter.next(), None); // Fused and non-cyclic

list.iter_mut().for_each(|item| *item *= 2);
assert_eq!(Vec::from_iter(list), vec![2, 4, 6]);
```

### Cursor Views

Beside iteration, the cursors [`Cursor`] and [`CursorMut`] provide more
flexible ways of viewing a list.

As the names suggest, they are like cursors and can move forward or backward
over the list. In a list with length *n*, there are *n* + 1 valid locations
for the cursor, indexed by 0, 1, ..., *n*, where *n* is the ghost node of the
list.

Cursors can also be used as iterators, but are cyclic and not fused.

**Warning**: Though cursor iterators have methods `rev`, they **DO NOT** behave
as double-ended iterators. Instead, they create a new iterator that reverses
the moving direction of the cursor.

#### Examples

```rust
use cyclic_list::List;
use std::iter::FromIterator;

let list = List::from_iter([1, 2, 3]);
// Create a cursor iterator
let mut cursor_iter = list.cursor_start().into_iter();
assert_eq!(cursor_iter.next(), Some(&1));
assert_eq!(cursor_iter.next(), Some(&2));
assert_eq!(cursor_iter.next(), Some(&3));
assert_eq!(cursor_iter.next(), None);
assert_eq!(cursor_iter.next(), Some(&1)); // Not fused and cyclic

// Create a cursor back iterator which reverses the moving direction
// of the cursor
let mut cursor_iter = cursor_iter.rev();
assert_eq!(cursor_iter.next(), Some(&1)); // Iterate in reversed direction
assert_eq!(cursor_iter.next(), None); // Pass through the ghost node boundary
assert_eq!(cursor_iter.next(), Some(&3)); // Reaches the ghost node
```

### Cursor Mutations

`CursorMut` provides many useful ways to mutate the list in any position.
- `insert`: insert a new item at the cursor;
- `remove`: remove the item at the cursor;
- `backspace`: remove the item before the cursor;
- `split`: split the list into a new one, from the cursor position to the end;
- `splice`: splice another list before the cursor position;

#### Examples

```rust
use cyclic_list::List;
use std::iter::FromIterator;

let mut list = List::from_iter([1, 2, 3, 4]);

let mut cursor = list.cursor_start_mut();

cursor.insert(5); // becomes [5, 1, 2, 3, 4], points to 1
assert_eq!(cursor.current(), Some(&1));

assert!(cursor.seek_forward(2).is_ok());
assert_eq!(cursor.remove(), Some(3)); // becomes [5, 1, 2, 4], points to 4
assert_eq!(cursor.current(), Some(&4));

assert_eq!(cursor.backspace(), Some(2)); // becomes [5, 1, 4], points to 4
assert_eq!(cursor.current(), Some(&4));

assert_eq!(Vec::from_iter(list), vec![5, 1, 4]);
```

## Develop Plans

Here is the develop plan of this project.

- [x] Basic supports: push, pop, insert, remove;
- [x] Cursor supports: move, seek, insert, remove, split, splice;
- [x] Iterator supports: from/into iterators, immutable/mutable iterators, 
      double-ended iterators, cursor-like iterators;
- [ ] Container operations:
    * [ ] rotate
    * [ ] reverse
- [ ] Algorithm supports:
    * [ ] drain
    * [ ] find
    * [ ] sort
    * [ ] sub-range view
- [ ] Advanced topics:
    * [ ] dynamic-sized types
    * [ ] concurrent support
