/*!
The [`Level`] type.
*/

use emit_core::{
    event::ToEvent,
    filter::Filter,
    props::Props,
    runtime::InternalFilter,
    value::FromValue,
    well_known::{KEY_LVL, LVL_DEBUG, LVL_ERROR, LVL_INFO, LVL_WARN},
};

use crate::value::{ToValue, Value};
use core::{fmt, str::FromStr};

/**
A severity level for a diagnostic event.

If a [`crate::Event`] has a level associated with it, it can be pulled from its props:

```
# use emit::{Event, Props};
# fn with_event(evt: impl emit::event::ToEvent) {
# let evt = evt.to_event();
match evt.props().pull::<emit::Level, _>(emit::well_known::KEY_LVL).unwrap_or_default() {
    emit::Level::Debug => {
        // The event is at the debug level
    }
    emit::Level::Info => {
        // The event is at the info level
    }
    emit::Level::Warn => {
        // The event is at the warn level
    }
    emit::Level::Error => {
        // The event is at the error level
    }
}
# }
```

The default level is [`Level::Info`].
*/
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /**
    The event is weakly informative.

    This variant is equal to [`LVL_DEBUG`].
    */
    Debug,
    /**
    The event is informative.

    This variant is equal to [`LVL_INFO`].
    */
    Info,
    /**
    The event is weakly erroneous.

    This variant is equal to [`LVL_WARN`].
    */
    Warn,
    /**
    The event is erroneous.

    This variant is equal to [`LVL_ERROR`].
    */
    Error,
}

impl Default for Level {
    fn default() -> Self {
        Level::Info
    }
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => LVL_INFO,
            Level::Error => LVL_ERROR,
            Level::Warn => LVL_WARN,
            Level::Debug => LVL_DEBUG,
        })
    }
}

impl FromStr for Level {
    type Err = ParseLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lvl = s.as_bytes();

        match lvl.get(0) {
            Some(b'I') | Some(b'i') => parse(lvl, b"INFORMATION", Level::Info),
            Some(b'D') | Some(b'd') => {
                parse(lvl, b"DEBUG", Level::Debug).or_else(|_| parse(lvl, b"DBG", Level::Debug))
            }
            Some(b'E') | Some(b'e') => parse(lvl, b"ERROR", Level::Error),
            Some(b'W') | Some(b'w') => {
                parse(lvl, b"WARNING", Level::Warn).or_else(|_| parse(lvl, b"WRN", Level::Warn))
            }
            Some(_) => Err(ParseLevelError {}),
            None => Err(ParseLevelError {}),
        }
    }
}

fn parse(
    mut input: &[u8],
    mut expected_uppercase: &[u8],
    ok: Level,
) -> Result<Level, ParseLevelError> {
    // Assume the first character has already been matched
    input = &input[1..];
    expected_uppercase = &expected_uppercase[1..];

    // Doesn't require a full match of the expected content
    // For example, `INF` will match `INFORMATION`
    while let Some(b) = input.get(0) {
        let Some(e) = expected_uppercase.get(0) else {
            return Err(ParseLevelError {});
        };

        if b.to_ascii_uppercase() != *e {
            return Err(ParseLevelError {});
        }

        expected_uppercase = &expected_uppercase[1..];
        input = &input[1..];
    }

    Ok(ok)
}

/**
An error attempting to parse a [`Level`] from text.
*/
#[derive(Debug)]
pub struct ParseLevelError {}

impl fmt::Display for ParseLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the input was not a valid level")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseLevelError {}

impl ToValue for Level {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for Level {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<Level>()
            .copied()
            .or_else(|| value.parse())
    }
}

/**
Only match events that carry the given [`Level`].
*/
pub fn min_filter(min: Level) -> MinLevelFilter {
    MinLevelFilter::new(min)
}

/**
A [`Filter`] that matches events with a specific [`Level`].

The level to match is pulled from the [`KEY_LVL`] well-known property. Events that don't carry any specific level are treated as carrying a default one, as set by [`MinLevelFilter::treat_unleveled_as`].
*/
pub struct MinLevelFilter {
    min: Level,
    default: Level,
}

impl From<Level> for MinLevelFilter {
    fn from(min: Level) -> Self {
        MinLevelFilter::new(min)
    }
}

impl MinLevelFilter {
    /**
    Construct a new [`MinLevelFilter`], treating unleveled events as [`Level::default`].
    */
    pub const fn new(min: Level) -> MinLevelFilter {
        MinLevelFilter {
            min,
            default: Level::Info,
        }
    }

    /**
    Treat events without an explicit level as having `default` when evaluating against the filter.
    */
    pub fn treat_unleveled_as(mut self, default: Level) -> Self {
        self.default = default;
        self
    }
}

impl Filter for MinLevelFilter {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        evt.to_event()
            .props()
            .pull::<Level, _>(KEY_LVL)
            .unwrap_or(self.default)
            >= self.min
    }
}

impl InternalFilter for MinLevelFilter {}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::vec::Vec;

    use emit_core::path::Path;

    /**
    Construct a set of [`MinLevelFilter`]s that are applied based on the module of an event.
    */
    pub fn min_by_path_filter<P: Into<Path<'static>>, L: Into<MinLevelFilter>>(
        levels: impl IntoIterator<Item = (P, L)>,
    ) -> MinLevelPathMap {
        MinLevelPathMap::from_iter(levels)
    }

    /**
    A filter that applies a [`MinLevelFilter`] based on the module of an event.

    This type allows different modules to apply different level filters. In particular, modules generating a lot of diagnostic noise can be silenced without affecting other modules.

    Event modules are matched based on [`Path::is_child_of`]. If an event's module is a child of one in the map then its [`MinLevelFilter`] will be checked against it. If an event's module doesn't match any in the map then it will pass the filter.
    */
    pub struct MinLevelPathMap {
        // TODO: Ensure more specific paths apply ahead of less specific ones
        paths: Vec<(Path<'static>, MinLevelFilter)>,
    }

    impl MinLevelPathMap {
        /**
        Create an empty map.
        */
        pub const fn new() -> Self {
            MinLevelPathMap { paths: Vec::new() }
        }

        /**
        Specify the minimum level for a module and its children.
        */
        pub fn min_level(
            &mut self,
            path: impl Into<Path<'static>>,
            min_level: impl Into<MinLevelFilter>,
        ) {
            let path = path.into();

            match self.paths.binary_search_by_key(&&path, |(path, _)| path) {
                Ok(index) => {
                    self.paths[index] = (path, min_level.into());
                }
                Err(index) => {
                    self.paths.insert(index, (path, min_level.into()));
                }
            }
        }
    }

    impl Filter for MinLevelPathMap {
        fn matches<E: ToEvent>(&self, evt: E) -> bool {
            let evt = evt.to_event();

            let evt_path = evt.module();

            if let Ok(index) = self.paths.binary_search_by_key(&evt_path, |(path, _)| path) {
                self.paths[index].1.matches(evt)
            } else {
                for (path, min_level) in &self.paths {
                    if evt_path.is_child_of(path) {
                        return min_level.matches(evt);
                    }
                }

                true
            }
        }
    }

    impl InternalFilter for MinLevelPathMap {}

    impl<P: Into<Path<'static>>, L: Into<MinLevelFilter>> FromIterator<(P, L)> for MinLevelPathMap {
        fn from_iter<T: IntoIterator<Item = (P, L)>>(iter: T) -> Self {
            let mut map = MinLevelPathMap::new();

            for (path, min_level) in iter {
                map.min_level(path, min_level);
            }

            map
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_roundtrip() {
        for lvl in [Level::Info, Level::Debug, Level::Warn, Level::Error] {
            let fmt = lvl.to_string();

            let parsed: Level = fmt.parse().unwrap();

            assert_eq!(lvl, parsed, "{}", fmt);
        }
    }
}
