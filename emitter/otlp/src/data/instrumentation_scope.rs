use sval_derive::Value;

#[derive(Value)]
pub struct InstrumentationScope<'a, N: ?Sized = str> {
    #[sval(label = "name", index = 1)]
    pub name: &'a N,
}
