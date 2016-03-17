extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate hyper;

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
    
    #[test]
    fn it_works() {
        emit!("Starting...");
        
        let u = "World";
        let q = 42;
        
        emit!("User {} exceeded quota of {}!", user: u, quota: q);
    }
    
}
