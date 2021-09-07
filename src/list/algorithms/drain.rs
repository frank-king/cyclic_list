use crate::list::cursor::CursorMut;
use crate::List;
use std::fmt;

pub struct Drain<'a, T: 'a> {
    list: &'a mut List<T>,
}

impl<'a, T: 'a> Drain<'a, T> {
    pub(crate) fn new(list: &'a mut List<T>) -> Self {
        Self { list }
    }
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front()
    }
}

impl<T> Drop for Drain<'_, T> {
    fn drop(&mut self) {
        self.list.clear();
    }
}

impl<T: fmt::Debug> fmt::Debug for Drain<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(self.list).finish()
    }
}

pub struct DrainFilter<'a, T: 'a, F: 'a>
where
    F: FnMut(&mut T) -> bool,
{
    cursor: CursorMut<'a, T>,
    filter: F,
}

impl<'a, T, F> DrainFilter<'a, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    pub(crate) fn new(list: &'a mut List<T>, filter: F) -> Self {
        let cursor = list.cursor_start_mut();
        Self { cursor, filter }
    }
}

impl<T, F> Iterator for DrainFilter<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if (self.filter)(self.cursor.current_mut()?) {
                return self.cursor.remove();
            }
            self.cursor.move_next_cyclic();
        }
    }
}

impl<T, F> Drop for DrainFilter<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    fn drop(&mut self) {
        self.for_each(drop);
    }
}

impl<T: fmt::Debug, F> fmt::Debug for DrainFilter<'_, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DrainFilter")
            .field(self.cursor.list)
            .finish()
    }
}
