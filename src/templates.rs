use std::fmt::Write;

pub struct MessageTemplate {
    text: String
}

impl MessageTemplate {
    pub fn new<T: Into<String>>(text: T) -> MessageTemplate {
         MessageTemplate { text: text.into() }
    }
    
    pub fn from_format(s: &str, names: &[&str]) -> MessageTemplate {
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
        
       Self::new(template)
    }
    
    pub fn text(&self) -> &String {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use templates::MessageTemplate;
    
    #[test]
    fn templates_without_parameters_are_built() {
        let names: Vec<&str> = vec![];
        let s = "Hello, world!";
        
        let built = MessageTemplate::from_format(s, &names);
        
        assert_eq!(built.text(), s);
    }

    #[test]
    fn templates_with_parameters_are_built() {
        let names = vec!["A", "B"];
        let s = "C {} D {} E";
        
        let built = MessageTemplate::from_format(s, &names);
        
        assert_eq!(built.text(), "C {A} D {B} E");
    }

    #[test]
    fn additional_names_are_ignored() {
        let names = vec!["A", "B"];
        let s = "C {} D";
        
        let built = MessageTemplate::from_format(s, &names);
        
        assert_eq!(built.text(), "C {A} D");
    }

    #[test]
    fn additional_holes_are_indexed() {
        let names = vec!["A"];
        let s = "C {} D {} E";
        
        let built = MessageTemplate::from_format(s, &names);
        
        assert_eq!(built.text(), "C {A} D {1} E");
    }

    #[test]
    fn leading_holes_are_handled() {
        let names = vec!["A"];
        let s = "{} D";
        
        let built = MessageTemplate::from_format(s, &names);
        
        assert_eq!(built.text(), "{A} D");
    }

    #[test]
    fn trailing_holes_are_handled() {
        let names = vec!["A"];
        let s = "C {}";
        
        let built = MessageTemplate::from_format(s, &names);
        
        assert_eq!(built.text(), "C {A}");
    }

}
