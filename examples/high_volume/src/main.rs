/*!
This example demonstrates mapping high frequency span events into exponential histograms.

Tracing every request in an application quickly becomes unfeasible. We don't want to just
throw away that valuable timing data though. We can use exponential histograms to compress
that data for all spans into a form we can still compute percentiles over without having
to store all the span events themselves.
*/

use std::{thread, time::Duration};

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| {
            use emit::{Emitter as _, Props as _};

            // If we're looking at a span event then collect some timing information from it
            if emit::kind::is_span(&evt) {
                if let Some(elapsed) = evt.extent().and_then(|extent| extent.len()) {
                    span_elapsed::observe(evt.tpl().to_owned(), elapsed);
                }

                // Filter out most spans for traces
                //
                // We do this here instead of as a top-level filter so the span is still
                // constructed and timed, even if we don't intend to fully emit it.
                if evt
                    .props()
                    .pull::<emit::TraceId, _>("trace_id")
                    .map(|tr| tr.to_u128())
                    .unwrap_or_default()
                    % 30_000
                    != 0
                {
                    return;
                }
            }

            // If we get this far then we're emitting the event

            emit_term::stdout().emit(evt)
        }))
        .init();

    // Construct some high-volume workers that will all produce spans
    let mut workers = Vec::new();
    for _ in 0..100 {
        workers.push(std::thread::spawn(|| {
            for _ in 0..1000 {
                a();
            }
        }));
    }

    for worker in workers {
        worker.join().unwrap();
    }

    // Emit our collected timing metrics
    //
    // In a more sophisticated application, we could use a `Reporter` to
    // do this regularly instead of just once at the end.
    for (tpl, distribution) in span_elapsed::take() {
        emit::count_sample!(
            name: tpl.to_string(),
            value: distribution.count(),
            props: distribution,
        );
    }

    rt.blocking_flush(Duration::from_secs(5));
}

// The work we're going to perform

#[emit::span("a")]
fn a() {
    thread::sleep(Duration::from_millis(rand::random_range(1..7)));

    for _ in 0..rand::random_range(1..3) {
        b();
    }
}

#[emit::span("b")]
fn b() {
    thread::sleep(Duration::from_millis(rand::random_range(1..3)));

    for _ in 0..rand::random_range(0..2) {
        c();
    }
}

#[emit::span("c")]
fn c() {
    thread::sleep(Duration::from_millis(rand::random_range(3..5)));
}

mod span_elapsed {
    use std::{
        collections::HashMap,
        sync::{LazyLock, Mutex},
        time::Duration,
    };

    // Track a set of distributions against the event's template
    //
    // Templates make good keys for identifying spans, so we can distinguish the timings of different operations.
    static SPAN_ELAPSED_SECS: LazyLock<
        Mutex<HashMap<emit::Template<'static>, emit::metric::exp::Distribution>>,
    > = LazyLock::new(|| Mutex::new(HashMap::new()));

    pub fn observe(key: emit::Template<'static>, elapsed: Duration) {
        let mut guard = SPAN_ELAPSED_SECS.lock().unwrap();

        guard
            .entry(key)
            .or_insert_with(|| emit::metric::exp::Distribution::default())
            .observe(elapsed.as_secs_f64());
    }

    pub fn take(
    ) -> impl IntoIterator<Item = (emit::Template<'static>, emit::metric::exp::Distribution)> {
        let mut guard = SPAN_ELAPSED_SECS.lock().unwrap();
        std::mem::take(&mut *guard)
    }
}
