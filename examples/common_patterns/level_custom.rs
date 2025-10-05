/*!
This example demonstrates defining your own level type and using it in a filter.

When emitting events using a custom level, it's most convenient to supply them as strings, but you can also use attributes like `#[emit::as_display]`.
*/

use std::{str::FromStr, time::Duration};

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

impl FromStr for MyLevel {
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

fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .emit_when({
            let mut filter = emit::level::MinLevelPathMap::new();
            filter.min_level(
                emit::path!("level_custom::noisy"),
                emit::level::MinLevelFilter::new(MyLevel::Notice),
            );

            filter
        })
        .init();

    noisy::exec();

    rt.blocking_flush(Duration::from_secs(10));
}

mod noisy {
    pub fn exec() {
        emit::emit!("this event will be filtered out", lvl: "info");
        emit::emit!("this event will be emitted", lvl: "notice");
    }
}
