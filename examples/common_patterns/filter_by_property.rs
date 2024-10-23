/*!
This example demonstrates how to inspect the properties of an event and use them for filtering.
*/

use std::time::Duration;

use emit::Props;

fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .emit_when(emit::filter::from_fn(|evt| {
            // Event properties are where extension data lives
            // This can be used to tell, for example, whether an event
            // is a span in a distributed trace, a metric sample, or something else
            //
            // In this case, if the event is a span then we always emit it
            if let Some("span") = evt.props().pull("evt_kind") {
                return true;
            };

            // Find a property called "matches", and if it's `false`, don't emit the event
            evt.props().pull("matches").unwrap_or(true)
        }))
        .init();

    run();

    rt.blocking_flush(Duration::from_secs(5));
}

#[emit::span("running")]
fn run() {
    emit::info!("This event is emitted");
    emit::info!("This event is not emitted", matches: false);
}
