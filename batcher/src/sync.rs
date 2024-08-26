/*!
Run channels on regular OS threads.
*/

use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

use crate::{BatchError, Channel, Sender};

/**
Wait for a channel running on a regular OS thread to process all items active at the point this call was made.
*/
pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    let notifier = Trigger::new();

    sender.when_flushed({
        let notifier = notifier.clone();

        move || {
            notifier.trigger();
        }
    });

    notifier.wait_timeout(timeout)
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.
*/
pub fn blocking_send<T: Channel>(
    sender: &Sender<T>,
    timeout: Duration,
    msg: T::Item,
) -> Result<(), BatchError<T::Item>> {
    crate::blocking_send(sender, timeout, msg, |timeout| {
        let notifier = Trigger::new();

        sender.when_empty({
            let notifier = notifier.clone();

            move || {
                let _ = notifier.trigger();
            }
        });

        notifier.wait_timeout(timeout);
    })
}

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

            // If the timeout is 0 then return
            // There's no point waiting for the condition
            if timeout == Duration::ZERO {
                return false;
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
