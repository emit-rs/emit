# Sampling and filtering traces

- Once a span is created, it _must_ be completed. Otherwise you'll end up with a broken trace.
- `emit` doesn't do sampling as a first-class concept. It uses the [same filtering as any other event](../../filtering-events.md).
- `emit` spans do support levels, so you can filter them by level to produce finer or coarser grained traces.
