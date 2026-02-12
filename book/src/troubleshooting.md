# Troubleshooting

Emitters write their own diagnostics to an alternative `emit` runtime, which you can configure via [`init_internal`](https://docs.rs/emit/1.17.2/emit/setup/struct.Setup.html#method.init_internal) to debug them:

```rust
# extern crate emit;
# extern crate emit_term;
# extern crate emit_file;
fn main() {
    // Configure the internal runtime before your regular setup
    let internal_rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    // Run your regular `emit` setup
    let rt = emit::setup()
        .emit_to(emit_file::set("./target/logs/my_app.txt").spawn())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));

    // Flush the internal runtime after your regular setup
    internal_rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
