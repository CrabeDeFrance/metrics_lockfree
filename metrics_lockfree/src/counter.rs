use std::{cell::UnsafeCell, marker::PhantomPinned, pin::Pin};

// counter
#[derive(Default)]
pub struct CounterCell {
    value: UnsafeCell<u64>,
    _pin: PhantomPinned,
}

// on autorise countercell a être partagée entre les threads, mais pas la structure qui les
// contient
unsafe impl Sync for CounterCell {}

impl CounterCell {
    pub fn get(&self) -> u64 {
        unsafe { *self.value.get() }
    }

    pub fn as_mut_ptr(&self) -> *mut u64 {
        self.value.get()
    }
}

impl CounterPin {
    pub fn get(&self) -> u64 {
        self.value.get()
    }

    pub fn as_mut_ptr(&self) -> *mut u64 {
        self.value.as_ref().as_mut_ptr()
    }
}

// we want this struct to never change address of u64 value
pub struct CounterPin {
    pub value: Pin<Box<CounterCell>>,
}

impl Default for CounterPin {
    fn default() -> Self {
        Self {
            value: Box::pin(CounterCell::default()),
        }
    }
}

pub struct Counter {
    value: *mut u64,
}

impl Counter {
    pub fn add(&mut self, inc: u64) {
        unsafe { *self.value += inc }
    }
}

impl From<&mut CounterPin> for Counter {
    fn from(cell: &mut CounterPin) -> Self {
        Counter {
            value: cell.as_mut_ptr(),
        }
    }
}
