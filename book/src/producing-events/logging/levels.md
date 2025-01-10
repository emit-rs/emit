# Log levels

Levels describe the significance of a log event in a coarse-grained way that's easy to organize and filter on. `emit` doesn't bake in the concept of log levels directly, but supports them through the `lvl` [well-known property](https://docs.rs/emit/0.11.4/emit/well_known/index.html). `emit` defines four well-known levels; a strong and a weak variant of informative and erroneous events:

- `"debug"`: for events supporting live debugging. These events are likely to be filtered out, or only retained for a short period.
- `"info"`: for most events. These events are the core diagnostics of your applications that give you a good picture of what's happening.
- `"warn"`: for recoverable or non-esential erroneous events. They may help explain some unexpected behavior or point to issues to investigate.
- `"error"`: for erroneous events that cause degraded behavior and need to be investigated.

When emitting events, you can use a macro corresponding to a given level to have it attached automatically:

- [`debug!`](https://docs.rs/emit/0.11.4/emit/macro.debug.html)
- [`info!`](https://docs.rs/emit/0.11.4/emit/macro.info.html)
- [`warn!`](https://docs.rs/emit/0.11.4/emit/macro.warn.html)
- [`error!`](https://docs.rs/emit/0.11.4/emit/macro.error.html)

`emit`'s levels are intentionally very coarse-grained and aren't intended to be extended. You can define your own levelling scheme in your applications if you want.
