use std::time::{Instant, Duration};
use std::ops::Add;
use std::cell::RefCell;
use std::collections::BinaryHeap;
use std::cmp::{Ordering, Reverse};


pub trait TimeOracle {
    fn get_now(&self) -> Instant;
    fn set_timer(&self, timeout: Duration, callback: Box<dyn FnOnce()>);
}


struct Timer {
    expires: Instant,
    callback: Box<dyn FnOnce()>,
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.expires.eq(&other.expires)
    }
}

impl Eq for Timer {}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expires.cmp(&other.expires)
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.expires.partial_cmp(&other.expires)
    }
}

pub struct MockTimeOracle {
    now: RefCell<Instant>,
    timers: RefCell<BinaryHeap<Reverse<Timer>>>,
}

impl MockTimeOracle {
    pub fn new() -> MockTimeOracle {
        Self {
            now: RefCell::new(Instant::now()),
            timers: RefCell::new(BinaryHeap::new()),
        }
    }

    pub fn add_time(&self, time: Duration) {
        debug!("time has been advanced by {:?}", time);
        let new_now = (*self.now.borrow()).add(time);
        *self.now.borrow_mut() = new_now;

        let mut timers = self.timers.borrow_mut();
        while let Some(next_expires) = timers.peek()
            .map(|timer| timer.0.expires) {
            if next_expires < new_now {
                (timers.pop().unwrap().0.callback)();
                continue;
            } else {
                break;
            }
        }
    }
}


impl TimeOracle for MockTimeOracle {
    fn get_now(&self) -> Instant {
        *self.now.borrow()
    }

    fn set_timer(&self, timeout: Duration, callback: Box<dyn FnOnce()>) {
        let timer = Timer {
            expires: self.now.borrow().add(timeout),
            callback
        };
        self.timers.borrow_mut().push(Reverse(timer));
    }
}
