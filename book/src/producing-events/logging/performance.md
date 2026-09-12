# Performance considerations for logging

## Filtering happens after construction

`emit` creates a complete `Event` before checking whether any filtering would discard it. Events are reasonably cheap to construct, because all capturing borrows from locals, but applications may want to use their own compile-time filtering if emission becomes expensive.

## Static runtimes avoid dynamic dispatch

`emit` uses an ambient, dynamically typed runtime by default. You can generally improve performance by using an explicitly specified static runtime instead. See [Configuring a static runtime](../../advanced-apps/embedded.md#configuring-a-static-runtime) for details.
