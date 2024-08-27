/*!
Integrate `emit` with the OpenTelemetry SDK.

This library forwards diagnostic events from emit through the OpenTelemetry SDK as log records and spans. This library is for applications that already use the OpenTelemetry SDK. It's also intended for applications that need to unify multiple instrumentation libraries, like `emit`, `log`, and `tracing`, into a shared pipeline. If you'd just like to send `emit` diagnostics via OTLP to the OpenTelemetry Collector or other compatible service, then consider [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.10/emit_otlp/index.html).

# Getting started

Configure OpenTelemetry as per its documentation, then add `emit` and `emit_opentelemetry` to your Cargo.toml:

```toml
[dependencies.emit]
version = "0.11.0-alpha.10"

[dependencies.emit_opentelemetry]
version = "0.11.0-alpha.10"
```

Initialize `emit` to send diagnostics to the OpenTelemetry SDK using [`setup`]:

```
fn main() {
    // Configure the OpenTelemetry SDK

    let rt = emit_opentelemetry::setup().init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));

    // Shutdown the OpenTelemetry SDK
}
```

Both the `emitter` and `ctxt` values must be set in order for `emit` to integrate with the OpenTelemetry SDK properly.

Diagnostic events produced by the [`macro@emit::span`] macro are sent to an [`opentelemetry::global::tracer`] as an [`opentelemetry::trace::Span`] on completion. All other emitted events are sent to an [`opentelemetry::global::logger`] as [`opentelemetry::logs::LogRecord`]s.

# Limitations

This library doesn't support `emit`'s metrics as OpenTelemetry metrics. Any metric samples produced by `emit` will be emitted as log records.

# Troubleshooting

If you're not seeing `emit` diagnostics flow as expected through the OpenTelemetry SDK, you can try configuring `emit`'s internal logger, and collect metrics from the integration:

```
# mod emit_term {
#     pub fn stdout() -> impl emit::runtime::InternalEmitter + Send + Sync + 'static {
#        emit::runtime::AssertInternal(emit::emitter::from_fn(|_| {}))
#     }
# }
use emit::metric::Source;

fn main() {
    // 1. Initialize the internal logger
    //    Diagnostics produced by `emit_opentelemetry` itself will go here
    let internal = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    let mut reporter = emit::metric::Reporter::new();

    let rt = emit_opentelemetry::setup()
        .map_emitter(|emitter| {
            // 2. Add `emit_opentelemetry`'s metrics to a reporter so we can see what it's up to
            //    You can do this independently of the internal emitter
            reporter.add_source(emitter.metric_source());

            emitter
        })
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));

    // 3. Report metrics after attempting to flush
    //    You could also do this periodically as your application runs
    reporter.emit_metrics(&internal.emitter());
}
```

Also see the [`opentelemetry`] docs for any details on getting diagnostics out of it.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]

use std::{cell::RefCell, fmt, ops::ControlFlow, sync::Arc};

use emit::{
    str::ToStr,
    value::ToValue,
    well_known::{
        KEY_ERR, KEY_EVT_KIND, KEY_LVL, KEY_SPAN_ID, KEY_SPAN_NAME, KEY_SPAN_PARENT, KEY_TRACE_ID,
        LVL_DEBUG, LVL_ERROR, LVL_INFO, LVL_WARN,
    },
    Filter, Props,
};

use opentelemetry::{
    global::{self, BoxedTracer, GlobalLoggerProvider},
    logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity},
    trace::{
        SpanContext, SpanId, Status, TraceContextExt, TraceFlags, TraceId, TraceState, Tracer,
    },
    Context, ContextGuard, Key, KeyValue, Value,
};

mod internal_metrics;

pub use internal_metrics::*;

/**
Start a builder for the `emit` to OpenTelemetry SDK integration.

```
fn main() {
    // Configure the OpenTelemetry SDK

    let rt = emit_opentelemetry::setup().init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));

    // Shutdown the OpenTelemetry SDK
}
```

Use [`emit::Setup::map_emitter`] and [`emit::Setup::map_ctxt`] on the returned value to customize the integration:

```
fn main() {
    // Configure the OpenTelemetry SDK

    let rt = emit_opentelemetry::setup()
        .map_emitter(|emitter| emitter
            .with_log_body(|evt, f| write!(f, "{}", evt.tpl()))
            .with_span_name(|evt, f| write!(f, "{}", evt.tpl()))
        )
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));

    // Shutdown the OpenTelemetry SDK
}
```
*/
pub fn setup() -> emit::Setup<
    OpenTelemetryEmitter,
    emit::setup::DefaultFilter,
    OpenTelemetryCtxt<emit::setup::DefaultCtxt>,
> {
    let name = "emit";
    let metrics = Arc::new(InternalMetrics::default());

    emit::setup()
        .emit_to(OpenTelemetryEmitter::new(metrics.clone(), name))
        .map_ctxt(|ctxt| OpenTelemetryCtxt::wrap(metrics.clone(), name, ctxt))
}

/**
An [`emit::Ctxt`] created during [`setup`] for integrating `emit` with the OpenTelemetry SDK.

This type is responsible for intercepting calls that push span state to `emit`'s ambient context and forwarding them to the OpenTelemetry SDK's own context.

When [`macro@emit::span`] is called, an [`opentelemetry::trace::Span`] is started using the given trace and span ids. The span doesn't carry any other ambient properties until it's completed either through [`emit::span::SpanGuard::complete`], or at the end of the scope the [`macro@emit::span`] macro covers.
*/
pub struct OpenTelemetryCtxt<C> {
    tracer: BoxedTracer,
    metrics: Arc<InternalMetrics>,
    inner: C,
}

/**
The [`emit::Ctxt::Frame`] used by [`OpenTelemetryCtxt`].
*/
pub struct OpenTelemetryFrame<F> {
    // Holds the OpenTelemetry context fetched when the span was started
    // If this field is `None`, and `active` is true then the slot has been
    // moved into the thread-local stack
    slot: Option<Context>,
    // Whether `slot` has been moved into the thread-local stack
    // If this field is false and `active` is `None` then the frame doesn't
    // hold an OpenTelemetry span
    active: bool,
    // Whether to try end the span upon closing the frame
    // Normally, this should be false by the time `close`` is called because
    // the emitter will have ended the span
    end_on_close: bool,
    // The frame from the wrapped `Ctxt`
    inner: F,
}

/**
The [`emit::Ctxt::Current`] used by [`OpenTelemetryCtxt`].
*/
pub struct OpenTelemetryProps<P: ?Sized> {
    trace_id: Option<emit::span::TraceId>,
    span_id: Option<emit::span::SpanId>,
    // Props from the wrapped `Ctxt`
    inner: *const P,
}

impl<C> OpenTelemetryCtxt<C> {
    fn wrap(metrics: Arc<InternalMetrics>, name: &'static str, ctxt: C) -> Self {
        OpenTelemetryCtxt {
            tracer: global::tracer(name),
            inner: ctxt,
            metrics,
        }
    }

    /**
    Get an [`emit::metric::Source`] for instrumentation produced by the `emit` to OpenTelemetry SDK integration.

    These metrics are shared by [`OpenTelemetryEmitter::metric_source`].

    These metrics can be used to monitor the running health of your diagnostic pipeline.
    */
    pub fn metric_source(&self) -> EmitOpenTelemetryMetrics {
        EmitOpenTelemetryMetrics {
            metrics: self.metrics.clone(),
        }
    }
}

impl<P: emit::Props + ?Sized> emit::Props for OpenTelemetryProps<P> {
    fn for_each<'kv, F: FnMut(emit::Str<'kv>, emit::Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref trace_id) = self.trace_id {
            for_each(emit::well_known::KEY_TRACE_ID.to_str(), trace_id.to_value())?;
        }

        if let Some(ref span_id) = self.span_id {
            for_each(emit::well_known::KEY_SPAN_ID.to_str(), span_id.to_value())?;
        }

        // SAFETY: This type is only exposed for arbitrarily short (`for<'a>`) lifetimes
        // so inner it's guaranteed to be valid for `'kv`, which must be shorter than its
        // original lifetime
        unsafe { &*self.inner }.for_each(|k, v| {
            if k != emit::well_known::KEY_TRACE_ID && k != emit::well_known::KEY_SPAN_ID {
                for_each(k, v)?;
            }

            ControlFlow::Continue(())
        })
    }
}

thread_local! {
    static ACTIVE_FRAME_STACK: RefCell<Vec<ActiveFrame>> = RefCell::new(Vec::new());
}

struct ActiveFrame {
    _guard: ContextGuard,
    end_on_close: bool,
}

fn push(guard: ActiveFrame) {
    ACTIVE_FRAME_STACK.with(|stack| stack.borrow_mut().push(guard));
}

fn pop() -> Option<ActiveFrame> {
    ACTIVE_FRAME_STACK.with(|stack| stack.borrow_mut().pop())
}

fn with_current(f: impl FnOnce(&mut ActiveFrame)) {
    ACTIVE_FRAME_STACK.with(|stack| {
        if let Some(frame) = stack.borrow_mut().last_mut() {
            f(frame);
        }
    })
}

impl<C: emit::Ctxt> emit::Ctxt for OpenTelemetryCtxt<C> {
    type Current = OpenTelemetryProps<C::Current>;

    type Frame = OpenTelemetryFrame<C::Frame>;

    fn open_root<P: emit::Props>(&self, props: P) -> Self::Frame {
        let trace_id = props.pull::<emit::span::TraceId, _>(emit::well_known::KEY_TRACE_ID);
        let span_id = props.pull::<emit::span::SpanId, _>(emit::well_known::KEY_SPAN_ID);

        // Only open a span if the props include a span id
        if let Some(span_id) = span_id {
            let ctxt = Context::current();

            let span_id = otel_span_id(span_id);

            // Only open a span if the id has changed
            if span_id != ctxt.span().span_context().span_id() {
                let trace_id = trace_id.map(otel_trace_id);

                let mut span = self.tracer.span_builder("emit_span").with_span_id(span_id);

                if let Some(trace_id) = trace_id {
                    span = span.with_trace_id(trace_id);
                }

                let ctxt = ctxt.with_span(span.start(&self.tracer));

                return OpenTelemetryFrame {
                    active: false,
                    end_on_close: true,
                    slot: Some(ctxt),
                    inner: self.inner.open_root(props),
                };
            }
        }

        OpenTelemetryFrame {
            active: false,
            end_on_close: false,
            slot: None,
            inner: self.inner.open_root(props),
        }
    }

    // TODO: open_push

    fn enter(&self, local: &mut Self::Frame) {
        self.inner.enter(&mut local.inner);

        if let Some(ctxt) = local.slot.take() {
            let guard = ctxt.attach();

            push(ActiveFrame {
                _guard: guard,
                end_on_close: local.end_on_close,
            });
            local.active = true;
        }
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        let ctxt = Context::current();
        let span = ctxt.span();

        // Use the trace and span ids from OpenTelemetry
        // If an OpenTelemetry span is created between calls to `enter`
        // and `exit` then these values won't match what's on the frame
        // We need to observe that to keep `emit`'s diagnostics aligned
        // with other OpenTelemetry consumers in the same application
        let trace_id = span.span_context().trace_id().to_bytes();
        let span_id = span.span_context().span_id().to_bytes();

        self.inner.with_current(|props| {
            let props = OpenTelemetryProps {
                trace_id: emit::span::TraceId::from_bytes(trace_id),
                span_id: emit::span::SpanId::from_bytes(span_id),
                inner: props as *const C::Current,
            };

            with(&props)
        })
    }

    fn exit(&self, frame: &mut Self::Frame) {
        self.inner.exit(&mut frame.inner);

        if frame.active {
            frame.slot = Some(Context::current());

            if let Some(active) = pop() {
                frame.end_on_close = active.end_on_close;
            }
            frame.active = false;
        }
    }

    fn close(&self, mut frame: Self::Frame) {
        // If the span hasn't been closed through an event, then close it now
        if frame.end_on_close {
            // This will only be `None` if `close` is called out-of-order
            // with `exit`
            if let Some(ctxt) = frame.slot.take() {
                self.metrics.span_unexpected_close.increment();

                let span = ctxt.span();
                span.end();
            }
        }
    }
}

/**
An [`emit::Emitter`] creating during [`setup`] for integrating `emit` with the OpenTelemetry SDK.

This type is responsible for emitting diagnostic events as log records through the OpenTelemetry SDK and completing spans created through the integration.
*/
pub struct OpenTelemetryEmitter {
    inner: <GlobalLoggerProvider as LoggerProvider>::Logger,
    span_name: Box<MessageFormatter>,
    log_body: Box<MessageFormatter>,
    metrics: Arc<InternalMetrics>,
}

type MessageFormatter = dyn Fn(&emit::event::Event<&dyn emit::props::ErasedProps>, &mut fmt::Formatter) -> fmt::Result
    + Send
    + Sync;

fn default_span_name() -> Box<MessageFormatter> {
    Box::new(|evt, f| {
        if let Some(name) = evt.props().get(KEY_SPAN_NAME) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", evt.msg())
        }
    })
}

fn default_log_body() -> Box<MessageFormatter> {
    Box::new(|evt, f| write!(f, "{}", evt.msg()))
}

struct MessageRenderer<'a, P> {
    pub fmt: &'a MessageFormatter,
    pub evt: &'a emit::event::Event<'a, P>,
}

impl<'a, P: emit::props::Props> fmt::Display for MessageRenderer<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.fmt)(&self.evt.erase(), f)
    }
}

impl OpenTelemetryEmitter {
    fn new(metrics: Arc<InternalMetrics>, name: &'static str) -> Self {
        OpenTelemetryEmitter {
            inner: global::logger_provider().logger(name),
            span_name: default_span_name(),
            log_body: default_log_body(),
            metrics,
        }
    }

    /**
    Specify a function that gets the name of a span from a diagnostic event.

    The default implementation uses the [`KEY_SPAN_NAME`] property on the event, or [`emit::Event::msg`] if it's not present.
    */
    pub fn with_span_name(
        mut self,
        writer: impl Fn(
                &emit::event::Event<&dyn emit::props::ErasedProps>,
                &mut fmt::Formatter,
            ) -> fmt::Result
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.span_name = Box::new(writer);
        self
    }

    /**
    Specify a function that gets the body of a log record from a diagnostic event.

    The default implementation uses [`emit::Event::msg`].
    */
    pub fn with_log_body(
        mut self,
        writer: impl Fn(
                &emit::event::Event<&dyn emit::props::ErasedProps>,
                &mut fmt::Formatter,
            ) -> fmt::Result
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.log_body = Box::new(writer);
        self
    }

    /**
    Get an [`emit::metric::Source`] for instrumentation produced by the `emit` to OpenTelemetry SDK integration.

    These metrics are shared by [`OpenTelemetryCtxt::metric_source`].

    These metrics can be used to monitor the running health of your diagnostic pipeline.
    */
    pub fn metric_source(&self) -> EmitOpenTelemetryMetrics {
        EmitOpenTelemetryMetrics {
            metrics: self.metrics.clone(),
        }
    }
}

impl emit::Emitter for OpenTelemetryEmitter {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        // If the event is for a span then attempt to end it
        // The typical case is the span was created through `#[emit::span]`
        // and so is the currently active frame. If it isn't the active frame
        // then it's been created manually or spans are being completed out of order
        if emit::kind::is_span_filter().matches(&evt) {
            let mut emitted = false;
            with_current(|frame| {
                if frame.end_on_close {
                    let ctxt = Context::current();
                    let span = ctxt.span();

                    let span_id = span.span_context().span_id();

                    let evt_span_id = evt
                        .props()
                        .pull::<emit::span::SpanId, _>(KEY_SPAN_ID)
                        .map(otel_span_id);

                    // If the event is for the current span then complete it
                    if Some(span_id) == evt_span_id {
                        let name = format!(
                            "{}",
                            MessageRenderer {
                                fmt: &self.span_name,
                                evt: &evt,
                            }
                        );

                        span.update_name(name);

                        evt.props().for_each(|k, v| {
                            if k == KEY_LVL {
                                if let Some(emit::Level::Error) = v.by_ref().cast::<emit::Level>() {
                                    span.set_status(Status::error("error"));

                                    return ControlFlow::Continue(());
                                }
                            }

                            if k == KEY_ERR {
                                span.add_event(
                                    "exception",
                                    vec![KeyValue::new("exception.message", v.to_string())],
                                );

                                return ControlFlow::Continue(());
                            }

                            if k == KEY_TRACE_ID
                                || k == KEY_SPAN_ID
                                || k == KEY_SPAN_PARENT
                                || k == KEY_SPAN_NAME
                                || k == KEY_EVT_KIND
                            {
                                return ControlFlow::Continue(());
                            }

                            if let Some(v) = otel_span_value(v) {
                                span.set_attribute(KeyValue::new(k.to_cow(), v));
                            }

                            ControlFlow::Continue(())
                        });

                        if let Some(extent) = evt.extent().and_then(|ex| ex.as_span()) {
                            span.end_with_timestamp(extent.end.to_system_time());
                        } else {
                            span.end();
                        }

                        frame.end_on_close = false;
                        emitted = true;
                    }
                }
            });

            if emitted {
                return;
            } else {
                self.metrics.span_unexpected_emit.increment();
            }
        }

        // If the event wasn't emitted as a span then emit it as a log record
        let mut record = LogRecord::builder();

        let body = format!(
            "{}",
            MessageRenderer {
                fmt: &self.log_body,
                evt: &evt,
            }
        );
        record = record.with_body(body);

        let mut trace_id = None;
        let mut span_id = None;
        let mut attributes = Vec::new();
        {
            let mut slot = Some(record);
            evt.props().for_each(|k, v| {
                if k == KEY_LVL {
                    match v.by_ref().cast::<emit::Level>() {
                        Some(emit::Level::Debug) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Debug)
                                    .with_severity_text(LVL_DEBUG),
                            );
                        }
                        Some(emit::Level::Info) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Info)
                                    .with_severity_text(LVL_INFO),
                            );
                        }
                        Some(emit::Level::Warn) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Warn)
                                    .with_severity_text(LVL_WARN),
                            );
                        }
                        Some(emit::Level::Error) => {
                            slot = Some(
                                slot.take()
                                    .unwrap()
                                    .with_severity_number(Severity::Error)
                                    .with_severity_text(LVL_ERROR),
                            );
                        }
                        None => {
                            slot = Some(slot.take().unwrap().with_severity_text(v.to_string()));
                        }
                    }

                    return ControlFlow::Continue(());
                }

                if k == KEY_TRACE_ID {
                    if let Some(id) = v.by_ref().cast::<emit::span::TraceId>() {
                        trace_id = Some(otel_trace_id(id));

                        return ControlFlow::Continue(());
                    }
                }

                if k == KEY_SPAN_ID {
                    if let Some(id) = v.by_ref().cast::<emit::span::SpanId>() {
                        span_id = Some(otel_span_id(id));

                        return ControlFlow::Continue(());
                    }
                }

                if k == KEY_SPAN_PARENT {
                    if v.by_ref().cast::<emit::span::SpanId>().is_some() {
                        return ControlFlow::Continue(());
                    }
                }

                if let Some(v) = otel_log_value(v) {
                    attributes.push((Key::new(k.to_cow()), v));
                }

                ControlFlow::Continue(())
            });

            record = slot.unwrap();
        }

        record = record.with_attributes(attributes);

        if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
            record = record.with_span_context(&SpanContext::new(
                trace_id,
                span_id,
                TraceFlags::SAMPLED,
                false,
                TraceState::NONE,
            ))
        }

        if let Some(extent) = evt.extent() {
            record = record.with_timestamp(extent.as_point().to_system_time());
        }

        self.inner.emit(record.build());
    }

    fn blocking_flush(&self, _: std::time::Duration) -> bool {
        false
    }
}

fn otel_trace_id(trace_id: emit::span::TraceId) -> TraceId {
    TraceId::from_bytes(trace_id.to_bytes())
}

fn otel_span_id(span_id: emit::span::SpanId) -> SpanId {
    SpanId::from_bytes(span_id.to_bytes())
}

fn otel_span_value(v: emit::Value) -> Option<Value> {
    match any_value::serialize(&v) {
        Ok(Some(av)) => match av {
            AnyValue::Int(v) => Some(Value::I64(v)),
            AnyValue::Double(v) => Some(Value::F64(v)),
            AnyValue::String(v) => Some(Value::String(v)),
            AnyValue::Boolean(v) => Some(Value::Bool(v)),
            // Variants not supported by `Value`
            AnyValue::Bytes(_) => Some(Value::String(v.to_string().into())),
            AnyValue::ListAny(_) => Some(Value::String(v.to_string().into())),
            AnyValue::Map(_) => Some(Value::String(v.to_string().into())),
        },
        Ok(None) => None,
        Err(()) => Some(Value::String(v.to_string().into())),
    }
}

fn otel_log_value(v: emit::Value) -> Option<AnyValue> {
    match any_value::serialize(&v) {
        Ok(v) => v,
        Err(()) => Some(AnyValue::String(v.to_string().into())),
    }
}

mod any_value {
    use std::{collections::HashMap, fmt};

    use opentelemetry::{logs::AnyValue, Key, StringValue};
    use serde::ser::{
        Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
        SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
    };

    /// Serialize an arbitrary `serde::Serialize` into an `AnyValue`.
    ///
    /// This method performs the following translations when converting between `serde`'s data model and OpenTelemetry's:
    ///
    /// - Integers that don't fit in a `i64` are converted into strings.
    /// - Unit types and nones are discarded (effectively treated as undefined).
    /// - Struct and tuple variants are converted into an internally tagged map.
    /// - Unit variants are converted into strings.
    pub(crate) fn serialize(value: impl serde::Serialize) -> Result<Option<AnyValue>, ()> {
        value.serialize(ValueSerializer).map_err(|_| ())
    }

    struct ValueSerializer;

    struct ValueSerializeSeq {
        value: Vec<AnyValue>,
    }

    struct ValueSerializeTuple {
        value: Vec<AnyValue>,
    }

    struct ValueSerializeTupleStruct {
        value: Vec<AnyValue>,
    }

    struct ValueSerializeMap {
        key: Option<Key>,
        value: HashMap<Key, AnyValue>,
    }

    struct ValueSerializeStruct {
        value: HashMap<Key, AnyValue>,
    }

    struct ValueSerializeTupleVariant {
        variant: &'static str,
        value: Vec<AnyValue>,
    }

    struct ValueSerializeStructVariant {
        variant: &'static str,
        value: HashMap<Key, AnyValue>,
    }

    #[derive(Debug)]
    struct ValueError(String);

    impl fmt::Display for ValueError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }

    impl Error for ValueError {
        fn custom<T>(msg: T) -> Self
        where
            T: fmt::Display,
        {
            ValueError(msg.to_string())
        }
    }

    impl StdError for ValueError {}

    impl Serializer for ValueSerializer {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        type SerializeSeq = ValueSerializeSeq;

        type SerializeTuple = ValueSerializeTuple;

        type SerializeTupleStruct = ValueSerializeTupleStruct;

        type SerializeTupleVariant = ValueSerializeTupleVariant;

        type SerializeMap = ValueSerializeMap;

        type SerializeStruct = ValueSerializeStruct;

        type SerializeStructVariant = ValueSerializeStructVariant;

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Boolean(v)))
        }

        fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Int(v)))
        }

        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(v as i64)
        }

        fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
            if let Ok(v) = v.try_into() {
                self.serialize_i64(v)
            } else {
                self.collect_str(&v)
            }
        }

        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            self.serialize_f64(v as f64)
        }

        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Double(v)))
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            self.collect_str(&v)
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::String(StringValue::from(v.to_owned()))))
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Bytes(v.to_owned())))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(None)
        }

        fn serialize_some<T: serde::Serialize + ?Sized>(
            self,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(None)
        }

        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
            name.serialize(self)
        }

        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            variant.serialize(self)
        }

        fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
            self,
            _: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error> {
            let mut map = self.serialize_map(Some(1))?;
            map.serialize_entry(variant, value)?;
            map.end()
        }

        fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(ValueSerializeSeq { value: Vec::new() })
        }

        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(ValueSerializeTuple { value: Vec::new() })
        }

        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(ValueSerializeTupleStruct { value: Vec::new() })
        }

        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(ValueSerializeTupleVariant {
                variant,
                value: Vec::new(),
            })
        }

        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(ValueSerializeMap {
                key: None,
                value: HashMap::new(),
            })
        }

        fn serialize_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Ok(ValueSerializeStruct {
                value: HashMap::new(),
            })
        }

        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            variant: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(ValueSerializeStructVariant {
                variant,
                value: HashMap::new(),
            })
        }
    }

    impl SerializeSeq for ValueSerializeSeq {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_element<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::ListAny(self.value)))
        }
    }

    impl SerializeTuple for ValueSerializeTuple {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_element<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::ListAny(self.value)))
        }
    }

    impl SerializeTupleStruct for ValueSerializeTupleStruct {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::ListAny(self.value)))
        }
    }

    impl SerializeTupleVariant for ValueSerializeTupleVariant {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.push(value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map({
                let mut variant = HashMap::new();
                variant.insert(Key::from(self.variant), AnyValue::ListAny(self.value));
                variant
            })))
        }
    }

    impl SerializeMap for ValueSerializeMap {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_key<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &T,
        ) -> Result<(), Self::Error> {
            let key = match key.serialize(ValueSerializer)? {
                Some(AnyValue::String(key)) => Key::from(String::from(key)),
                key => Key::from(format!("{:?}", key)),
            };

            self.key = Some(key);

            Ok(())
        }

        fn serialize_value<T: serde::Serialize + ?Sized>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = self
                .key
                .take()
                .ok_or_else(|| Self::Error::custom("missing key"))?;

            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map(self.value)))
        }
    }

    impl SerializeStruct for ValueSerializeStruct {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = Key::from(key);

            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map(self.value)))
        }
    }

    impl SerializeStructVariant for ValueSerializeStructVariant {
        type Ok = Option<AnyValue>;

        type Error = ValueError;

        fn serialize_field<T: serde::Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let key = Key::from(key);

            if let Some(value) = value.serialize(ValueSerializer)? {
                self.value.insert(key, value);
            }

            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Some(AnyValue::Map({
                let mut variant = HashMap::new();
                variant.insert(Key::from(self.variant), AnyValue::Map(self.value));
                variant
            })))
        }
    }
}
