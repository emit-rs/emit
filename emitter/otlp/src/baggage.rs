/*!
Support for the [W3C baggage](https://www.w3.org/TR/baggage/) format.
*/

use crate::Error;

use std::{borrow::Cow, ops::Range, str};

/**
Parse [W3C baggage](https://www.w3.org/TR/baggage/).

Baggage is a set of key-value pairs like:

```text
key1=value1;property1;property2, key2 = value2, key3=value3; propertyKey=propertyValue
```

Values themselves are a collection of key and optional percent-encoded value pairs.
*/
pub(crate) fn parse(input: &str) -> Result<Vec<(&str, Value)>, Error> {
    let mut results = Vec::new();

    let b = input.as_bytes();
    let mut state = State { i: 0, error: false };

    while state.i < b.len() {
        // (`key` OWS `=` OWS `value`) `,` *
        let key = parse_key(input, &mut state).trim_ascii();
        state.i += 1;
        let (value, properties, escaped) = parse_value(input, &mut state);
        state.i += 1;

        let value = if properties > 1 {
            // If the value contains nested properties then parse them out
            // We double handle here, but simplifies the `key=value` case
            let end = state.i;
            state.i = 0;
            let properties = parse_properties(value, &mut state, properties);
            state.i = end;

            Value::List(properties)
        } else if escaped {
            Value::Single(Cow::Owned(unescape(value.trim_ascii(), &mut state)))
        } else {
            Value::Single(Cow::Borrowed(value.trim_ascii()))
        };

        results.push((key, value));
    }

    if !state.error {
        Ok(results)
    } else {
        Err(Error::msg("baggage parser failed"))
    }
}

/**
A baggage value.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value<'a> {
    Single(Cow<'a, str>),
    List(Vec<(&'a str, Property<'a>)>),
}

/**
A nested property value.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Property<'a> {
    None,
    Single(Cow<'a, str>),
}

struct State {
    i: usize,
    error: bool,
}

fn parse_key<'a>(input: &'a str, state: &mut State) -> &'a str {
    let b = input.as_bytes();

    let key_start = state.i;

    while state.i < b.len() {
        if b[state.i] == b'=' {
            break;
        } else {
            state.i += 1;
            continue;
        }
    }

    let key_end = state.i;

    if key_start == key_end {
        state.error = true;
        ""
    } else {
        slice(b, key_start..key_end)
    }
}

fn parse_value<'a>(input: &'a str, state: &mut State) -> (&'a str, usize, bool) {
    let b = input.as_bytes();

    let value_start = state.i;

    let mut escaped = false;
    let mut properties = 1;
    while state.i < b.len() {
        match b[state.i] {
            b',' => break,
            b';' | b'=' => {
                properties += 1;
            }
            b'%' => {
                escaped = true;
            }
            _ => (),
        }

        state.i += 1;
        continue;
    }

    let value_end = state.i;

    if value_start == value_end {
        state.error = true;
        ("", 1, false)
    } else {
        (slice(b, value_start..value_end), properties, escaped)
    }
}

fn parse_properties<'a>(
    input: &'a str,
    state: &mut State,
    properties: usize,
) -> Vec<(&'a str, Property<'a>)> {
    // NOTE: This may overallocate (we track both `;` and `=` as properties)
    let mut properties = Vec::with_capacity(properties);

    let b = input.as_bytes();

    while state.i < b.len() {
        // (`key` OWS `=` OWS `value` OWS) `;` *
        let (key, has_value) = parse_property_key(input, state);
        state.i += 1;

        let value = if has_value {
            let (value, escaped) = parse_property_value(input, state);
            state.i += 1;

            if escaped {
                Property::Single(Cow::Owned(unescape(value.trim_ascii(), state)))
            } else {
                Property::Single(Cow::Borrowed(value.trim_ascii()))
            }
        } else {
            Property::None
        };

        properties.push((key.trim_ascii(), value));
    }

    properties
}

fn parse_property_key<'a>(input: &'a str, state: &mut State) -> (&'a str, bool) {
    let b = input.as_bytes();
    let key_start = state.i;

    let mut has_value = false;
    while state.i < b.len() {
        match b[state.i] {
            b';' => break,
            b'=' => {
                has_value = true;
                break;
            }
            _ => (),
        }

        state.i += 1;
        continue;
    }

    let key_end = state.i;

    (slice(b, key_start..key_end), has_value)
}

fn parse_property_value<'a>(input: &'a str, state: &mut State) -> (&'a str, bool) {
    let b = input.as_bytes();
    let value_start = state.i;

    let mut escaped = false;
    while state.i < b.len() {
        match b[state.i] {
            b';' => break,
            b'%' => {
                escaped = true;
            }
            _ => (),
        }

        state.i += 1;
        continue;
    }

    let value_end = state.i;

    (slice(b, value_start..value_end), escaped)
}

fn unescape(escaped: &str, state: &mut State) -> String {
    let b = escaped.as_bytes();

    let mut unescaped = Vec::with_capacity(b.len());

    let mut s = 0;
    let mut i = 0;

    while i < b.len() {
        if b[i] == b'%' {
            unescaped.extend_from_slice(&b[s..i]);
            i += 1;

            if b.len() < i + 2 {
                state.error = true;
                break;
            }

            unescaped.push(hex_byte(b[i], b[i + 1], state));
            i += 2;

            s = i;
            continue;
        }

        i += 1;
    }

    unescaped.extend_from_slice(&b[s..]);

    if let Ok(unescaped) = String::from_utf8(unescaped) {
        unescaped
    } else {
        state.error = true;
        String::new()
    }
}

fn hex_byte(a: u8, b: u8, state: &mut State) -> u8 {
    const HEX_DECODE_TABLE: &[u8; 256] = &{
        let mut buf = [0; 256];
        let mut i: u8 = 0;

        loop {
            buf[i as usize] = match i {
                b'0'..=b'9' => i - b'0',
                b'a'..=b'f' => i - b'a' + 10,
                b'A'..=b'F' => i - b'A' + 10,
                _ => 0xff,
            };

            if i == 255 {
                break buf;
            }

            i += 1
        }
    };

    let h1 = HEX_DECODE_TABLE[a as usize];
    let h2 = HEX_DECODE_TABLE[b as usize];

    if h1 | h2 == 0xff {
        state.error = true;
        0
    } else {
        (h1 << 4) | h2
    }
}

fn slice(b: &[u8], range: Range<usize>) -> &str {
    str::from_utf8(&b[range]).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        for (case, expected) in [
            ("", Vec::<(&str, Value)>::new()),
            ("a=b", vec![("a", Value::Single(Cow::Borrowed("b")))]),
            ("a = b", vec![("a", Value::Single(Cow::Borrowed("b")))]),
            ("a=b,c=d", vec![("a", Value::Single(Cow::Borrowed("b"))), ("c", Value::Single(Cow::Borrowed("d")))]),
            ("a=b,", vec![("a", Value::Single(Cow::Borrowed("b")))]),
            ("a=b=c", vec![("a", Value::List(vec![("b", Property::Single(Cow::Borrowed("c")))]))]),
            (
                "a=b;c=d",
                vec![("a", Value::List(vec![("b", Property::None), ("c", Property::Single(Cow::Borrowed("d")))]))],
            ),
            (
                "a = b; c = d",
                vec![("a", Value::List(vec![("b", Property::None), ("c", Property::Single(Cow::Borrowed("d")))]))],
            ),
            ("a=b;", vec![("a", Value::List(vec![("b", Property::None)]))]),
            ("a=b%20", vec![("a", Value::Single(Cow::Owned("b ".into())))]),
            (
                "key1=value1;property1;property2 , key2 = value2, key3=value3; propertyKey=property%20Value",
                vec![
                    (
                        "key1",
                        Value::List(vec![
                            ("value1", Property::None),
                            ("property1", Property::None),
                            ("property2", Property::None),
                        ]),
                    ),
                    ("key2", Value::Single(Cow::Borrowed("value2"))),
                    (
                        "key3",
                        Value::List(vec![
                            ("value3", Property::None),
                            ("propertyKey", Property::Single(Cow::Owned("property Value".into()))),
                        ]),
                    ),
                ],
            ),
        ] {
            let Ok(actual) = parse(case) else {
                panic!("parsing {case} failed");
            };

            assert_eq!(expected, actual, "parsing {case}");
        }
    }

    #[test]
    fn parse_invalid() {
        for case in [
            "a", "a=", "=a", "a;b", "a=,", "=,", "a,b", "a=b%", "a=b%1", "a=b%gg", "a=b%ff",
        ] {
            if let Ok(actual) = parse(case) {
                panic!("expected parsing {case} to fail but it produced {actual:?}");
            };
        }
    }
}
