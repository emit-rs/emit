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
    span::{SpanCtxt, SpanGuard, SpanId, TraceId},
    Level, Timer,
};

#[diagnostic::on_unimplemented(
    message = "capturing requires `{Self}` implements `Display + Any` by default. If this value does implement `Display`, then dereference or annotate it with `#[emit::as_display]`. If it doesn't, then use one of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureWithDefault {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureWithDefault for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_display(self))
    }
}

impl CaptureWithDefault for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_display(inspect: true)]` requires `{Self}` implements `Display + 'static`. If this value does implement `Display`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsDisplay {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureAsDisplay for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_display(self))
    }
}

impl CaptureAsDisplay for dyn fmt::Display {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureAsDisplay for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_display]` requires `{Self}` implements `Display`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonDisplay {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureAsAnonDisplay for T
where
    T: fmt::Display,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_display(self))
    }
}

impl CaptureAsAnonDisplay for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_debug(inspect: true)]` requires `{Self}` implements `Debug + 'static`. If this value does implement `Debug`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsDebug {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureAsDebug for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_debug(self))
    }
}

impl CaptureAsDebug for dyn fmt::Debug {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureAsDebug for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_debug]` requires `{Self}` implements `Debug`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonDebug {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureAsAnonDebug for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_debug(self))
    }
}

impl CaptureAsAnonDebug for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_value(inspect: true)]` requires `{Self}` implements `ToValue + 'static`. If this value does implement `ToValue`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsValue {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureAsValue for T
where
    T: ToValue + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureAsValue for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_value]` requires `{Self}` implements `ToValue`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonValue {
    fn capture(&self) -> Option<Value>;
}

impl<T> CaptureAsAnonValue for T
where
    T: ToValue,
{
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureAsAnonValue for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_sval(inspect: true)]` requires `{Self}` implements `Value + 'static`. If this value does implement `Value`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsSval {
    fn capture(&self) -> Option<Value>;
}

#[cfg(feature = "sval")]
impl<T> CaptureAsSval for T
where
    T: sval::Value + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_sval(self))
    }
}

impl CaptureAsSval for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_sval]` requires `{Self}` implements `Value`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonSval {
    fn capture(&self) -> Option<Value>;
}

#[cfg(feature = "sval")]
impl<T> CaptureAsAnonSval for T
where
    T: sval::Value,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_sval(self))
    }
}

impl CaptureAsAnonSval for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_serde(inspect: true)]` requires `{Self}` implements `Serialize + 'static`. If this value does implement `Serialize`, then dereference or remove the `inspect` argument. If it doesn't, then use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsSerde {
    fn capture(&self) -> Option<Value>;
}

#[cfg(feature = "serde")]
impl<T> CaptureAsSerde for T
where
    T: serde::Serialize + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_serde(self))
    }
}

impl CaptureAsSerde for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing with `#[emit::as_serde]` requires `{Self}` implements `Serialize`. Use another of the `#[emit::as_*]` attributes to capture this value using a trait it does implement."
)]
pub trait CaptureAsAnonSerde {
    fn capture(&self) -> Option<Value>;
}

#[cfg(feature = "serde")]
impl<T> CaptureAsAnonSerde for T
where
    T: serde::Serialize,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_serde(self))
    }
}

impl CaptureAsAnonSerde for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing an error requires a `str` or that `{Self}` implements `Error + 'static`."
)]
pub trait CaptureAsError {
    fn capture(&self) -> Option<Value>;
}

#[cfg(feature = "std")]
impl<T> CaptureAsError for T
where
    T: Error + 'static,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_error(self))
    }
}

#[cfg(feature = "std")]
impl CaptureAsError for (dyn Error + 'static) {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureAsError for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing a span id requires a `str`, `u64`, or `SpanId`."
)]
pub trait CaptureSpanId {
    fn capture(&self) -> Option<Value>;
}

impl<'a, T: CaptureSpanId + ?Sized> CaptureSpanId for &'a T {
    fn capture(&self) -> Option<Value> {
        (**self).capture()
    }
}

impl CaptureSpanId for SpanId {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureSpanId for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureSpanId for u64 {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: CaptureSpanId> CaptureSpanId for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

#[diagnostic::on_unimplemented(
    message = "capturing a trace id requires a `str`, `u128`, or `TraceId`."
)]
pub trait CaptureTraceId {
    fn capture(&self) -> Option<Value>;
}

impl<'a, T: CaptureTraceId + ?Sized> CaptureTraceId for &'a T {
    fn capture(&self) -> Option<Value> {
        (**self).capture()
    }
}

impl CaptureTraceId for TraceId {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureTraceId for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureTraceId for u128 {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: CaptureTraceId> CaptureTraceId for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

#[diagnostic::on_unimplemented(message = "capturing a level requires a `str` or `Level`.")]
pub trait CaptureLevel {
    fn capture(&self) -> Option<Value>;
}

impl<'a, T: CaptureLevel + ?Sized> CaptureLevel for &'a T {
    fn capture(&self) -> Option<Value> {
        (**self).capture()
    }
}

impl CaptureLevel for Level {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl CaptureLevel for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: CaptureLevel> CaptureLevel for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

pub trait __PrivateOptionalCaptureHook {
    fn __private_optional_capture_some(&self) -> Option<&Self>;

    fn __private_optional_capture_option(&self) -> &Self;
}

impl<T: ?Sized> __PrivateOptionalCaptureHook for T {
    fn __private_optional_capture_some(&self) -> Option<&Self> {
        Some(self)
    }

    fn __private_optional_capture_option(&self) -> &Self {
        self
    }
}

pub trait __PrivateOptionalMapHook<T> {
    fn __private_optional_map_some<F: FnOnce(T) -> Option<U>, U>(self, map: F) -> Option<U>;

    fn __private_optional_map_option<'a, F: FnOnce(&'a T) -> Option<U>, U: 'a>(
        &'a self,
        map: F,
    ) -> Option<U>
    where
        T: 'a;
}

impl<T> __PrivateOptionalMapHook<T> for Option<T> {
    fn __private_optional_map_some<F: FnOnce(T) -> Option<U>, U>(self, map: F) -> Option<U> {
        self.and_then(map)
    }

    fn __private_optional_map_option<'a, F: FnOnce(&'a T) -> Option<U>, U: 'a>(
        &'a self,
        map: F,
    ) -> Option<U> {
        self.as_ref().and_then(map)
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
    fn __private_capture_as_default(&self) -> Option<Value>
    where
        Self: CaptureWithDefault,
    {
        CaptureWithDefault::capture(self)
    }

    fn __private_capture_as_display(&self) -> Option<Value>
    where
        Self: CaptureAsDisplay,
    {
        CaptureAsDisplay::capture(self)
    }

    fn __private_capture_anon_as_display(&self) -> Option<Value>
    where
        Self: CaptureAsAnonDisplay,
    {
        CaptureAsAnonDisplay::capture(self)
    }

    fn __private_capture_as_debug(&self) -> Option<Value>
    where
        Self: CaptureAsDebug,
    {
        CaptureAsDebug::capture(self)
    }

    fn __private_capture_anon_as_debug(&self) -> Option<Value>
    where
        Self: CaptureAsAnonDebug,
    {
        CaptureAsAnonDebug::capture(self)
    }

    fn __private_capture_as_value(&self) -> Option<Value>
    where
        Self: CaptureAsValue,
    {
        CaptureAsValue::capture(self)
    }

    fn __private_capture_anon_as_value(&self) -> Option<Value>
    where
        Self: CaptureAsAnonValue,
    {
        CaptureAsAnonValue::capture(self)
    }

    fn __private_capture_as_sval(&self) -> Option<Value>
    where
        Self: CaptureAsSval,
    {
        CaptureAsSval::capture(self)
    }

    fn __private_capture_anon_as_sval(&self) -> Option<Value>
    where
        Self: CaptureAsAnonSval,
    {
        CaptureAsAnonSval::capture(self)
    }

    fn __private_capture_as_serde(&self) -> Option<Value>
    where
        Self: CaptureAsSerde,
    {
        CaptureAsSerde::capture(self)
    }

    fn __private_capture_anon_as_serde(&self) -> Option<Value>
    where
        Self: CaptureAsAnonSerde,
    {
        CaptureAsAnonSerde::capture(self)
    }

    fn __private_capture_as_error(&self) -> Option<Value>
    where
        Self: CaptureAsError,
    {
        CaptureAsError::capture(self)
    }

    fn __private_capture_as_level(&self) -> Option<Value>
    where
        Self: CaptureLevel,
    {
        CaptureLevel::capture(self)
    }

    fn __private_capture_as_span_id(&self) -> Option<Value>
    where
        Self: CaptureSpanId,
    {
        CaptureSpanId::capture(self)
    }

    fn __private_capture_as_trace_id(&self) -> Option<Value>
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

pub trait __PrivateKeyHook {
    fn __private_key_as_default(self) -> Self;
    fn __private_key_as_static(self, key: &'static str) -> Self;
    fn __private_key_as<K: Into<Self>>(self, key: K) -> Self
    where
        Self: Sized;
}

impl<'a> __PrivateKeyHook for Str<'a> {
    fn __private_key_as_default(self) -> Self {
        self
    }

    fn __private_key_as_static(self, key: &'static str) -> Self {
        Str::new(key)
    }

    fn __private_key_as<K: Into<Self>>(self, key: K) -> Self {
        key.into()
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
    fn mdl_control_param(&self) -> Path;
}

impl<'a, T: MdlControlParam + ?Sized> MdlControlParam for &'a T {
    fn mdl_control_param(&self) -> Path {
        (**self).mdl_control_param()
    }
}

impl<'a> MdlControlParam for Path<'a> {
    fn mdl_control_param(&self) -> Path {
        self.by_ref()
    }
}

pub trait TplControlParam {
    fn tpl_control_param(&self) -> Template;
}

impl<'a, T: TplControlParam + ?Sized> TplControlParam for &'a T {
    fn tpl_control_param(&self) -> Template {
        (**self).tpl_control_param()
    }
}

impl<'a> TplControlParam for Template<'a> {
    fn tpl_control_param(&self) -> Template {
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
pub fn __private_evt<'a, B: Props + ?Sized, P: Props + ?Sized>(
    mdl: &'a (impl MdlControlParam + ?Sized),
    tpl: &'a (impl TplControlParam + ?Sized),
    extent: &'a (impl ToExtent + ?Sized),
    base_props: &'a B,
    props: &'a P,
) -> Event<'a, And<&'a P, &'a B>> {
    Event::new(
        mdl.mdl_control_param(),
        tpl.tpl_control_param(),
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
    S: FnOnce(Span<'static, Empty>),
>(
    rt: &'a Runtime<E, F, C, T, R>,
    mdl: impl Into<Path<'static>>,
    name: impl Into<Str<'static>>,
    tpl: &'b (impl TplControlParam + ?Sized),
    lvl: Option<&'b (impl CaptureLevel + ?Sized)>,
    when: Option<&'b (impl Filter + ?Sized)>,
    span_ctxt_props: &'b (impl Props + ?Sized),
    default_complete: S,
) -> (Frame<&'a C>, SpanGuard<'static, &'a T, Empty, S>) {
    let mdl = mdl.into();
    let name = name.into();
    let tpl = tpl.tpl_control_param();
    let lvl_prop = lvl.and_then(|lvl| lvl.capture()).map(|lvl| (KEY_LVL, lvl));

    let mut span = SpanGuard::filtered_new(
        |span_ctxt, span| {
            rt.ctxt().with_current(|ctxt_props| {
                FirstDefined(when, rt.filter()).matches(&span.to_event().with_tpl(tpl).map_props(
                    |span_props| {
                        lvl_prop
                            .and_props(span_props)
                            .and_props(&span_ctxt_props)
                            .and_props(&span_ctxt)
                            .and_props(ctxt_props)
                    },
                ))
            })
        },
        mdl,
        Timer::start(rt.clock()),
        name,
        // NOTE: We could avoid constructing a context if `span_ctxt_props`
        // already carries trace/span ids
        SpanCtxt::current(rt.ctxt()).new_child(rt.rng()),
        Empty,
        default_complete,
    );

    let frame = span.push_ctxt(rt.ctxt(), span_ctxt_props);

    (frame, span)
}

#[track_caller]
pub fn __private_complete_span<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    span: Span<'static, Empty>,
    tpl: &'b (impl TplControlParam + ?Sized),
    lvl: Option<&'b (impl CaptureLevel + ?Sized)>,
    panic_lvl: Option<&'b (impl CaptureLevel + ?Sized)>,
) {
    let completion_props = if panic_lvl.is_some() && is_panicking() {
        [
            panic_lvl.unwrap().capture().map(|lvl| (KEY_LVL, lvl)),
            Some((KEY_ERR, Value::from_any(&PanicError))),
        ]
    } else {
        [
            lvl.and_then(|lvl| lvl.capture()).map(|lvl| (KEY_LVL, lvl)),
            None,
        ]
    };

    emit_core::emit(
        rt.emitter(),
        crate::Empty,
        rt.ctxt(),
        rt.clock(),
        span.to_event()
            .with_tpl(tpl.tpl_control_param())
            .map_props(|span_props| completion_props.and_props(span_props)),
    );
}

struct PanicError;

impl fmt::Debug for PanicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for PanicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "panicked")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PanicError {}

impl ToValue for PanicError {
    fn to_value(&self) -> Value {
        #[cfg(feature = "std")]
        {
            Value::capture_error(self)
        }
        #[cfg(not(feature = "std"))]
        {
            Value::capture_display(self)
        }
    }
}

fn is_panicking() -> bool {
    #[cfg(feature = "std")]
    {
        std::thread::panicking()
    }
    #[cfg(not(feature = "std"))]
    {
        false
    }
}

#[track_caller]
pub fn __private_complete_span_ok<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    span: Span<'static, Empty>,
    tpl: &'b (impl TplControlParam + ?Sized),
    lvl: Option<&'b (impl CaptureLevel + ?Sized)>,
) {
    let lvl_prop = lvl.and_then(|lvl| lvl.capture()).map(|lvl| (KEY_LVL, lvl));

    emit_core::emit(
        rt.emitter(),
        crate::Empty,
        rt.ctxt(),
        rt.clock(),
        span.to_event()
            .with_tpl(tpl.tpl_control_param())
            .map_props(|span_props| lvl_prop.and_props(span_props)),
    );
}

#[track_caller]
pub fn __private_complete_span_err<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    span: Span<'static, Empty>,
    tpl: &'b (impl TplControlParam + ?Sized),
    lvl: &'b (impl CaptureLevel + ?Sized),
    err: &'b (impl CaptureAsError + ?Sized),
) {
    let lvl_prop = lvl.capture().map(|lvl| (KEY_LVL, lvl));
    let err_prop = err.capture().map(|err| (KEY_ERR, err));

    emit_core::emit(
        rt.emitter(),
        crate::Empty,
        rt.ctxt(),
        rt.clock(),
        span.to_event()
            .with_tpl(tpl.tpl_control_param())
            .map_props(|span_props| [lvl_prop, err_prop].and_props(span_props)),
    );
}

#[repr(transparent)]
pub struct __PrivateMacroProps<'a>([(Str<'a>, Option<Value<'a>>)]);

impl __PrivateMacroProps<'static> {
    pub fn new(props: &'static [(Str<'static>, Option<Value<'static>>)]) -> &'static Self {
        Self::new_ref(props)
    }
}

impl<'a> __PrivateMacroProps<'a> {
    pub fn new_ref<'b>(props: &'b [(Str<'a>, Option<Value<'a>>)]) -> &'b Self {
        // SAFETY: `__PrivateMacroProps` and the array have the same ABI
        unsafe {
            &*(props as *const [(Str<'a>, Option<Value<'a>>)] as *const __PrivateMacroProps<'a>)
        }
    }
}

impl<'a> Props for __PrivateMacroProps<'a> {
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
}
