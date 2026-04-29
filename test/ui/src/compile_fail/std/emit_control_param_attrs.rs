fn main() {
    emit::emit!(#[cfg(enabled)] mdl: emit::path!("mdl"), "template");
}
