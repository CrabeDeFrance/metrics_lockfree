use ahash::{HashMap, HashMapExt};
//use std::collections::HashMap;
use std::{cell::UnsafeCell, marker::PhantomPinned, pin::Pin};

// counter
pub struct CounterCell<const MAX_TAGS: usize> {
    values: UnsafeCell<[u64; MAX_TAGS]>,
    _pin: PhantomPinned,
}

impl<const MAX_TAGS: usize> Default for CounterCell<MAX_TAGS> {
    fn default() -> Self {
        CounterCell {
            values: UnsafeCell::new([0; MAX_TAGS]),
            _pin: PhantomPinned,
        }
    }
}

// on autorise countercell a être partagée entre les threads, mais pas la structure qui les
// contient
unsafe impl<const MAX_TAGS: usize> Sync for CounterCell<MAX_TAGS> {}

impl<const MAX_TAGS: usize> CounterCell<MAX_TAGS> {
    fn get(&self, idx: usize) -> u64 {
        if idx >= MAX_TAGS {
            panic!("get: idx >= MAX_TAGS");
        }

        let ptr = self.values.get() as *mut u64;
        unsafe { *ptr.add(idx) }
    }

    fn as_mut_ptr(&self) -> *mut [u64; MAX_TAGS] {
        self.values.get()
    }
}

impl<const MAX_TAGS: usize> CounterPin<MAX_TAGS> {
    pub fn get(&self, idx: usize) -> u64 {
        self.values.get(idx)
    }

    fn as_mut_ptr(&self) -> *mut [u64; MAX_TAGS] {
        self.values.as_ref().as_mut_ptr()
    }
}

pub struct CounterPin<const MAX_TAGS: usize> {
    // we want the values to never change their address (we have pointers on them)
    values: Pin<Box<CounterCell<MAX_TAGS>>>,
}

impl<const MAX_TAGS: usize> Default for CounterPin<MAX_TAGS> {
    fn default() -> Self {
        Self {
            values: Box::pin(CounterCell::default()),
        }
    }
}

pub type AllocTagsFn = fn(&[(String, String)]) -> Option<usize>;

pub struct Counter<const MAX_TAGS: usize = 1> {
    // ptr to list of values, indexed by tag id
    values: *mut [u64; MAX_TAGS],
    // local cache for mapping tags to id
    tags: HashMap<Vec<(String, String)>, usize>,
    // function to call to create a new tag, when it is not in local cache
    global_allocator: Option<AllocTagsFn>,
}

impl<const MAX_TAGS: usize> Counter<MAX_TAGS> {
    pub fn add(&mut self, inc: u64, tags: Option<&[(String, String)]>) {
        let idx = if let Some(tags) = tags {
            if let Some(idx) = self.tags_get(tags) {
                idx
            } else {
                // TODO log it : too many tags or global allocator not set
                return;
            }
        } else {
            0
        };

        if idx >= MAX_TAGS {
            panic!("get: idx >= MAX_TAGS");
        }

        unsafe {
            let ptr = self.values as *mut u64;
            *ptr.add(idx) += inc;
        };
    }

    fn tags_get(&mut self, tags: &[(String, String)]) -> Option<usize> {
        // try local cache first
        if let Some(id) = self.tags.get(tags) {
            return Some(*id);
        }

        if let Some(allocator) = self.global_allocator {
            if let Some(id) = (allocator)(tags) {
                self.tags.insert(tags.to_vec(), id);
                return Some(id);
            }
        }

        // TODO log it : too many tags or global allocator not set
        None
    }

    pub fn set_fn(mut self, f: AllocTagsFn) -> Self {
        self.global_allocator = Some(f);
        self
    }
}

impl<const MAX_TAGS: usize> From<&mut CounterPin<MAX_TAGS>> for Counter<MAX_TAGS> {
    fn from(cell: &mut CounterPin<MAX_TAGS>) -> Self {
        Counter {
            values: cell.as_mut_ptr(),
            tags: HashMap::new(),
            global_allocator: None,
        }
    }
}
