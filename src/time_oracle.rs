use std::time::{Instant, Duration};
use std::ops::Add;
use std::cell::{RefCell, Cell};
use std::collections::{BinaryHeap, VecDeque};
use std::cmp::{Ordering, Reverse};


pub trait TimeOracle<'a> {
    fn get_now(&self) -> Instant;
    fn set_timer(&self, timeout: Duration, callback: Box<dyn Fn() + 'a>);
    fn reset_timer(&self);
    fn get_random_duration(&self, min_time: Duration, max_time: Duration) -> Duration;
}


struct Timer<'a> {
    expires: Instant,
    duration: Duration,
    callback: Box<dyn Fn() + 'a>,
}

pub struct MockTimeOracle<'a> {
    now: Cell<Instant>,
    timer: RefCell<Option<Timer<'a>>>,
    random_duration_queue: RefCell<VecDeque<Duration>>,
}

impl<'a> MockTimeOracle<'a> {
    pub fn new() -> MockTimeOracle<'a> {
        Self {
            now: Cell::new(Instant::now()),
            timer: RefCell::new(None),
            random_duration_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn add_time(&self, time: Duration) {
        debug!("time has been advanced by {:?}", time);
        let then = self.now.get();
        let new_now = then.add(time);
        self.now.set(new_now);

        if let Some(ref mut timer) = *self.timer.borrow_mut() {
            if timer.expires <= new_now {
                (timer.callback)();
                timer.expires = timer.expires.add(timer.duration);
            }
        }
    }

    pub fn push_duration(&self, duration: Duration) {
        self.random_duration_queue.borrow_mut().push_back(duration);
    }
}


impl<'a> TimeOracle<'a> for MockTimeOracle<'a> {
    fn get_now(&self) -> Instant {
        self.now.get()
    }

    fn set_timer(&self, timeout: Duration, callback: Box<dyn Fn() + 'a>) {
        let timer = Timer {
            expires: self.now.get().add(timeout),
            duration: timeout,
            callback,
        };
        *self.timer.borrow_mut() = Some(timer);
    }

    fn reset_timer(&self) {
        if let Some(ref mut timer) = *self.timer.borrow_mut() {
            info!("Timer reset");
            timer.expires = self.now.get().add(timer.duration);
        }
    }

    fn get_random_duration(&self, min_time: Duration, max_time: Duration) -> Duration {
        let duration = self.random_duration_queue.borrow_mut().pop_front().unwrap();
        if duration >= min_time && duration <= max_time {
            return duration;
        }
        panic!("Expected queue duration between {:?} and {:?} but was {:?}", min_time, max_time, duration);
    }
}
