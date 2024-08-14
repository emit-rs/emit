use emit::Props;

#[test]
fn event_basic() {
    match emit::event!(
        "Hello, {user}",
        user: "Rust",
    ) {
        evt => {
            assert_eq!("Hello, Rust", evt.msg().to_string());
            assert_eq!("Hello, {user}", evt.tpl().to_string());
            assert_eq!(module_path!(), evt.mdl());

            assert!(evt.extent().is_none());

            assert_eq!("Rust", evt.props().pull::<&str, _>("user").unwrap());
        }
    }
}
