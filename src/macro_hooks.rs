#![allow(missing_docs)]

use core::{any::Any, fmt, ops::ControlFlow};

use emit_core::{
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
    timestamp::Timestamp,
    value::{ToValue, Value},
};

use emit_core::{empty::Empty, event::Event};

use crate::{frame::Frame, span::SpanEvent};

#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    span::{Span, SpanCtxt, SpanId, TraceId},
    Level, Timer,
};

/**
Similar to `log`'s `ToValue` trait, but with a generic pivot parameter.

The parameter lets us choose the way values are captured, since many
types can be captured in lots of different ways.
*/
pub trait Capture<T: ?Sized> {
    fn capture(&self) -> Option<Value>;
}

/**
The default capturing method is with the `Value::capture_display` method.
*/
pub type WithDefault = CaptureDisplay;

/**
A marker trait used to allow a single implementation for capturing common unsized
types for any sized capture strategy.
*/
pub trait CaptureStr {}

pub enum CaptureDisplay {}
pub enum CaptureAnonDisplay {}

pub enum CaptureDebug {}
pub enum CaptureAnonDebug {}

pub enum CaptureSval {}
pub enum CaptureAnonSval {}

pub enum CaptureSerde {}
pub enum CaptureAnonSerde {}

pub enum CaptureLevel {}

pub enum CaptureError {}

pub enum CaptureValue {}
pub enum CaptureAnonValue {}

pub enum CaptureSpanId {}
pub enum CaptureTraceId {}

impl CaptureStr for CaptureDisplay {}
impl CaptureStr for CaptureDebug {}
impl CaptureStr for CaptureSval {}
impl CaptureStr for CaptureSerde {}
impl CaptureStr for CaptureLevel {}
impl CaptureStr for CaptureError {}
impl CaptureStr for CaptureSpanId {}
impl CaptureStr for CaptureTraceId {}
impl CaptureStr for CaptureValue {}

impl<T: CaptureStr + ?Sized> Capture<T> for str {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl Capture<CaptureSpanId> for SpanId {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: Capture<CaptureSpanId>> Capture<CaptureSpanId> for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

impl Capture<CaptureTraceId> for TraceId {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: Capture<CaptureTraceId>> Capture<CaptureTraceId> for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

impl Capture<CaptureLevel> for Level {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T: Capture<CaptureLevel>> Capture<CaptureLevel> for Option<T> {
    fn capture(&self) -> Option<Value> {
        self.as_ref().and_then(|v| v.capture())
    }
}

impl<T> Capture<CaptureDisplay> for T
where
    T: fmt::Display + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_display(self))
    }
}

impl Capture<CaptureDisplay> for dyn fmt::Display {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T> Capture<CaptureAnonDisplay> for T
where
    T: fmt::Display,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_display(self))
    }
}

impl<T> Capture<CaptureDebug> for T
where
    T: fmt::Debug + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_debug(self))
    }
}

impl Capture<CaptureDebug> for dyn fmt::Debug {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T> Capture<CaptureAnonDebug> for T
where
    T: fmt::Debug,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_debug(self))
    }
}

impl<T> Capture<CaptureValue> for T
where
    T: ToValue + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

impl<T> Capture<CaptureAnonValue> for T
where
    T: ToValue,
{
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureSval> for T
where
    T: sval::Value + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_sval(self))
    }
}

#[cfg(feature = "sval")]
impl<T> Capture<CaptureAnonSval> for T
where
    T: sval::Value,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_sval(self))
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureSerde> for T
where
    T: serde::Serialize + Any,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_serde(self))
    }
}

#[cfg(feature = "serde")]
impl<T> Capture<CaptureAnonSerde> for T
where
    T: serde::Serialize,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::from_serde(self))
    }
}

#[cfg(feature = "std")]
impl<T> Capture<CaptureError> for T
where
    T: Error + 'static,
{
    fn capture(&self) -> Option<Value> {
        Some(Value::capture_error(self))
    }
}

#[cfg(feature = "std")]
impl<'a> Capture<CaptureError> for (dyn Error + 'static) {
    fn capture(&self) -> Option<Value> {
        Some(self.to_value())
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
        Self: Capture<WithDefault>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_display(&self) -> Option<Value>
    where
        Self: Capture<CaptureDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_display(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonDisplay>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_debug(&self) -> Option<Value>
    where
        Self: Capture<CaptureDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_debug(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonDebug>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_value(&self) -> Option<Value>
    where
        Self: Capture<CaptureValue>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_value(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonValue>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_sval(&self) -> Option<Value>
    where
        Self: Capture<CaptureSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_sval(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonSval>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_serde(&self) -> Option<Value>
    where
        Self: Capture<CaptureSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_anon_as_serde(&self) -> Option<Value>
    where
        Self: Capture<CaptureAnonSerde>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_level(&self) -> Option<Value>
    where
        Self: Capture<CaptureLevel>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_error(&self) -> Option<Value>
    where
        Self: Capture<CaptureError>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_span_id(&self) -> Option<Value>
    where
        Self: Capture<CaptureSpanId>,
    {
        Capture::capture(self)
    }

    fn __private_capture_as_trace_id(&self) -> Option<Value>
    where
        Self: Capture<CaptureTraceId>,
    {
        Capture::capture(self)
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

#[track_caller]
pub fn __private_now<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
) -> Option<Timestamp> {
    rt.clock().now()
}

#[track_caller]
pub fn __private_start_timer<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
) -> Timer<&'a T> {
    Timer::start(rt.clock())
}

#[track_caller]
pub fn __private_emit<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    module: impl Into<Path<'b>>,
    when: Option<impl Filter>,
    extent: impl ToExtent,
    tpl: Template<'b>,
    props: impl Props,
) {
    rt.ctxt().with_current(|ctxt| {
        let evt = Event::new(
            module,
            extent.to_extent().or_else(|| rt.clock().now().to_extent()),
            tpl,
            props.and_props(ctxt),
        );

        if FirstDefined(when, rt.filter()).matches(&evt) {
            rt.emitter().emit(&evt);
        }
    });
}

#[track_caller]
pub fn __private_emit_event<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    when: Option<impl Filter>,
    event: &'b impl ToEvent,
    tpl: Option<Template<'b>>,
    props: impl Props,
) {
    rt.ctxt().with_current(|ctxt| {
        let mut event = event.to_event();

        if let Some(tpl) = tpl {
            event = event.with_tpl(tpl);
        }

        let event = event.map_props(|event_props| props.and_props(event_props).and_props(ctxt));

        if FirstDefined(when, rt.filter()).matches(&event) {
            rt.emitter().emit(&event);
        }
    });
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
    S: FnOnce(SpanEvent<'static, Empty>),
>(
    rt: &'a Runtime<E, F, C, T, R>,
    module: impl Into<Path<'static>>,
    when: Option<impl Filter>,
    tpl: Template<'b>,
    ctxt_props: impl Props,
    evt_props: impl Props,
    name: impl Into<Str<'static>>,
    default_complete: S,
) -> (Frame<Option<&'a C>>, Span<'static, &'a T, Empty, S>) {
    let mut span = Span::filtered_new(
        |span| {
            FirstDefined(when, rt.filter()).matches(
                &span
                    .to_event()
                    .with_tpl(tpl)
                    .map_props(|props| props.and_props(&ctxt_props).and_props(&evt_props)),
            )
        },
        module,
        Timer::start(rt.clock()),
        name,
        SpanCtxt::current(rt.ctxt()).new_child(rt.rng()),
        Empty,
        default_complete,
    );

    let frame = span.push_ctxt(rt.ctxt(), ctxt_props);

    (frame, span)
}

#[track_caller]
pub fn __private_complete_span<'a, 'b, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
    rt: &'a Runtime<E, F, C, T, R>,
    span: SpanEvent<'static, Empty>,
    tpl: Template<'b>,
    evt_props: impl Props,
) {
    __private_emit(
        rt,
        span.module(),
        Some(crate::filter::always()),
        span.extent(),
        tpl,
        evt_props.and_props(&span),
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
