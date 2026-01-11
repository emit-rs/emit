/*!
This example demonstrates how to add links to spans.

Links relate spans outside of the normal parent-child hierarchy. In this example, we
have a dummy messaging system where a message includes the trace context of its producer.

Instead of linking the span for the worker to the producer as a child, we link it
using a span link instead.

The messaging infrastructure here isn't important. What's important is the way the span link
is attached to the span created on the worker thread, which happens in the `worker` function.
*/

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
    thread,
    time::Duration,
};

type MessageQueue<T> = Mutex<VecDeque<Message<T>>>;

struct Message<T> {
    producer_ctxt: emit::span::SpanCtxt,
    data: T,
}

/*
The producer of a span link.
*/
#[emit::span("produce")]
fn produce(queue: Arc<MessageQueue<i32>>, data: i32) {
    let msg = Message {
        producer_ctxt: emit::span::SpanCtxt::current(emit::ctxt()),
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

        // 1. Convert the producer's span context into a span link
        //    This could be done any number of ways depending on the system.
        //    In this example, we're just converting an `emit::span::SpanCtxt` into
        //    an `emit::span::SpanLink`.
        let span_links = if let (Some(trace_id), Some(span_id)) =
            (msg.producer_ctxt.trace_id(), msg.producer_ctxt.span_id())
        {
            Some([emit::span::SpanLink::new(*trace_id, *span_id)])
        } else {
            None
        };

        // 2. Create a span for the worker that will include the span link
        //    This is an inline alternative to creating a function with `#[emit::span]`
        //    on it, which we could also use
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
