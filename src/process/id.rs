use core::fmt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Deserialize, Serialize)]
pub struct Id {
    pub counter: AtomicUsize,
}

impl Id {
    pub fn new(start: usize) -> Self {
        Id {
            counter: AtomicUsize::new(start),
        }
    }
    pub fn next(&self) -> usize {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

impl Clone for Id {
    fn clone(&self) -> Self {
        Id {
            counter: AtomicUsize::new(self.counter.load(Ordering::SeqCst)),
        }
    }
}

impl FromStr for Id {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse::<usize>() {
            Ok(Id::new(value))
        } else {
            Err("Failed to parse string into usize")
        }
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        match s.parse::<Id>() {
            Ok(id) => id,
            Err(_) => Id::new(0),
        }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.counter.load(Ordering::SeqCst))
    }
}
