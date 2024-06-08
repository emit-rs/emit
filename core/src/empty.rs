/*!
The [`Empty`] type.

An [`Empty`] can be used as a default in place of a more meaningful implementation of most traits. For example, for [`crate::props::Props`], it behaves like an empty set, for [`crate::emitter::Emitter`]s, it discards emitted events, and for [`crate::filter::Filter`]s, it always evaluates to `true`.
*/

/**
A type that behaves like a default, empty, null value.
*/
#[derive(Default, Debug, Clone, Copy)]
pub struct Empty;
