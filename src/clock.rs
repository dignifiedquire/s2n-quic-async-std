use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use std::time::Instant;

use s2n_quic_core::time::{self, Clock as ClockTrait, Timestamp};

#[derive(Clone, Debug)]
pub struct Clock(Instant);

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    pub fn new() -> Self {
        Self(Instant::now())
    }

    pub fn timer(&self) -> Timer {
        Timer::new(self.clone())
    }
}

impl ClockTrait for Clock {
    fn get_time(&self) -> time::Timestamp {
        let duration = self.0.elapsed();
        unsafe {
            // Safety: time duration is only derived from a single `Instant`
            time::Timestamp::from_duration(duration)
        }
    }
}

pub struct Timer {
    /// A reference to the current clock
    clock: Clock,
    /// The `Instant` at which the timer should expire
    target: Option<Instant>,
    /// The handle to the timer entry in the async_std runtime
    sleep: Pin<Box<dyn Future<Output = ()>>>,
}

impl Timer {
    fn new(clock: Clock) -> Self {
        /// We can't create a timer without first arming it to something, so just set it to 1s in
        /// the future.
        const INITIAL_TIMEOUT: Duration = Duration::from_secs(1);

        let target = clock.0 + INITIAL_TIMEOUT;
        let sleep = Box::pin(async_std::task::sleep(INITIAL_TIMEOUT));
        Self {
            clock,
            target: Some(target),
            sleep,
        }
    }

    /// Modifies the target expiration timestamp for the timer
    pub fn update(&mut self, timestamp: Timestamp) {
        let delay = unsafe {
            // Safety: the same clock epoch is being used
            timestamp.as_duration()
        };

        // floor the delay to milliseconds to reduce timer churn
        let delay = Duration::from_millis(delay.as_millis() as u64);

        // add the delay to the clock's epoch
        let next_time = self.clock.0 + delay;

        // If the target hasn't changed then don't do anything
        if Some(next_time) == self.target {
            return;
        }

        // if the clock has changed let the sleep future know
        let new_time = next_time.duration_since(self.clock.0);
        self.sleep = Box::pin(async_std::task::sleep(new_time));
        self.target = Some(next_time);
    }
}

impl Future for Timer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Only poll the inner timer if we have a target set
        if self.target.is_none() {
            return Poll::Pending;
        }

        let res = self.sleep.as_mut().poll(cx);

        if res.is_ready() {
            // clear the target after it fires, otherwise we'll endlessly wake up the task
            self.target = None;
        }

        res
    }
}
