/*!
This example demonstrates one approach to using `emit` for wide events.

Operations push their context into a single container which is emitted as a single large event at
the end. In this example, that container is ambient.
*/

use std::time::Duration;

use crate::wide_event::WideEvent;

// The `guard` control parameter lets us manipulate the span within the body of the function
// The `evt_props` control parameter lets us specify the type for properties on the resulting span event
#[emit::info_span(guard: span, evt_props: WideEvent::begin(), "Running an example", i)]
fn example(i: i32) {
    // Your code goes here

    check_i_is_big(i);
    check_i_is_even(i);
}

fn check_i_is_big(i: i32) {
    if i > 4 {
        WideEvent::set("is_big", true);
    } else {
        WideEvent::set("is_big", false);
    }
}

fn check_i_is_even(i: i32) {
    if i % 2 == 0 {
        WideEvent::set("is_even", true);
    } else {
        WideEvent::set("is_even", false);
    }
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    for i in 0..6 {
        example(i);
    }

    rt.blocking_flush(Duration::from_secs(5));
}

mod wide_event {
    /*!
    Ambient wide event context.

    This module implements a toy thread-local context for pushing wide event properties.
    It allows internal operations to contribute properties to the context without needing
    to pass it through to them all.

    It's not suitable for async code as-is, but could be converted by replacing non-`Sync`
    smart pointers with `Sync` ones, and by ensuring the context is initialized on-thread
    when futures move between them.
    */

    use std::{
        cell::{OnceCell, RefCell},
        collections::HashMap,
        ops::ControlFlow,
    };

    // Store our context in a thread-local
    // We're using a boolean to indicate whether the context is currently active or not
    thread_local! {
        static ACTIVE_CTXT: RefCell<(bool, ActiveCtxt)> = RefCell::new((false, HashMap::new()));
    }

    // Store wide event context in a hashmap
    type ActiveCtxt = HashMap<emit::Str<'static>, emit::value::OwnedValue>;

    /**
    The shared wide event context.

    The span that's responsible for emitting the wide event should initialize the context,
    and child operations should use the context ambiently.
    */
    pub struct WideEvent(OnceCell<ActiveCtxt>);

    impl Drop for WideEvent {
        fn drop(&mut self) {
            Self::clear(false);
        }
    }

    impl WideEvent {
        /**
        Begin a wide event.

        Once started, child operations can push context into it ambiently.
        Attempting to read properties from the context using `Props` will freeze the event.
        */
        pub fn begin() -> Self {
            Self::clear(true);

            WideEvent(OnceCell::new())
        }

        fn clear(activate: bool) {
            ACTIVE_CTXT.with(|ctxt| {
                let mut ctxt = ctxt.borrow_mut();

                ctxt.1 = Default::default();

                // Run this check after dumping context, so we can at least recover
                if ctxt.0 && activate {
                    ctxt.0 = false;

                    panic!("attempt to initialize overlapping wide event context");
                } else {
                    ctxt.0 = activate;
                }
            });
        }

        /**
        Set a property on the active wide event context.

        This method is a no-op if no context is currently active.
        */
        pub fn set(k: impl emit::str::ToStr, v: impl emit::value::ToValue) {
            ACTIVE_CTXT.with(|ctxt| {
                let mut ctxt = ctxt.borrow_mut();

                if !ctxt.0 {
                    return;
                }

                let key = k.to_str().to_owned();
                let value = v.to_value().to_owned();

                ctxt.1.insert(key, value);
            });
        }

        fn complete(&self) -> &ActiveCtxt {
            // Move the constructed context into the wide event so we can emit it
            self.0.get_or_init(|| {
                ACTIVE_CTXT.with(|ctxt| {
                    let mut ctxt = ctxt.borrow_mut();

                    if !ctxt.0 {
                        panic!("attempt to complete the same wide event context multiple times");
                    }

                    ctxt.0 = false;
                    ctxt.1.clone()
                })
            })
        }
    }

    impl emit::Props for WideEvent {
        fn for_each<'kv, F: FnMut(emit::Str<'kv>, emit::Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            for_each: F,
        ) -> ControlFlow<()> {
            emit::Props::for_each(self.complete(), for_each)
        }

        fn get<'v, K: emit::str::ToStr>(&'v self, key: K) -> Option<emit::Value<'v>> {
            emit::Props::get(self.complete(), key)
        }
    }
}
