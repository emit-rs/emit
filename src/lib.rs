extern crate serde;
extern crate serde_json;
extern crate chrono;

pub mod message_templates;
pub mod payloads;
pub mod pipeline;

macro_rules! get_event_data {
    ($s:expr, $( $n:ident: $v:expr ),* ) => {{
        use std::fmt::Write;
        use std::collections;
        use serde_json;

        let mut names: Vec<&str> = vec![];
        let mut properties: collections::BTreeMap<&'static str, String> = collections::BTreeMap::new();

        $(
            names.push(stringify!($n));
            properties.insert(stringify!($n), serde_json::to_string(&$v).unwrap());            
        )*
        
        let template = $crate::message_templates::build_template($s, &names);
                
        (template, properties)
    }};
}

#[macro_export]
macro_rules! emit {
    ( $s:expr, $( $n:ident: $v:expr ),* ) => {{
        let (template, properties) = get_event_data!($s, $($n: $v),*);
        $crate::pipeline::emit(&template, &properties);
    }};
    
    ( $s:expr ) => {{
        emit!($s,);
    }};
}

#[cfg(test)]
mod tests {
    use message_templates::{build_template};

    #[test]
    fn templates_without_parameters_are_built() {
        let names: Vec<&str> = vec![];
        let s = "Hello, world!";
        
        let built = build_template(s, &names);
        
        assert!(built == s);
    }

    #[test]
    fn templates_with_parameters_are_built() {
        let names = vec!["A", "B"];
        let s = "C {} D {} E";
        
        let built = build_template(s, &names);
        
        assert!(built == "C {A} D {B} E");
    }

    #[test]
    fn additional_names_are_ignored() {
        let names = vec!["A", "B"];
        let s = "C {} D";
        
        let built = build_template(s, &names);
        
        println!("{}", built);
        assert!(built == "C {A} D");
    }

    #[test]
    fn additional_holes_are_indexed() {
        let names = vec!["A"];
        let s = "C {} D {} E";
        
        let built = build_template(s, &names);
        
        assert!(built == "C {A} D {1} E");
    }

    #[test]
    fn leading_holes_are_handled() {
        let names = vec!["A"];
        let s = "{} D";
        
        let built = build_template(s, &names);
        
        assert!(built == "{A} D");
    }

    #[test]
    fn trailing_holes_are_handled() {
        let names = vec!["A"];
        let s = "C {}";
        
        let built = build_template(s, &names);
        
        assert!(built == "C {A}");
    }

    #[test]
    fn it_works() {
        emit!("Starting...");
        
        let u = "World";
        let q = 42;
        
        emit!("User {} exceeded quota of {}!", user: u, quota: q);
    }
}
