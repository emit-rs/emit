use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| {
            use emit::{Emitter as _, Props as _};

            if evt
                .props()
                .pull::<emit::Kind, _>(emit::well_known::KEY_EVT_KIND)
                == Some(emit::Kind::Span)
            {
                if let Some(elapsed) = evt.extent().and_then(|extent| extent.len()) {
                    let key = evt.tpl().to_owned();

                    let mut guard = SPAN_ELAPSED_SECS.lock().unwrap();

                    guard
                        .current_value_mut()
                        .entry(key)
                        .or_insert_with(|| emit::metric::exp::Histogram::new(160))
                        .observe(elapsed.as_secs_f64());
                }

                if let Some(tr) = evt
                    .props()
                    .pull::<emit::TraceId, _>(emit::well_known::KEY_TRACE_ID)
                {
                    if tr.to_u128() % 30_000 != 0 {
                        return;
                    }
                }
            }

            emit_term::stdout().emit(evt)
        }))
        .init();

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

    {
        let mut guard = SPAN_ELAPSED_SECS.lock().unwrap();
        let (extent, metrics) = guard.advance(emit::clock().now());
        let metrics = std::mem::take(metrics);
        drop(guard);

        for (tpl, distribution) in metrics {
            emit::count_sample!(
                extent,
                name: &tpl.to_string(),
                value: distribution.count(),
                props: distribution,
            );
        }
    }

    rt.blocking_flush(Duration::from_secs(5));
}

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

    for _ in 0..rand::random_range(1..2) {
        c();
    }
}

#[emit::span("c")]
fn c() {
    thread::sleep(Duration::from_millis(rand::random_range(3..5)));
}

static SPAN_ELAPSED_SECS: LazyLock<
    Mutex<emit::metric::Delta<HashMap<emit::Template<'static>, emit::metric::exp::Distribution>>>,
> = LazyLock::new(|| {
    Mutex::new(emit::metric::Delta::new(
        emit::clock().now(),
        HashMap::new(),
    ))
});
