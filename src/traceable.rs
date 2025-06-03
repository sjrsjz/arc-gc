use std::collections::VecDeque;

use crate::arc::GCArcWeak;

pub trait GCTraceable<T: GCTraceable<T> + 'static> {
    /// collects all reachable objects and adds them to the provided queue.
    fn collect(&self, queue: &mut VecDeque<GCArcWeak<T>>);
}
