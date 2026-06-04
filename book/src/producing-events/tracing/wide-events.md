# Wide events

Wide events are a pattern for application diagnostics that collects all the context for an operation together during its execution, and then emits a single event at the end with all of it. Wide events help you better structure your diagnostics by giving you a fixed paradigm to conform to so your diagnostics are more consistent. Span events are a natural fit for wide events because they already represent the execution of some operation, and carry additional timing and context. You could also use regular log events implement wide events.

You can implement wide events using `emit` by attaching an additional property collection to your outer-most span call and sharing it with child procedures. See [Adding properties to a span as it runs](./properties.md#adding-properties-to-a-span-as-it-runs) for details. A simple example could look like:

```rust
# extern crate emit;
# use std::{collections::HashMap, io, ops::{ControlFlow, DerefMut}};
// In our outer-most span we create our wide event context
#[emit::span(guard, evt_props: WideEvent::begin(), ok_lvl: "info", "exec")]
fn exec() -> io::Result<()> {
    let mut cx = WideEvent::cx(guard.props_mut());

    // Our span guard carries our `WideEvent` context, so we can access it
    check(&mut cx, 7)?;

    // When `exec` completes, the accumulated context will be emitted

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
    fn for_each<'v, F: FnMut(emit::Str<'v>, emit::Value<'v>) -> ControlFlow<()>>(&'v self, for_each: F) -> ControlFlow<()> {
        let Some(ref props) = self.0 else {
            return ControlFlow::Continue(());
        };

        emit::Props::for_each(&**props, for_each)
    }
}
```

```text
Event {
    mdl: "my_app",
    tpl: "example",
    extent: Some(
        "2026-05-28T01:05:04.957599420Z".."2026-05-28T01:05:04.957604581Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "example",
        "is_big": true,
        "i": 7,
        "trace_id": 2d9bb14ac65f414e1be85923000a3c30,
        "span_id": ac0115090dd86335,
    },
}
```

Applications may already have established patterns for handling this kind of context. All that's needed is for the value attached to the wide event span via its `evt_props` [control parameter](../../reference/control-parameters.md) to implement [`Props`](https://docs.rs/emit/1.19.0/emit/props/trait.Props.html), yielding any context pushed during its execution.
