use std::fmt::Write;
use serde;
use serde_json;

pub fn build_template(s: &str, names: &[&str]) -> String {
    let mut template = "".to_owned();
    let mut next_name = 0;
    
    let mut first = true;
    for literal in s.split("{}") {
        if !first {
            if names.len() > next_name {
                write!(&mut template, "{{{}}}", names[next_name]).is_ok();                    
            } else {
                write!(&mut template, "{{{}}}", next_name).is_ok();
            }
            next_name += 1;            
        }

        template.push_str(literal);
        
        first = false;
    }
    
    template
}

pub fn capture<T: serde::ser::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

#[cfg(test)]
mod tests {
    use message_templates::build_template;
    
    #[test]
    fn templates_without_parameters_are_built() {
        let names: Vec<&str> = vec![];
        let s = "Hello, world!";
        
        let built = build_template(s, &names);
        
        assert_eq!(built, s);
    }

    #[test]
    fn templates_with_parameters_are_built() {
        let names = vec!["A", "B"];
        let s = "C {} D {} E";
        
        let built = build_template(s, &names);
        
        assert_eq!(built, "C {A} D {B} E");
    }

    #[test]
    fn additional_names_are_ignored() {
        let names = vec!["A", "B"];
        let s = "C {} D";
        
        let built = build_template(s, &names);
        
        assert_eq!(built, "C {A} D");
    }

    #[test]
    fn additional_holes_are_indexed() {
        let names = vec!["A"];
        let s = "C {} D {} E";
        
        let built = build_template(s, &names);
        
        assert_eq!(built, "C {A} D {1} E");
    }

    #[test]
    fn leading_holes_are_handled() {
        let names = vec!["A"];
        let s = "{} D";
        
        let built = build_template(s, &names);
        
        assert_eq!(built, "{A} D");
    }

    #[test]
    fn trailing_holes_are_handled() {
        let names = vec!["A"];
        let s = "C {}";
        
        let built = build_template(s, &names);
        
        assert_eq!(built, "C {A}");
    }

}
