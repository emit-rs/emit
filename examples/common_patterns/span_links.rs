/*!
This example demonstrates how to add links to spans.

Links relate spans outside of the normal parent-child hierarchy. In this example, we
have a dummy messaging system where a message includes a set of text-based headers.
One of these headers is a W3C traceparent, similar to how trace propagation works in
web-based applications.

Instead of linking the span for the worker to the producer as a child, we link it
using a span link instead.

The messaging infrastructure here isn't important. What's important is the way the span link
is attached to the span created on the worker thread, which happens in the `worker` function.
*/

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, Weak},
    thread,
    time::Duration,
};

type MessageQueue<T> = Mutex<VecDeque<Message<T>>>;

#[derive(Default)]
struct Message<T> {
    headers: HashMap<String, String>,
    data: T,
}

/*
The producer of a span link.
*/
#[emit::span("produce")]
fn produce(queue: Arc<MessageQueue<i32>>, data: i32) {
    let msg = Message {
        headers: {
            let mut headers = HashMap::new();

            // 1. Get the currently executing span
            let span_ctxt = emit::span::SpanCtxt::current(emit::ctxt());

            // 2. Format the span as a traceparent header
            let traceparent = emit_traceparent::Traceparent::new(
                span_ctxt.trace_id().copied(),
                span_ctxt.span_id().copied(),
                emit_traceparent::TraceFlags::SAMPLED,
            );

            headers.insert("traceparent".to_owned(), traceparent.to_string());

            headers
        },
        data,
    };

    queue.lock().unwrap().push_back(msg);
}

/*
The consumer of a span link.
*/
fn worker<T>(queue: Weak<MessageQueue<T>>, mut process: impl FnMut(T)) {
    loop {
        let process = &mut process;

        let Some(queue) = queue.upgrade() else {
            // The other side has hung up; return
            return;
        };

        let Some(msg) = queue.lock().unwrap().pop_front() else {
            // Nothing to do; wait for a bit
            thread::sleep(Duration::from_micros(1));
            continue;
        };

        // 1. Pull the incoming traceparent from the message headers.
        //    This could be done any number of ways depending on the messaging system.
        //    In this example, we're parsing it from text like a HTTP header.
        let span_links = msg
            .headers
            .get("traceparent")
            // Parse the `traceparent` header
            .and_then(|traceparent| emit_traceparent::Traceparent::try_from_str(traceparent).ok())
            // Only consider `traceparent`s that are sampled
            // We could also use this information to avoid creating
            // a span altogether. That would involve integrating
            // `emit_traceparent` into `emit::setup()`, and adding
            // the trace flags from the incoming traceparent to
            // the span created in the `new_span!` call below
            .filter(|traceparent| traceparent.is_sampled())
            // Convert the `traceparent` into a span link
            .and_then(|traceparent| {
                Some(emit::span::SpanLink::new(
                    *traceparent.trace_id()?,
                    *traceparent.span_id()?,
                ))
            })
            // Wrap the link in a sequence, which is the expected type of `span_links`
            .map(|span_link| [span_link]);

        // 2. Create a span for the worker that will include the span link
        let (span, frame) = emit::new_span!("worker");

        frame.call(move || {
            // 3. Add the link as a property on the span
            let mut span = span.push_props(
                span_links
                    .as_ref()
                    .map(|span_links| ("span_links", emit::Value::capture_serde(span_links))),
            );

            span.start();

            // Invoke the worker closure in the context of the span
            process(msg.data);
        })
    }
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    let queue = Arc::new(MessageQueue::<i32>::default());

    let worker = thread::spawn({
        let queue = Arc::downgrade(&queue);
        move || {
            worker(queue, |data: i32| {
                emit::info!("processing {data}");
            })
        }
    });

    produce(queue, 42);

    worker.join().unwrap();

    rt.blocking_flush(Duration::from_secs(5));
}
