/*!
A diagnostic framework for Rust applications.

This library is the core API of `emit`, defining the fundamental abstractions used by the higher-level `emit` crate. This library is home to [`event::Event`], `emit`'s model of diagnostic data through with their [`template::Template`], [`props::Props`], and [`extent::Extent`].

In this library is also the all-encapsulating [`runtime::Runtime`], which collects the platform capabilities and event processing pipeline into a single value that powers the diagnostics for your applications.

If you're looking to explore and understand `emit`'s API, you can start with [`runtime::Runtime`] and [`event::Event`] and follow their encapsulated types.

If you're looking to use `emit` in an application you can use this library directly, but `emit` itself is recommended.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

extern crate core;

mod buf;

pub mod and;
pub mod clock;
pub mod ctxt;
pub mod emitter;
pub mod empty;
pub mod event;
pub mod extent;
pub mod filter;
pub mod or;
pub mod path;
pub mod props;
pub mod rng;
pub mod runtime;
pub mod str;
pub mod template;
pub mod timestamp;
pub mod value;
pub mod well_known;

/**
Emit an event.

This function uses the components of the runtime to process the event. It will:

1. Attempt to assign an extent to the event using [`clock::Clock::now`] if the event doesn't already have one.
2. Add [`ctxt::Ctxt::Current`] to the event properties.
3. Ensure the event passes [`filter::Filter::matches`].
4. Emit the event through [`emitter::Emitter::emit`].
*/
pub fn emit(
    emitter: impl emitter::Emitter,
    filter: impl filter::Filter,
    ctxt: impl ctxt::Ctxt,
    clock: impl clock::Clock,
    evt: impl event::ToEvent,
) {
    use self::{extent::ToExtent, props::Props};

    ctxt.with_current(|ctxt| {
        let evt = evt.to_event();

        let extent = evt.extent().cloned().or_else(|| clock.now().to_extent());

        let evt = evt
            .with_extent(extent)
            .map_props(|props| props.and_props(ctxt));

        if filter.matches(&evt) {
            emitter.emit(evt);
        }
    });
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{cell::Cell, time::Duration};

    use crate::props::Props as _;

    struct MyClock(Option<timestamp::Timestamp>);

    impl clock::Clock for MyClock {
        fn now(&self) -> Option<timestamp::Timestamp> {
            self.0
        }
    }

    struct MyCtxt(&'static str, usize);

    impl ctxt::Ctxt for MyCtxt {
        type Current = (&'static str, usize);
        type Frame = ();

        fn open_root<P: props::Props>(&self, _: P) -> Self::Frame {}

        fn enter(&self, _: &mut Self::Frame) {}

        fn exit(&self, _: &mut Self::Frame) {}

        fn close(&self, _: Self::Frame) {}

        fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
            with(&(self.0, self.1))
        }
    }

    #[test]
    fn emit_uses_clock_ctxt() {
        let called = Cell::new(false);

        emit(
            emitter::from_fn(|evt| {
                assert_eq!(13, evt.props().pull::<usize, _>("ctxt_prop").unwrap());
                assert_eq!(true, evt.props().pull::<bool, _>("evt_prop").unwrap());

                assert_eq!(
                    timestamp::Timestamp::from_unix(Duration::from_secs(77)).unwrap(),
                    evt.extent().unwrap().as_point()
                );

                called.set(true);
            }),
            empty::Empty,
            MyCtxt("ctxt_prop", 13),
            MyClock(timestamp::Timestamp::from_unix(Duration::from_secs(77))),
            event::Event::new(
                path::Path::new_unchecked("test"),
                template::Template::literal("text"),
                empty::Empty,
                ("evt_prop", true),
            ),
        );

        assert!(called.get());
    }

    #[test]
    fn emit_uses_filter() {
        emit(
            emitter::from_fn(|_| panic!("filter should apply")),
            filter::from_fn(|_| false),
            empty::Empty,
            empty::Empty,
            event::Event::new(
                path::Path::new_unchecked("test"),
                template::Template::literal("text"),
                empty::Empty,
                ("evt_prop", true),
            ),
        );
    }
}
