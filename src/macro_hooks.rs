#![allow(missing_docs)]

use core::{any::Any, fmt, ops::ControlFlow};

use emit_core::{
    and::And,
    clock::Clock,
    ctxt::Ctxt,
    emitter::Emitter,
    event::ToEvent,
    extent::ToExtent,
    filter::Filter,
    path::Path,
    props::Props,
    rng::Rng,
    runtime::Runtime,
    str::{Str, ToStr},
    template::{Formatter, Part, Template},
    value::{ToValue, Value},
    well_known::{KEY_ERR, KEY_LVL},
};

use emit_core::{empty::Empty, event::Event};

use crate::{frame::Frame, span::Span};

#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    span::{self, Completion, SpanGuard, SpanId, TraceId},
    Level,
};

#[diagnostic::on_unimplemented(
    message = "capturing requires `{Self}` implements `Display + Any` by default. If this value does implement `Display`, then dereference or annotate it with `#[emit::as_display]`. If it doesn't, then use one of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureWithDefault {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureWithDefault for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::capture_display(self))
    }
}

impl CaptureWithDefault for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_display(inspect: true)]` requires `{Self}` implements `Display + 'static`. If this value does implement `Display`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsDisplay {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureAsDisplay for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::capture_display(self))
    }
}

impl CaptureAsDisplay for dyn fmt::Display {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureAsDisplay for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_display]` requires `{Self}` implements `Display`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonDisplay {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureAsAnonDisplay for T
where
    T: fmt::Display,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::from_display(self))
    }
}

impl CaptureAsAnonDisplay for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_debug(inspect: true)]` requires `{Self}` implements `Debug + 'static`. If this value does implement `Debug`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsDebug {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureAsDebug for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::capture_debug(self))
    }
}

impl CaptureAsDebug for dyn fmt::Debug {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureAsDebug for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_debug]` requires `{Self}` implements `Debug`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonDebug {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureAsAnonDebug for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::from_debug(self))
    }
}

impl CaptureAsAnonDebug for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_value(inspect: true)]` requires `{Self}` implements `ToValue + 'static`. If this value does implement `ToValue`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsValue {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureAsValue for T
where
    T: ToValue + Any,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureAsValue for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_value]` requires `{Self}` implements `ToValue`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonValue {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<T> CaptureAsAnonValue for T
where
    T: ToValue,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureAsAnonValue for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_sval(inspect: true)]` requires `{Self}` implements `Value + 'static`. If this value does implement `Value`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsSval {
    fn capture(&self) -> Option<Value<'_>>;
}

#[cfg(feature = "sval")]
impl<T> CaptureAsSval for T
where
    T: sval::Value + Any,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::capture_sval(self))
    }
}

impl CaptureAsSval for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_sval]` requires `{Self}` implements `Value`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonSval {
    fn capture(&self) -> Option<Value<'_>>;
}

#[cfg(feature = "sval")]
impl<T> CaptureAsAnonSval for T
where
    T: sval::Value,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::from_sval(self))
    }
}

impl CaptureAsAnonSval for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_serde(inspect: true)]` requires `{Self}` implements `Serialize + 'static`. If this value does implement `Serialize`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsSerde {
    fn capture(&self) -> Option<Value<'_>>;
}

#[cfg(feature = "serde")]
impl<T> CaptureAsSerde for T
where
    T: serde::Serialize + Any,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::capture_serde(self))
    }
}

impl CaptureAsSerde for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_serde]` requires `{Self}` implements `Serialize`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonSerde {
    fn capture(&self) -> Option<Value<'_>>;
}

#[cfg(feature = "serde")]
impl<T> CaptureAsAnonSerde for T
where
    T: serde::Serialize,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::from_serde(self))
    }
}

impl CaptureAsAnonSerde for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing an error requires a `str` or that `{Self}` implements `Error + 'static`."
)]
pub trait CaptureAsError {
    fn capture(&self) -> Option<Value<'_>>;
}

#[cfg(feature = "std")]
impl<T> CaptureAsError for T
where
    T: Error + 'static,
{
    fn capture(&self) -> Option<Value<'_>> {
        Some(Value::capture_error(self))
    }
}

#[cfg(feature = "std")]
impl CaptureAsError for (dyn Error + 'static) {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureAsError for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing a span id requires a `str`, `u64`, or `SpanId`."
)]
pub trait CaptureSpanId {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<'a, T: CaptureSpanId + ?Sized> CaptureSpanId for &'a T {
    fn capture(&self) -> Option<Value<'_>> {
        (**self).capture()
    }
}

impl CaptureSpanId for SpanId {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureSpanId for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureSpanId for u64 {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl<T: CaptureSpanId> CaptureSpanId for Option<T> {
    fn capture(&self) -> Option<Value<'_>> {
        self.as_ref().and_then(|v| v.capture())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing a trace id requires a `str`, `u128`, or `TraceId`."
)]
pub trait CaptureTraceId {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<'a, T: CaptureTraceId + ?Sized> CaptureTraceId for &'a T {
    fn capture(&self) -> Option<Value<'_>> {
        (**self).capture()
    }
}

impl CaptureTraceId for TraceId {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureTraceId for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureTraceId for u128 {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl<T: CaptureTraceId> CaptureTraceId for Option<T> {
    fn capture(&self) -> Option<Value<'_>> {
        self.as_ref().and_then(|v| v.capture())
    }
}

#[diagnostic::on_unimplemented(message = "capturing a level requires a `str` or `Level`.")]
pub trait CaptureLevel {
    fn capture(&self) -> Option<Value<'_>>;
}

impl<'a, T: CaptureLevel + ?Sized> CaptureLevel for &'a T {
    fn capture(&self) -> Option<Value<'_>> {
        (**self).capture()
    }
}

impl CaptureLevel for Level {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl CaptureLevel for str {
    fn capture(&self) -> Option<Value<'_>> {
        Some(self.to_value())
    }
}

impl<T: CaptureLevel> CaptureLevel for Option<T> {
    fn capture(&self) -> Option<Value<'_>> {
        self.as_ref().and_then(|v| v.capture())
    }
}

pub trait __PrivateOptionalCaptureHook {
    fn __private_optional_capture_some(&self) -> Option<&Self> {
        Some(self)
    }

    fn __private_optional_capture_option_ref(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl<T: ?Sized> __PrivateOptionalCaptureHook for T {}

#[diagnostic::on_unimplemented(
    message = "capturing an optional value requires `Option<&T>`. Try calling `.as_ref()`."
)]
pub trait Optional<'a> {
    type Value: ?Sized + 'a;

    fn into_option(self) -> Option<&'a Self::Value>;
}

impl<'a, T: ?Sized> Optional<'a> for Option<&'a T> {
    type Value = T;

    fn into_option(self) -> Option<&'a T> {
        self
    }
}

pub trait __PrivateOptionalMapHook<'a> {
    fn __private_optional_map_some<
        F: FnOnce(&'a <Self as Optional<'a>>::Value) -> Option<U>,
        U: 'a,
    >(
        self,
        map: F,
    ) -> Option<U>
    where
        Self: Optional<'a>;

    fn __private_optional_map_option_ref<
        F: FnOnce(&'a <Self as Optional<'a>>::Value) -> Option<U>,
        U: 'a,
    >(
        self,
        map: F,
    ) -> Option<U>
    where
        Self: Optional<'a>;
}

impl<'a, T> __PrivateOptionalMapHook<'a> for T {
    fn __private_optional_map_some<F: FnOnce(&'a <Self as Optional<'a>>::Value) -> Option<U>, U>(
        self,
        map: F,
    ) -> Option<U>
    where
        Self: Optional<'a>,
    {
        self.into_option().and_then(map)
    }

    fn __private_optional_map_option_ref<
        F: FnOnce(&'a <Self as Optional<'a>>::Value) -> Option<U>,
        U,
    >(
        self,
        map: F,
    ) -> Option<U>
    where
        Self: Optional<'a>,
    {
        self.into_option().and_then(map)
    }
}

pub trait __PrivateInterpolatedHook {
    fn __private_interpolated(self) -> Self;
    fn __private_uninterpolated(self) -> Self;

    fn __private_captured(self) -> Self;
    fn __private_uncaptured(self) -> Self;
}

impl<T> __PrivateInterpolatedHook for T {
    fn __private_interpolated(self) -> Self {
        self
    }

    fn __private_uninterpolated(self) -> Self {
        self
    }

    fn __private_captured(self) -> Self {
        self
    }

    fn __private_uncaptured(self) -> Self {
        self
    }
}

/**
An API to the specialized `Capture` trait for consuming in a macro.

This trait is a bit weird looking. It's shaped to serve a few purposes
in the private macro API:

- It supports auto-ref so that something like a `u64` or `&str` can be
captured using the same `x.method()` syntax.
- It uses `Self` bounds on each method, and is unconditionally implemented
so that when a bound isn't satisfied we get a more accurate type error.
- It uses clumsily uglified names that are unlikely to clash in non-hygienic
contexts. (We're expecting non-hygienic spans to support value interpolation).
*/
pub trait __PrivateCaptureHook {
    fn __private_capture_as_default(&self) -> Option<Value<'_>>
    where
        Self: CaptureWithDefault,
    {
        CaptureWithDefault::capture(self)
    }

    fn __private_capture_as_display(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsDisplay,
    {
        CaptureAsDisplay::capture(self)
    }

    fn __private_capture_anon_as_display(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsAnonDisplay,
    {
        CaptureAsAnonDisplay::capture(self)
    }

    fn __private_capture_as_debug(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsDebug,
    {
        CaptureAsDebug::capture(self)
    }

    fn __private_capture_anon_as_debug(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsAnonDebug,
    {
        CaptureAsAnonDebug::capture(self)
    }

    fn __private_capture_as_value(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsValue,
    {
        CaptureAsValue::capture(self)
    }

    fn __private_capture_anon_as_value(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsAnonValue,
    {
        CaptureAsAnonValue::capture(self)
    }

    fn __private_capture_as_sval(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsSval,
    {
        CaptureAsSval::capture(self)
    }

    fn __private_capture_anon_as_sval(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsAnonSval,
    {
        CaptureAsAnonSval::capture(self)
    }

    fn __private_capture_as_serde(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsSerde,
    {
        CaptureAsSerde::capture(self)
    }

    fn __private_capture_anon_as_serde(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsAnonSerde,
    {
        CaptureAsAnonSerde::capture(self)
    }

    fn __private_capture_as_error(&self) -> Option<Value<'_>>
    where
        Self: CaptureAsError,
    {
        CaptureAsError::capture(self)
    }

    fn __private_capture_as_level(&self) -> Option<Value<'_>>
    where
        Self: CaptureLevel,
    {
        CaptureLevel::capture(self)
    }

    fn __private_capture_as_span_id(&self) -> Option<Value<'_>>
    where
        Self: CaptureSpanId,
    {
        CaptureSpanId::capture(self)
    }

    fn __private_capture_as_trace_id(&self) -> Option<Value<'_>>
    where
        Self: CaptureTraceId,
    {
        CaptureTraceId::capture(self)
    }
}

impl<T: ?Sized> __PrivateCaptureHook for T {}

pub trait __PrivateFmtHook<'a> {
    fn __private_fmt_as_default(self) -> Self;
    fn __private_fmt_as(self, formatter: Formatter) -> Self;
}

impl<'a> __PrivateFmtHook<'a> for Part<'a> {
    fn __private_fmt_as_default(self) -> Self {
        self
    }

    fn __private_fmt_as(self, formatter: Formatter) -> Self {
        self.with_formatter(formatter)
    }
}

pub struct Key(pub &'static str);

pub trait __PrivateKeyHook {
    fn __private_key_as_default(self) -> Str<'static>;
    fn __private_key_as(self, key: &'static str) -> Str<'static>;
}

impl<'a> __PrivateKeyHook for Key {
    fn __private_key_as_default(self) -> Str<'static> {
        Str::new(self.0)
    }

    fn __private_key_as(self, key: &'static str) -> Str<'static> {
        Str::new(key)
    }
}

// Work-around for const-fn in traits
// Mirrors trait fns in `macro_hooks`
#[doc(hidden)]
impl Key {
    pub const fn __private_key_as_default(self) -> Str<'static> {
        Str::new(self.0)
    }

    pub const fn __private_key_as(self, key: &'static str) -> Str<'static> {
        Str::new(key)
    }
}

#[track_caller]
#[cfg(feature = "alloc")]
pub fn __private_format(tpl: Template, props: impl Props) -> alloc::string::String {
    let mut s = alloc::string::String::new();
    tpl.render(props).write(&mut s).expect("infallible write");

    s
}

struct FirstDefined<A, B>(Option<A>, B);

impl<A: Filter, B: Filter> Filter for FirstDefined<A, B> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        let evt = evt.to_event();

        if let Some(ref first) = self.0 {
            return first.matches(evt);
        }

        self.1.matches(evt)
    }
}

pub trait MdlControlParam {
    fn mdl_control_param(&self) -> Path<'_>;
}

impl<'a, T: MdlControlParam + ?Sized> MdlControlParam for &'a T {
    fn mdl_control_param(&self) -> Path<'_> {
        (**self).mdl_control_param()
    }
}

impl<'a> MdlControlParam for Path<'a> {
    fn mdl_control_param(&self) -> Path<'_> {
        self.by_ref()
    }
}

pub trait TplControlParam {
    fn tpl_control_param(&self) -> Template<'_>;
}

impl<'a, T: TplControlParam + ?Sized> TplControlParam for &'a T {
    fn tpl_control_param(&self) -> Template<'_> {
        (**self).tpl_control_param()
    }
}

impl<'a> TplControlParam for Template<'a> {
    fn tpl_control_param(&self) -> Template<'_> {
        self.by_ref()
    }
}

#[track_caller]
pub fn __private_emit<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    mdl: &'b (impl MdlControlParam + ?Sized),
    when: Option<&'b (impl Filter + ?Sized)>,
    extent: &'b (impl ToExtent + ?Sized),
    tpl: &'b (impl TplControlParam + ?Sized),
    base_props: &'b (impl Props + ?Sized),
    props: &'b (impl Props + ?Sized),
) {
    emit_core::emit(
        rt.emitter(),
        FirstDefined(when, rt.filter()),
        rt.ctxt(),
        rt.clock(),
        Event::new(
            mdl.mdl_control_param(),
            tpl.tpl_control_param(),
            extent,
            props.and_props(base_props),
        ),
    );
}

#[track_caller]
pub fn __private_emit_event<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    when: Option<&'b (impl Filter + ?Sized)>,
    event: &'b (impl ToEvent + ?Sized),
    tpl: Option<&'b (impl TplControlParam + ?Sized)>,
    props: &'b (impl Props + ?Sized),
) {
    let mut event = event.to_event();

    if let Some(tpl) = tpl {
        event = event.with_tpl(tpl.tpl_control_param());
    }

    let event = event.map_props(|event_props| props.and_props(event_props));

    emit_core::emit(
        rt.emitter(),
        FirstDefined(when, rt.filter()),
        rt.ctxt(),
        rt.clock(),
        event,
    );
}

#[track_caller]
pub fn __private_evt<'a, B: Props + ?Sized, P: Props>(
    mdl: impl Into<Path<'a>>,
    tpl: impl Into<Template<'a>>,
    extent: impl ToExtent,
    base_props: &'a B,
    props: P,
) -> Event<'a, And<P, &'a B>> {
    Event::new(
        mdl.into(),
        tpl.into(),
        extent.to_extent(),
        props.and_props(base_props),
    )
}

#[track_caller]
pub fn __private_begin_span<
    'a,
    'b,
    E: Emitter,
    F: Filter,
    C: Ctxt,
    T: Clock,
    R: Rng,
    S: Completion,
>(
    rt: &'a Runtime<E, F, C, T, R>,
    mdl: impl Into<Path<'static>>,
    name: impl Into<Str<'static>>,
    lvl: Option<&'b (impl CaptureLevel + ?Sized)>,
    when: Option<&'b (impl Filter + ?Sized)>,
    span_ctxt_props: &'b (impl Props + ?Sized),
    default_complete: S,
) -> (SpanGuard<'static, &'a T, Empty, S>, Frame<&'a C>) {
    let mdl = mdl.into();
    let name = name.into();

    SpanGuard::new(
        __PrivateBeginSpanFilter { rt, when, lvl },
        rt.ctxt(),
        rt.clock(),
        rt.rng(),
        default_complete,
        span_ctxt_props,
        mdl,
        name,
        Empty,
    )
}

pub struct __PrivateBeginSpanFilter<'a, 'b, E, F, C, T, R, W: ?Sized, CL: ?Sized> {
    rt: &'a Runtime<E, F, C, T, R>,
    when: Option<&'b W>,
    lvl: Option<&'b CL>,
}

impl<'a, 'b, E, F: Filter, C, T, R, W: Filter + ?Sized, CL: CaptureLevel + ?Sized> Filter
    for __PrivateBeginSpanFilter<'a, 'b, E, F, C, T, R, W, CL>
{
    fn matches<ET: ToEvent>(&self, evt: ET) -> bool {
        let evt = evt.to_event();

        let lvl_prop = self
            .lvl
            .and_then(|lvl| lvl.capture())
            .map(|lvl| (KEY_LVL, lvl));

        FirstDefined(self.when, self.rt.filter())
            .matches(evt.map_props(|props| props.and_props(&lvl_prop)))
    }
}

pub fn __private_complete_span<'a, 'b, E, F, C, T, R, CL: ?Sized, CLP: ?Sized>(
    rt: &'a Runtime<E, F, C, T, R>,
    tpl: impl Into<Template<'b>>,
    lvl: Option<&'b CL>,
    panic_lvl: Option<&'b CLP>,
) -> __PrivateCompleteSpan<'a, 'b, E, F, C, T, R, CL, CLP> {
    __PrivateCompleteSpan {
        rt,
        tpl: tpl.into(),
        lvl,
        panic_lvl,
    }
}

pub struct __PrivateCompleteSpan<'a, 'b, E, F, C, T, R, CL: ?Sized, CLP: ?Sized> {
    rt: &'a Runtime<E, F, C, T, R>,
    tpl: Template<'b>,
    lvl: Option<&'b CL>,
    panic_lvl: Option<&'b CLP>,
}

impl<'a, 'b, E, F, C, T, R, CL, CLP> crate::span::completion::Completion
    for __PrivateCompleteSpan<'a, 'b, E, F, C, T, R, CL, CLP>
where
    E: Emitter,
    F: Filter,
    C: Ctxt,
    T: Clock,
    R: Rng,
    CL: CaptureLevel + ?Sized,
    CLP: CaptureLevel + ?Sized,
{
    #[track_caller]
    fn complete<P: Props>(&self, span: Span<P>) {
        let mut completion = span::completion::Default::new(self.rt.emitter(), self.rt.ctxt())
            .with_tpl(self.tpl.by_ref());

        if let Some(lvl) = self.lvl.and_then(|lvl| lvl.capture()) {
            completion = completion.with_lvl(lvl);
        }

        if let Some(lvl) = self.panic_lvl.and_then(|lvl| lvl.capture()) {
            completion = completion.with_panic_lvl(lvl);
        }

        completion.complete(span);
    }
}

pub fn __private_complete_span_ok<'a, 'b, E, F, C, T, R, CL: ?Sized>(
    rt: &'a Runtime<E, F, C, T, R>,
    tpl: impl Into<Template<'b>>,
    lvl: Option<&'b CL>,
) -> __PrivateCompleteSpanOk<'a, 'b, E, F, C, T, R, CL> {
    __PrivateCompleteSpanOk {
        rt,
        tpl: tpl.into(),
        lvl,
    }
}

pub struct __PrivateCompleteSpanOk<'a, 'b, E, F, C, T, R, CL: ?Sized> {
    rt: &'a Runtime<E, F, C, T, R>,
    tpl: Template<'b>,
    lvl: Option<&'b CL>,
}

impl<'a, 'b, E, F, C, T, R, CL> crate::span::completion::Completion
    for __PrivateCompleteSpanOk<'a, 'b, E, F, C, T, R, CL>
where
    E: Emitter,
    F: Filter,
    C: Ctxt,
    T: Clock,
    R: Rng,
    CL: CaptureLevel + ?Sized,
{
    #[track_caller]
    fn complete<P: Props>(&self, span: Span<P>) {
        let lvl_prop = self
            .lvl
            .and_then(|lvl| lvl.capture())
            .map(|lvl| (KEY_LVL, lvl));

        emit_core::emit(
            self.rt.emitter(),
            crate::Empty,
            self.rt.ctxt(),
            self.rt.clock(),
            span.to_event()
                .with_tpl(self.tpl.by_ref())
                .map_props(|span_props| lvl_prop.and_props(span_props)),
        );
    }
}

pub fn __private_complete_span_err<'a, 'b, E, F, C, T, R, CL: ?Sized, CE: ?Sized>(
    rt: &'a Runtime<E, F, C, T, R>,
    tpl: impl Into<Template<'b>>,
    lvl: &'b CL,
    err: &'b CE,
) -> __PrivateCompleteSpanErr<'a, 'b, E, F, C, T, R, CL, CE> {
    __PrivateCompleteSpanErr {
        rt,
        tpl: tpl.into(),
        lvl,
        err,
    }
}

pub struct __PrivateCompleteSpanErr<'a, 'b, E, F, C, T, R, CL: ?Sized, CE: ?Sized> {
    rt: &'a Runtime<E, F, C, T, R>,
    tpl: Template<'b>,
    lvl: &'b CL,
    err: &'b CE,
}

impl<'a, 'b, E, F, C, T, R, CL, CE> crate::span::completion::Completion
    for __PrivateCompleteSpanErr<'a, 'b, E, F, C, T, R, CL, CE>
where
    E: Emitter,
    F: Filter,
    C: Ctxt,
    T: Clock,
    R: Rng,
    CL: CaptureLevel + ?Sized,
    CE: CaptureAsError + ?Sized,
{
    #[track_caller]
    fn complete<P: Props>(&self, span: Span<P>) {
        let lvl_prop = self.lvl.capture().map(|lvl| (KEY_LVL, lvl));
        let err_prop = self.err.capture().map(|err| (KEY_ERR, err));

        emit_core::emit(
            self.rt.emitter(),
            crate::Empty,
            self.rt.ctxt(),
            self.rt.clock(),
            span.to_event()
                .with_tpl(self.tpl.by_ref())
                .map_props(|span_props| [lvl_prop, err_prop].and_props(span_props)),
        );
    }
}

#[repr(transparent)]
pub struct __PrivateMacroProps<'a, const N: usize>([(Str<'a>, Option<Value<'a>>); N]);

impl<'a, const N: usize> __PrivateMacroProps<'a, N> {
    pub fn from_array(props: [(Str<'a>, Option<Value<'a>>); N]) -> Self {
        __PrivateMacroProps(props)
    }
}

impl<'a, const N: usize> Props for __PrivateMacroProps<'a, N> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for kv in &self.0 {
            let k = &kv.0;

            if let Some(ref v) = kv.1 {
                for_each(k.by_ref(), v.by_ref())?;
            }
        }

        ControlFlow::Continue(())
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_str();

        self.0
            .binary_search_by(|(k, _)| k.cmp(&key))
            .ok()
            .and_then(|i| self.0[i].1.as_ref().map(|v| v.by_ref()))
    }

    fn is_unique(&self) -> bool {
        true
    }

    fn size(&self) -> Option<usize> {
        Some(self.0.len())
    }
}
