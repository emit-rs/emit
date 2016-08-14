//! Helpers for replacing values in a message template.
//! 
//! All properties in the template must be of the form: `"{label}"`.
//! Use either `MessageTempalte::new()` or `MessageTemplate::from_format()` to make sure
//! it's in the right format.

use std::collections::BTreeMap;
use std::str;
use events::Value;
use std::fmt::Write;

pub struct MessageTemplateRepl<'a> {
    text: &'a str,
    param_slices: Vec<ParamSlice>
}

macro_rules! repl_slice {
    ($_self:ident, $last_index:ident, $slice:ident, $result:ident, $values:ident) => (
        let lit = &$_self.text[$last_index..$slice.start];
        let r = write!($result, "{}", lit);
        r.unwrap();

        if let Some(ref val) = $values.get($slice.label.as_str()) {
            let r = write!($result, "{}", val);
            r.unwrap();
        }

        $last_index = $slice.end;
    )
}

impl <'a> MessageTemplateRepl<'a> {
    pub fn new(text: &'a str) -> MessageTemplateRepl {
        let slices = parse_slices(
            text.as_bytes(),
            State::Lit(0),
            Vec::new()
        );

        MessageTemplateRepl {
            text: text,
            param_slices: slices
        }
    }

    pub fn replace(&self, values: &BTreeMap<&str, Value>) -> String {
        let mut result = String::new();
        let mut slice_iter = self.param_slices.iter();
        let mut last_index = 0;

        //The first slice
        if let Some(slice) = slice_iter.next() {
            repl_slice!(self, last_index, slice, result, values);
        }

        //The middle slices
        for slice in slice_iter {
            repl_slice!(self, last_index, slice, result, values);
        }

        //The last slice
        if last_index < self.text.len() {
            let lit = &self.text[last_index..];
            write!(result, "{}", lit).is_ok();
        }

        result
    }

    pub fn text(&self) -> &str {
        self.text
    }
}

struct ParamSlice {
    pub start: usize,
    pub end: usize,
    pub label: String
}

enum State {
    Lit(usize),
    Label(usize)
}

//TODO: escape double curly braces
//TODO: don't treat unclosed labels patterns as labels
fn parse_slices<'a>(i: &'a [u8], state: State, mut slices: Vec<ParamSlice>) -> Vec<ParamSlice> {
    if i.len() == 0 {
        slices
    }
    else {
        match state {
            State::Lit(c_start) => {
                let (ci, rest) = parse_lit(i);
                let c_end = c_start + ci;

                parse_slices(rest, State::Label(c_end), slices)
            },
            State::Label(c_start) => {
                let (ci, rest, label) = parse_label(i);
                let c_end = c_start + ci;

                if let Some(label) = label {
                    slices.push(ParamSlice {
                        start: c_start,
                        end: c_end,
                        label: label.to_string()
                    });
                }

                parse_slices(rest, State::Lit(c_end), slices)
            }
        }
    }
}

//Parse the 'myproperty: ' in 'myproperty: {somevalue} other'
fn parse_lit<'a>(i: &'a [u8]) -> (usize, &'a [u8]) {
    shift_while(i, |c| c != b'{')
}

//TODO: If label isn't valid (escaped or unclosed) return None
//Parse the 'somevalue' in '{somevalue} other'
fn parse_label<'a>(i: &'a [u8]) -> (usize, &'a [u8], Option<&'a str>) {
    //Shift over the '{'
    let (c_open, k_open) = shift(i, 1);
    //Parse the label
    let (c, k_label, s) = take_while(k_open, |c| c != b'}');
    //Shift over the '}'
    let (c_close, k_close) = shift(k_label, 1);

    let name = match s.len() {
        0 => None,
        _ => Some(s)
    };

    (c_open + c + c_close, k_close, name)
}

fn take_while<F>(i: &[u8], f: F) -> (usize, &[u8], &str) where F: Fn(u8) -> bool {
    let mut ctr = 0;

    for c in i {
        if f(*c) {
            ctr += 1;
        }
        else {
            break;
        }
    }

    (ctr, &i[ctr..], str::from_utf8(&i[0..ctr]).unwrap())
}

fn shift(i: &[u8], c: usize) -> (usize, &[u8]) {
    match c {
        c if c >= i.len() => (i.len(), &[]),
        _ => (c, &i[c..])
    }
}

fn shift_while<F>(i: &[u8], f: F) -> (usize, &[u8]) where F: Fn(u8) -> bool {
    let mut ctr = 0;

    for c in i {
        if f(*c) {
            ctr += 1;
        }
        else {
            break;
        }
    }

    (ctr, &i[ctr..])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use ::templates::repl::MessageTemplateRepl;
    use ::events::IntoValue;

    #[test]
    fn values_are_replaced() {
        let template_repl = MessageTemplateRepl::new("C {A} D {Bert} E");

        let mut map = BTreeMap::new();
        map.insert("A", "value1".into_value());
        map.insert("Bert", "value2".into_value());

        let replaced = template_repl.replace(&map);

        assert_eq!("C value1 D value2 E", &replaced);
    }

    #[test]
    fn missing_values_are_replaced_as_blank() {
        let template_repl = MessageTemplateRepl::new("C {A} D {Bert} E");

        let mut map = BTreeMap::new();
        map.insert("Bert", "value2".into_value());

        let replaced = template_repl.replace(&map);

        assert_eq!("C  D value2 E", &replaced);
    }

    #[test]
    fn duplicate_values_are_replaced() {
        let template_repl = MessageTemplateRepl::new("C {A}{B} D {A} {B} E");

        let mut map = BTreeMap::new();
        map.insert("A", "value1".into_value());
        map.insert("B", "value2".into_value());

        let replaced = template_repl.replace(&map);

        assert_eq!("C value1value2 D value1 value2 E", &replaced);
    }

    #[test]
    fn leading_values_are_replaced() {
        let template_repl = MessageTemplateRepl::new("{A} DE {B} F");

        let mut map = BTreeMap::new();
        map.insert("A", "value1".into_value());
        map.insert("B", "value2".into_value());

        let replaced = template_repl.replace(&map);

        assert_eq!("value1 DE value2 F", &replaced);
    }

    #[test]
    fn trailing_values_are_replaced() {
        let template_repl = MessageTemplateRepl::new("C {A} D {B}");

        let mut map = BTreeMap::new();
        map.insert("A", "value1".into_value());
        map.insert("B", "value2".into_value());

        let replaced = template_repl.replace(&map);

        assert_eq!("C value1 D value2", &replaced);
    }

    //The issue is that {{B}} is escaped, so should be treated as a literal
    //Also that A isn't closed, so unclosed labels shouldn't be pushed
    #[test]
    fn malformed_labels_are_not_replaced() {
        let template_repl = MessageTemplateRepl::new("C {A} D {{B}} {A");

        let mut map = BTreeMap::new();
        map.insert("A", "value1".into_value());
        map.insert("B", "value2".into_value());

        let replaced = template_repl.replace(&map);

        assert_eq!("C value1 D } value1", &replaced);
    }
}
