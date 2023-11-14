use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AutoIncrement {
    pub counter: AtomicUsize,
}

impl AutoIncrement {
    pub fn new(start: usize) -> Self { AutoIncrement { counter: AtomicUsize::new(start) } }
    pub fn next(&self) -> usize { self.counter.fetch_add(1, Ordering::SeqCst) }
}
