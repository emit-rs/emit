# Log levels

Levels describe the significance of a log event in a coarse-grained way that's easy to organize and filter on. `emit` doesn't bake in the concept of log levels directly, but supports them through the `lvl` [well-known property](https://docs.rs/emit/1.17.1/emit/well_known/index.html). `emit` defines four well-known levels; a strong and a weak variant of informative and erroneous events:

- `"debug"`: for events supporting live debugging. These events are likely to be filtered out, or only retained for a short period.
- `"info"`: for most events. These events are the core diagnostics of your applications that give you a good picture of what's happening.
- `"warn"`: for recoverable or non-esential erroneous events. They may help explain some unexpected behavior or point to issues to investigate.
- `"error"`: for erroneous events that cause degraded behavior and need to be investigated.

When emitting events, you can use a macro corresponding to a given level to have it attached automatically:

- [`debug!`](https://docs.rs/emit/1.17.1/emit/macro.debug.html)
- [`info!`](https://docs.rs/emit/1.17.1/emit/macro.info.html)
- [`warn!`](https://docs.rs/emit/1.17.1/emit/macro.warn.html)
- [`error!`](https://docs.rs/emit/1.17.1/emit/macro.error.html)

## Custom levels

`emit`'s well-known levels are intentionally very coarse-grained and aren't intended to be extended. If you need finer grained levels, you can define your own scheme. Your scheme should integrate with the `debug`, `info`, `warn`, and `error` scheme `emit` uses by default, but doesn't technically have to either.

To use a custom level, you can specify your own value for the `lvl` [well-known property](https://docs.rs/emit/1.17.1/emit/well_known/index.html) when emitting an event:

```rust
# extern crate emit;
emit::emit!("Some noteworthy event", lvl: "notice");
```

If you define your own level type, you can also use it when constructing [a level filter](../../filtering-events.md#filtering-by-level):

```rust
# extern crate emit;
// Define your custom level as a struct or enum
//
// To use your level in a `MinLevelPathMap`, it needs to implement the following traits:
// - `Default`
// - `FromValue`
// - `Ord`
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MyLevel {
    Debug,
    Info,
    Notice,
    Warn,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl Default for MyLevel {
    fn default() -> Self {
        MyLevel::Info
    }
}

// The `FromStr` impl here is primitive, but makes the `FromValue` impl able
// to parse levels supplied as strings
impl std::str::FromStr for MyLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_ascii_lowercase() {
            "debug" => Ok(MyLevel::Debug),
            "info" => Ok(MyLevel::Info),
            "notice" => Ok(MyLevel::Notice),
            "warn" => Ok(MyLevel::Warn),
            "error" => Ok(MyLevel::Error),
            "critical" => Ok(MyLevel::Critical),
            "alert" => Ok(MyLevel::Alert),
            "emergency" => Ok(MyLevel::Emergency),
            _ => Err(format!("'{s}' was not recognized as a level")),
        }
    }
}

impl<'a> emit::value::FromValue<'a> for MyLevel {
    fn from_value(value: emit::Value<'a>) -> Option<Self> {
        value.downcast_ref().copied().or_else(|| value.parse())
    }
}

let rt = emit::setup()
    .emit_when({
        // If you're using a custom level, you need to construct your `MinLevelPathMap` manually
        let mut filter = emit::level::MinLevelPathMap::new();

        filter.min_level(
            emit::path!("level_custom::noisy"),
            emit::level::MinLevelFilter::new(MyLevel::Notice),
        );

        filter
    })
    .init();

// Your app code goes here

rt.blocking_flush(std::time::Duration::from_secs(5));
```
