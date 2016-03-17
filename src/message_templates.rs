use std::fmt::Write;

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
