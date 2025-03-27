# Propagating span context across threads

Ambient span properties are not shared across threads by default. This context needs to be fetched and sent across threads manually:

```rust
# extern crate emit;
# fn my_operation() {}
std::thread::spawn(emit::Frame::current(emit::ctxt()).in_fn(||{
    // Your code goes here
}));
```

or scoped threads:

```rust
# extern crate emit;
# fn my_operation() {}
let ctxt = emit::Frame::current(emit::ctxt());
std::thread::scope(|s| {
    ctxt.call(|| {
        s.spawn(emit::Frame::current(emit::ctxt()).in_fn(|| {/* frame active here */}));
        s.spawn(emit::Frame::current(emit::ctxt()).in_fn(|| {/* frame active here */}));

        // Also active here
    })
});
```

This same process is also needed for async code that involves thread spawning:

```rust
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
