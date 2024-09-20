# Propagating span context across threads

Ambient span properties are not shared across threads by default. This context needs to be fetched and sent across threads manually:

```rust
# extern crate emit;
# fn my_operation() {}
std::thread::spawn({
    let ctxt = emit::Frame::current(emit::ctxt());

    move || ctxt.call(|| {
        // Your code goes here
    })
});
```

This same process is also needed for async code that involves thread spawning:

```edition2021
# extern crate emit;
# mod tokio { pub fn spawn(_: impl std::future::Future) {} }
# fn main() {
tokio::spawn(
    emit::Frame::current(emit::ctxt()).in_future(async {
        // Your code goes here
    }),
);
# }
```

Async functions that simply migrate across threads in work-stealing runtimes don't need any manual work to keep their context across those threads.
