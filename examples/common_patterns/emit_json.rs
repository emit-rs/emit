/*!
This example demonstrates an emitter that writes newline JSON to the terminal.

The `Event` type doesn't implement `serde::Serialize` directly, so you're free to pick a representation
that suits your needs. You can serialize types that implement `emit::Props` using the `as_map` method,
which we use on both the extent and props to flatten them onto a single object.

You can tweak the `Event` type as written here to produce a different result.
*/

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| {
            use emit::Props as _;

            // Generics avoid needing to specify concrete types here
            #[derive(serde::Serialize)]
            struct Event<E, M, R, P> {
                #[serde(flatten)]
                extent: E,
                mdl: M,
                msg: R,
                #[serde(flatten)]
                props: P,
            }

            let json = serde_json::to_string(&Event {
                // `as_map()` serializes the extent as a map with one or two keys:
                // `ts` for the end timestamp, and `ts_start` for the start, if there is one
                extent: evt.extent().as_map(),
                mdl: evt.mdl(),
                msg: evt.msg(),
                // `dedup()` ensures there are no duplicate properties
                // `as_map()` serializes properties as a map where each property is a key-value pair
                props: evt.props().dedup().as_map(),
            })
            .unwrap();

            // Instead of printing to the console here we could use `emit_file` or any other
            // `io::Write`. If that destination requires flushing, consider implementing
            // `emit::Emitter` instead of using `from_fn` here
            println!("{json}");
        }))
        .init();

    let user = "Rust";

    emit::info!("Hello, {user}");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
