extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate hyper;

#[macro_use]
extern crate log;

pub mod message_templates;
pub mod payloads;
pub mod pipeline;

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_get_event_data {
    ($target:expr, $s:expr, $( $n:ident: $v:expr ),* ) => {{
        #[allow(unused_imports)]
        use std::fmt::Write;
        use std::collections;

        // Underscores avoid the unused_mut warning
        let mut _names: Vec<&str> = vec![];
        let mut _properties: collections::BTreeMap<&'static str, String> = collections::BTreeMap::new();

        $(
            _names.push(stringify!($n));
            _properties.insert(stringify!($n), $crate::message_templates::capture(&$v));            
        )*
        
        _properties.insert("target", $crate::message_templates::capture(&$target));
        
        let template = $crate::message_templates::build_template($s, &_names);
                
        (template, _properties)
    }};
}

#[macro_export]
macro_rules! emit {
    ( target: $target:expr, $s:expr, $( $n:ident: $v:expr ),* ) => {{
        use log::LogLevel;        
        log!(target: $target, LogLevel::Info, $s, $($v),*);
        let (template, properties) = __emit_get_event_data!($target, $s, $($n: $v),*);
        $crate::pipeline::emit(&template, &properties);
    }};
    
    ( target: $target:expr, $s:expr ) => {{
        emit!(target: $target, $s,);
    }};
    
    ( $s:expr, $( $n:ident: $v:expr ),* ) => {{
        emit!(target: module_path!(), $s, $($n: $v),*);
    }};
    
    ( $s:expr ) => {{
        emit!(target: module_path!(), $s,);
    }};
}

#[cfg(test)]
mod tests {
    use pipeline;
    use std::env;
    
    #[test]
    fn unparameterized_templates_are_captured() {
        let (template, properties) = __emit_get_event_data!("t", "Starting...",);
        assert!(template == "Starting...");
        assert!(properties.len() == 1);
    }
    
    #[test]
    fn template_and_properties_are_captured() {
        let u = "nblumhardt";
        let q = 42;
        
        let (template, properties) = __emit_get_event_data!("t", "User {} exceeded quota of {}!", user: u, quota: q);
        assert!(template == "User {user} exceeded quota of {quota}!");
        assert!(properties.get("user") == Some(&"\"nblumhardt\"".to_owned()));
        assert!(properties.get("quota") == Some(&"42".to_owned()));
        assert!(properties.len() == 3);
    }    

    #[test]
    pub fn emitted_events_are_flushed() {
        let _flush = pipeline::init("http://localhost:5341/", None);
        emit!("Hello, {}!", name: env::var("USERNAME").unwrap());
    }
}
