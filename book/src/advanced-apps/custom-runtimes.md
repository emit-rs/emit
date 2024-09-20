# Custom runtimes

- Create your own `emit::Runtime`s for subcomponents in your apps.
- A runtime with concrete types won't use dynamic dispatch on the hot path.
- Remove the `implicit_rt` Cargo feature to force a runtime to be specified whenever you use `emit::emit!` or `#[emit::span]`.
