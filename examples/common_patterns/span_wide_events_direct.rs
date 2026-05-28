/*!
This example demonstrates one approach to using `emit` for wide events.

Operations push their context into a single container which is emitted as a single large event at
the end. In this example, that container is passed around directly.
*/

use std::{
    collections::HashMap,
    io,
    ops::{ControlFlow, DerefMut},
    time::Duration,
};

// The `guard` control parameter lets us manipulate the span within the body of the function
// The `evt_props` control parameter lets us specify the type for properties on the resulting span event
#[emit::span(guard, evt_props: WideEvent::begin(), ok_lvl: "info", "Running an example")]
fn example() -> io::Result<()> {
    let mut cx = WideEvent::cx(guard.props_mut());

    // Our span guard carries our `WideEvent` context, so we can access it
    check(&mut cx, 7)?;

    // When `example` completes, the accumulated context will be emitted

    Ok(())
}

// Child operations add to the context
fn check(cx: &mut WideEvent, i: i32) -> io::Result<()> {
    cx.set("i", i);

    if i > 4 {
        cx.set("is_big", true);

        Err(io::Error::new(io::ErrorKind::Other, "value is too big"))
    } else {
        Ok(())
    }
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    let _ = example();

    rt.blocking_flush(Duration::from_secs(5));
}

// Our container for the wide event context
// This type just makes pushing props a little more ergonomic

struct WideEvent<'a>(Option<&'a mut WideEventProps>);

type WideEventProps = HashMap<String, emit::value::OwnedValue>;

impl<'a> WideEvent<'a> {
    pub fn begin() -> WideEventProps {
        Default::default()
    }

    pub fn cx(props: Option<&'a mut impl DerefMut<Target = WideEventProps>>) -> WideEvent<'a> {
        // `props` will be `None` if filtering excluded the span
        // For wide events, you're more likely to want tail sampling in case context changes what you want to emit
        WideEvent(props.map(|props| &mut **props))
    }

    pub fn set(&mut self, k: impl Into<String>, v: impl emit::value::ToValue) {
        let Some(ref mut props) = self.0 else {
            return;
        };

        props.insert(k.into(), v.to_value().to_owned());
    }
}

impl emit::Props for WideEvent<'_> {
    fn for_each<'v, F: FnMut(emit::Str<'v>, emit::Value<'v>) -> ControlFlow<()>>(
        &'v self,
        for_each: F,
    ) -> ControlFlow<()> {
        let Some(ref props) = self.0 else {
            return ControlFlow::Continue(());
        };

        emit::Props::for_each(&**props, for_each)
    }
}
