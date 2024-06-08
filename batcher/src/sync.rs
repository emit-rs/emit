/*!
Run channels on regular OS threads.
*/

use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

use crate::{Channel, Sender};

#[derive(Clone)]
struct Trigger(Arc<(Mutex<bool>, Condvar)>);

impl Trigger {
    pub fn new() -> Self {
        Trigger(Arc::new((Mutex::new(false), Condvar::new())))
    }

    pub fn trigger(self) {
        *(self.0).0.lock().unwrap() = true;
        (self.0).1.notify_all();
    }

    pub fn wait_timeout(&self, mut timeout: Duration) -> bool {
        let mut flushed_slot = (self.0).0.lock().unwrap();
        loop {
            // If we flushed then return
            // This condition may already be set before we start waiting
            if *flushed_slot {
                return true;
            }

            let now = Instant::now();
            match (self.0).1.wait_timeout(flushed_slot, timeout).unwrap() {
                (flushed, r) if !r.timed_out() => {
                    flushed_slot = flushed;

                    // Reduce the remaining timeout just in case we didn't time out,
                    // but woke up spuriously for some reason
                    timeout = match timeout.checked_sub(now.elapsed()) {
                        Some(timeout) => timeout,
                        // We didn't time out, but got close enough that we should now anyways
                        None => {
                            return *flushed_slot;
                        }
                    };

                    continue;
                }
                // Timed out
                (flushed, _) => {
                    return *flushed;
                }
            }
        }
    }
}

/**
Wait for a channel running on a regular OS thread to process all items active at the point this call was made.
*/
pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    let on_flush = Trigger::new();

    sender.on_next_flush({
        let on_flush = on_flush.clone();

        move || {
            on_flush.trigger();
        }
    });

    on_flush.wait_timeout(timeout)
}
