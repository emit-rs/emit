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

# Parsing

`Level` has a permissive parser that will match partial and incomplete contents, ignoring case. So long as the start of the text is a submatch of the full level, and any trailing unmatched characters are not ASCII control characters, the level will parse.

For example, the following will all be parsed as `Level::Info`:

- `info`
- `INFO`
- `i`
- `inf`
- `information`
- `INFO1`
- `inf(13)`

Note that any trailing data is lost when levels are parsed. That means, for example, that `INFO3` and `INFO4` will both parse to the same value, `Level::Info`, and will sort equivalently.
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

impl Level {
    /**
    Try parse a level from a formatted representation.
    */
    pub fn try_from_str(s: &str) -> Result<Self, ParseLevelError> {
        s.parse()
    }
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
        let s = s.trim();

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
        match b {
            // If the character is a letter then match against the expected value
            b if b.is_ascii_alphabetic() => {
                let Some(e) = expected_uppercase.get(0) else {
                    return Err(ParseLevelError {});
                };

                if b.to_ascii_uppercase() != *e {
                    return Err(ParseLevelError {});
                }

                expected_uppercase = &expected_uppercase[1..];
                input = &input[1..];
            }
            // If the character is not alphabetic then break
            // This lets us match more complex schemes like `info13` or `INFO(4)`
            b if b.is_ascii() && !b.is_ascii_control() => break,
            // Any other character is an error
            _ => {
                return Err(ParseLevelError {});
            }
        }
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
#[derive(Debug)]
pub struct MinLevelFilter<L = Level> {
    min: L,
    default: Option<L>,
}

impl From<Level> for MinLevelFilter {
    fn from(min: Level) -> Self {
        MinLevelFilter::new(min)
    }
}

impl<L> MinLevelFilter<L> {
    /**
    Construct a new [`MinLevelFilter`], treating unleveled events as [`Level::default`].
    */
    pub const fn new(min: L) -> MinLevelFilter<L> {
        MinLevelFilter { min, default: None }
    }

    /**
    Treat events without an explicit level as having `default` when evaluating against the filter.
    */
    pub fn treat_unleveled_as(mut self, default: L) -> Self {
        self.default = Some(default);
        self
    }
}

impl<L: for<'a> FromValue<'a> + Ord + Default> Filter for MinLevelFilter<L> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        evt.to_event()
            .props()
            .pull::<L, _>(KEY_LVL)
            .as_ref()
            .or_else(|| self.default.as_ref())
            .unwrap_or(&L::default())
            >= &self.min
    }
}

impl InternalFilter for MinLevelFilter {}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::vec::Vec;
    use emit_core::path::Path;
    use emit_core::str::Str;

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
    #[derive(Debug)]
    pub struct MinLevelPathMap<L = Level> {
        root: PathNode<L>,
    }

    #[derive(Debug)]
    struct PathNode<L> {
        min_level: Option<MinLevelFilter<L>>,
        children: Vec<(Str<'static>, PathNode<L>)>,
    }

    impl<L> MinLevelPathMap<L> {
        /**
        Create an empty map.
        */
        pub const fn new() -> Self {
            MinLevelPathMap {
                root: PathNode {
                    min_level: None,
                    children: Vec::new(),
                },
            }
        }

        /**
        Specify the minimum level for any modules that don't match any added by [`MinLevelPathMap::min_level`].
        */
        pub fn default_min_level(&mut self, min_level: impl Into<MinLevelFilter<L>>) -> &mut Self {
            self.root.min_level = Some(min_level.into());

            self
        }

        /**
        Specify the minimum level for a module and its children.
        */
        pub fn min_level(
            &mut self,
            path: impl Into<Path<'static>>,
            min_level: impl Into<MinLevelFilter<L>>,
        ) -> &mut Self {
            let path = path.into();

            let mut node = &mut self.root;
            for segment in path.segments() {
                node = match node
                    .children
                    .binary_search_by_key(&segment, |(key, _)| key.by_ref())
                {
                    Ok(idx) => &mut node.children[idx].1,
                    Err(idx) => {
                        node.children.insert(
                            idx,
                            (
                                segment.to_owned(),
                                PathNode {
                                    min_level: None,
                                    children: Vec::new(),
                                },
                            ),
                        );

                        &mut node.children[idx].1
                    }
                };
            }

            node.min_level = Some(min_level.into());

            self
        }
    }

    impl<L: for<'a> FromValue<'a> + Ord + Default> Filter for MinLevelPathMap<L> {
        fn matches<E: ToEvent>(&self, evt: E) -> bool {
            let evt = evt.to_event();

            let path = evt.mdl();

            // Find the most specific path to the given node
            let mut node = &self.root;
            let mut filter = self.root.min_level.as_ref();
            for segment in path.segments() {
                let Ok(idx) = node
                    .children
                    .binary_search_by_key(&segment, |(key, _)| key.by_ref())
                else {
                    break;
                };

                node = &node.children[idx].1;
                filter = node.min_level.as_ref().or(filter);
            }

            filter.matches(evt)
        }
    }

    impl InternalFilter for MinLevelPathMap {}

    impl<L, P: Into<Path<'static>>, T: Into<MinLevelFilter<L>>> FromIterator<(P, T)>
        for MinLevelPathMap<L>
    {
        fn from_iter<I: IntoIterator<Item = (P, T)>>(iter: I) -> Self {
            let mut map = MinLevelPathMap::new();

            for (path, min_level) in iter {
                map.min_level(path, min_level);
            }

            map
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn min_level() {
            let mut filter = MinLevelPathMap::new();

            filter.min_level(Path::new_raw("a"), Level::Error);

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));

            assert!(filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Error),
            )));
        }

        #[test]
        fn min_level_default() {
            let mut filter = MinLevelPathMap::new();

            filter.default_min_level(Level::Error);

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));

            assert!(filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Error),
            )));
        }

        #[test]
        fn min_level_child() {
            let mut filter = MinLevelPathMap::new();

            filter
                .min_level(Path::new_raw("a"), Level::Error)
                .min_level(Path::new_raw("a::b::c"), Level::Warn);

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("a::b"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));

            assert!(filter.matches(crate::Event::new(
                Path::new_raw("a::b::c"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));
        }

        #[test]
        fn min_level_unmatched() {
            let mut filter = MinLevelPathMap::new();

            filter.min_level(Path::new_raw("a"), Level::Error);

            assert!(filter.matches(crate::Event::new(
                Path::new_raw("b"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));
        }

        #[test]
        fn min_level_default_unmatched() {
            let mut filter = MinLevelPathMap::new();

            filter
                .default_min_level(Level::Error)
                .min_level(Path::new_raw("a"), Level::Error);

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("b"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, Level::Warn),
            )));
        }

        #[test]
        fn min_level_integer() {
            let mut filter = MinLevelPathMap::<u8>::new();

            filter
                .default_min_level(MinLevelFilter::new(3))
                .min_level(Path::new_raw("a"), MinLevelFilter::new(1));

            assert!(filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, 2),
            )));

            assert!(filter.matches(crate::Event::new(
                Path::new_raw("b"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, 6),
            )));

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("a"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, 0),
            )));

            assert!(!filter.matches(crate::Event::new(
                Path::new_raw("b"),
                crate::Template::literal("test"),
                crate::Empty,
                (KEY_LVL, 2),
            )));
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        for (case, expected) in [
            ("d", Ok(Level::Debug)),
            ("dbg", Ok(Level::Debug)),
            ("debug", Ok::<Level, ParseLevelError>(Level::Debug)),
            ("i", Ok(Level::Info)),
            ("inf", Ok(Level::Info)),
            ("info", Ok(Level::Info)),
            ("information", Ok(Level::Info)),
            ("w", Ok(Level::Warn)),
            ("wrn", Ok(Level::Warn)),
            ("warn", Ok(Level::Warn)),
            ("warning", Ok(Level::Warn)),
            ("e", Ok(Level::Error)),
            ("err", Ok(Level::Error)),
            ("error", Ok(Level::Error)),
            ("INFO3", Ok(Level::Info)),
            ("WRN(x)", Ok(Level::Warn)),
            ("info warn", Ok(Level::Info)),
            ("", Err(ParseLevelError {})),
            ("ifo", Err(ParseLevelError {})),
            ("trace", Err(ParseLevelError {})),
            ("erroneous", Err(ParseLevelError {})),
            ("infoℹ️", Err(ParseLevelError {})),
        ] {
            match expected {
                Ok(expected) => {
                    assert_eq!(expected, Level::from_str(case).unwrap());
                    assert_eq!(expected, Level::from_str(&case.to_uppercase()).unwrap());
                    assert_eq!(expected, Level::from_str(&format!(" {case} ")).unwrap());
                }
                Err(expected) => assert_eq!(
                    expected.to_string(),
                    Level::from_str(case).unwrap_err().to_string()
                ),
            }
        }
    }

    #[test]
    fn roundtrip() {
        for lvl in [Level::Info, Level::Debug, Level::Warn, Level::Error] {
            let fmt = lvl.to_string();

            let parsed: Level = fmt.parse().unwrap();

            assert_eq!(lvl, parsed, "{}", fmt);
        }
    }

    #[test]
    fn to_from_value() {
        for case in [Level::Debug, Level::Info, Level::Warn, Level::Error] {
            let value = case.to_value();

            assert_eq!(case, value.cast::<Level>().unwrap());

            let formatted = case.to_string();
            let value = Value::from(&*formatted);

            assert_eq!(case, value.cast::<Level>().unwrap());
        }
    }

    #[test]
    fn min_level_filter() {
        let filter = MinLevelFilter::new(Level::Warn);

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_ERROR),
        )));

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_WARN),
        )));

        assert!(!filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            crate::Empty,
        )));

        assert!(!filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_DEBUG),
        )));

        assert!(!filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_INFO),
        )));
    }

    #[test]
    fn min_level_filter_with_default() {
        let filter = MinLevelFilter::new(Level::Info).treat_unleveled_as(Level::Info);

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_ERROR),
        )));

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_WARN),
        )));

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_INFO),
        )));

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            crate::Empty,
        )));

        assert!(!filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, LVL_DEBUG),
        )));
    }

    #[test]
    fn min_level_filter_integer() {
        let filter = MinLevelFilter::new(3).treat_unleveled_as(1);

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, 3),
        )));

        assert!(filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, 4),
        )));

        assert!(!filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            (KEY_LVL, 2),
        )));

        assert!(!filter.matches(crate::Event::new(
            crate::Path::new_raw("test"),
            crate::Template::literal("test"),
            crate::Empty,
            crate::Empty,
        )));
    }
}
