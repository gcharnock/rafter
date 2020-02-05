use std::time::{Instant, Duration};
use std::ops::Add;
use std::cell::RefCell;
use std::collections::{BinaryHeap, VecDeque};
use std::cmp::{Ordering, Reverse};


pub trait TimeOracle<'a> {
    fn get_now(&self) -> Instant;
    fn set_timer(&self, timeout: Duration, callback: Box<dyn FnOnce() + 'a>);
    fn get_random_duration(&self, min_time: Duration, max_time: Duration) -> Duration;
}


struct Timer<'a> {
    expires: Instant,
    callback: Box<dyn FnOnce() + 'a>,
}

impl PartialEq for Timer<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.expires.eq(&other.expires)
    }
}

impl Eq for Timer<'_> {}

impl Ord for Timer<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expires.cmp(&other.expires)
    }
}

impl PartialOrd for Timer<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.expires.partial_cmp(&other.expires)
    }
}

pub struct MockTimeOracle<'a> {
    now: RefCell<Instant>,
    timers: RefCell<BinaryHeap<Reverse<Timer<'a>>>>,
    random_duration_queue: RefCell<VecDeque<Duration>>,
}

impl<'a> MockTimeOracle<'a> {
    pub fn new() -> MockTimeOracle<'a> {
        Self {
            now: RefCell::new(Instant::now()),
            timers: RefCell::new(BinaryHeap::new()),
            random_duration_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn add_time(&self, time: Duration) {
        debug!("time has been advanced by {:?}", time);
        let new_now = (*self.now.borrow()).add(time);
        *self.now.borrow_mut() = new_now;

        let mut timers = self.timers.borrow_mut();
        while let Some(next_expires) = timers.peek()
            .map(|timer| timer.0.expires) {
            if next_expires <= new_now {
                (timers.pop().unwrap().0.callback)();
                continue;
            } else {
                break;
            }
        }
    }

    pub fn push_duration(&self, duration: Duration) {
        self.random_duration_queue.borrow_mut().push_back(duration);
    }
}


impl<'a> TimeOracle<'a> for MockTimeOracle<'a> {
    fn get_now(&self) -> Instant {
        *self.now.borrow()
    }

    fn set_timer(&self, timeout: Duration, callback: Box<dyn FnOnce() + 'a>) {
        let timer = Timer {
            expires: self.now.borrow().add(timeout),
            callback,
        };
        self.timers.borrow_mut().push(Reverse(timer));
    }

    fn get_random_duration(&self, min_time: Duration, max_time: Duration) -> Duration {
        let duration = self.random_duration_queue.borrow_mut().pop_front().unwrap();
        if duration >= min_time && duration <= max_time {
            return duration;
        }
        panic!("Expected queue duration between {:?} and {:?} but was {:?}", min_time, max_time, duration);
    }
}
