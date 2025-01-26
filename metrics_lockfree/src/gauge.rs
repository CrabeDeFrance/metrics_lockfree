use std::{cell::UnsafeCell, marker::PhantomPinned, pin::Pin};

// counter
#[derive(Default)]
pub struct GaugeCell {
    value: UnsafeCell<u64>,
    _pin: PhantomPinned,
}

// on autorise countercell a être partagée entre les threads, mais pas la structure qui les
// contient
unsafe impl Sync for GaugeCell {}

impl GaugeCell {
    fn get(&self) -> u64 {
        unsafe { *self.value.get() }
    }

    fn as_mut_ptr(&self) -> *mut u64 {
        self.value.get()
    }
}

impl GaugePin {
    pub fn get(&self) -> u64 {
        self.value.get()
    }

    fn as_mut_ptr(&self) -> *mut u64 {
        self.value.as_ref().as_mut_ptr()
    }
}

// we want this struct to never change address of u64 value
pub struct GaugePin {
    value: Pin<Box<GaugeCell>>,
}

impl Default for GaugePin {
    fn default() -> Self {
        Self {
            value: Box::pin(GaugeCell::default()),
        }
    }
}

pub struct Gauge {
    value: *mut u64,
}

impl Gauge {
    pub fn set(&mut self, inc: u64) {
        unsafe { *self.value = inc }
    }
}

impl From<&mut GaugePin> for Gauge {
    fn from(cell: &mut GaugePin) -> Self {
        Gauge {
            value: cell.as_mut_ptr(),
        }
    }
}
