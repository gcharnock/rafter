use std::time::{Instant, Duration};


pub trait TimeOracle<'a> {
    fn get_now(&self) -> Instant;
    fn set_timer(&self, timeout: Duration, callback: Box<dyn Fn() + 'a>);
    fn reset_timer(&self);
    fn get_random_duration(&self, min_time: Duration, max_time: Duration) -> Duration;
}


