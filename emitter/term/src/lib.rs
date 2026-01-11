/*!
Emit diagnostic events to the console.

This library implements a text-based format that's intended for direct end-user consumption, such as in interactive applications.

# Getting started

Add `emit` and `emit_term` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "1.16.0"

[dependencies.emit_term]
version = "1.16.0"
```

Initialize `emit` using `emit_term`:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

`emit_term` uses a format optimized for human legibility, not for machine processing. You may also want to emit diagnostics to another location, such as OTLP through `emit_otlp` or a rolling file through `emit_file` for processing. You can use [`emit::Setup::and_emit_to`] to combine multiple emitters:

```
# fn some_other_emitter() -> impl emit::Emitter + Send + Sync + 'static {
#    emit::emitter::from_fn(|_| {})
# }
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .and_emit_to(some_other_emitter())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

## Configuration

`emit_term` has a fixed format, but can be configured to force or disable color output instead of detect it.

To disable colors, call [`Stdout::colored`] with the value `false`:

```rust
fn main() {
    let rt = emit::setup()
        // Disable colors
        .emit_to(emit_term::stdout().colored(false))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

To force colors, call [`Stdout::colored`] with the value `true`:

```rust
fn main() {
    let rt = emit::setup()
        // Force colors
        .emit_to(emit_term::stdout().colored(true))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]

use std::{cell::RefCell, collections::BTreeMap, fmt, io::Write, iter, str, time::Duration};

use emit::{
    metric::exp::Point,
    well_known::{
        KEY_DIST_EXP_BUCKETS, KEY_DIST_EXP_SCALE, KEY_ERR, KEY_EVT_KIND, KEY_LVL, KEY_SPAN_ID,
        KEY_TRACE_ID,
    },
};
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

/**
Get an emitter that writes to `stdout`.

Colors will be used if the terminal supports them.
*/
pub fn stdout() -> Stdout {
    Stdout::new()
}

/**
Get an emitter that writes to `stderr`.

Colors will be used if the terminal supports them.
*/
pub fn stderr() -> Stderr {
    Stderr::new()
}

/**
An emitter that writes to `stdout`.

Use [`stdout`] to construct an emitter and pass the result to [`emit::Setup::emit_to`] to configure `emit` to use it:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```
*/
pub struct Stdout {
    writer: Writer,
}

impl Stdout {
    /**
    Get an emitter that writes to `stdout`.

    Colors will be used if the terminal supports them.
    */
    pub fn new() -> Self {
        Stdout {
            writer: Writer {
                writer: BufferWriter::stdout(ColorChoice::Auto),
            },
        }
    }

    /**
    Whether to write using colors.

    By default, colors will be used if the terminal supports them. You can explicitly enable or disable colors using this function. If `colored` is true then colors will always be used. If `colored` is false then colors will never be used.
    */
    pub fn colored(mut self, colored: bool) -> Self {
        if colored {
            self.writer = Writer {
                writer: BufferWriter::stdout(ColorChoice::Always),
            };
        } else {
            self.writer = Writer {
                writer: BufferWriter::stdout(ColorChoice::Never),
            };
        }

        self
    }
}

impl emit::emitter::Emitter for Stdout {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        self.writer.emit(evt)
    }

    fn blocking_flush(&self, _: Duration) -> bool {
        true
    }
}

impl emit::runtime::InternalEmitter for Stdout {}

/**
An emitter that writes to `stderr`.

Use [`stderr`] to construct an emitter and pass the result to [`emit::Setup::emit_to`] to configure `emit` to use it:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stderr())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```
*/
pub struct Stderr {
    writer: Writer,
}

impl Stderr {
    /**
    Get an emitter that writes to `stderr`.

    Colors will be used if the terminal supports them.
    */
    pub fn new() -> Self {
        Stderr {
            writer: Writer {
                writer: BufferWriter::stderr(ColorChoice::Auto),
            },
        }
    }

    /**
    Whether to write using colors.

    By default, colors will be used if the terminal supports them. You can explicitly enable or disable colors using this function. If `colored` is true then colors will always be used. If `colored` is false then colors will never be used.
    */
    pub fn colored(mut self, colored: bool) -> Self {
        if colored {
            self.writer = Writer {
                writer: BufferWriter::stderr(ColorChoice::Always),
            };
        } else {
            self.writer = Writer {
                writer: BufferWriter::stderr(ColorChoice::Never),
            };
        }

        self
    }
}

impl emit::emitter::Emitter for Stderr {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        self.writer.emit(evt)
    }

    fn blocking_flush(&self, _: Duration) -> bool {
        true
    }
}

impl emit::runtime::InternalEmitter for Stderr {}

struct Writer {
    writer: BufferWriter,
}

impl Writer {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        with_shared_buf(&self.writer, |writer, buf| {
            write_event(buf, evt);

            let _ = writer.print(buf);
        });
    }
}

fn write_event(buf: &mut Buffer, evt: emit::event::Event<impl emit::props::Props>) {
    if let Some(span_id) = evt.props().pull::<emit::SpanId, _>(KEY_SPAN_ID) {
        if let Some(trace_id) = evt.props().pull::<emit::TraceId, _>(KEY_TRACE_ID) {
            let trace_id_color = trace_id_color(&trace_id);

            write_fg(buf, "▓", Color::Ansi256(trace_id_color));
            write_plain(buf, " ");
            write_plain(buf, hex_slice(&trace_id.to_hex(), 6));
            write_plain(buf, " ");
        } else {
            write_plain(buf, "░      ");
        }

        let span_id_color = span_id_color(&span_id);

        write_fg(buf, "▓", Color::Ansi256(span_id_color));
        write_plain(buf, " ");
        write_plain(buf, hex_slice(&span_id.to_hex(), 4));
        write_plain(buf, " ");
    }

    if let Some(extent) = evt.extent() {
        if let Some(len) = extent.len() {
            write_timestamp(buf, *extent.as_point());
            write_plain(buf, " ");
            write_duration(buf, len);
        } else if let Some(range) = extent.as_range() {
            write_timestamp(buf, range.start);
            write_plain(buf, "..");
            write_timestamp(buf, range.end);
        } else {
            write_timestamp(buf, *extent.as_point());
        }

        write_plain(buf, " ");
    }

    let mut lvl = None;
    if let Some(level) = evt.props().pull::<emit::Level, _>(KEY_LVL) {
        lvl = level_color(&level).map(Color::Ansi256);

        try_write_fg(buf, level, lvl);
        write_plain(buf, " ");
    }

    if let Some(kind) = evt.props().get(KEY_EVT_KIND) {
        write_fg(buf, kind, KIND);
        write_plain(buf, " ");
    }

    let mut mdl = evt.mdl().segments();
    if let (Some(first), last) = (mdl.next(), mdl.last()) {
        write_fg(buf, first, MDL_FIRST);
        write_plain(buf, " ");

        if let Some(last) = last {
            write_fg(buf, last, MDL_LAST);
            write_plain(buf, " ");
        }
    }

    let _ = evt.msg().write(TokenWriter { buf });
    write_plain(buf, "\n");

    if let Some(err) = evt.props().get(KEY_ERR) {
        if let Some(err) = err.to_borrowed_error() {
            write_plain(buf, "  ");
            try_write_fg(buf, "err", lvl);
            write_plain(buf, format_args!(": {err}\n"));

            for cause in iter::successors(err.source(), |err| (*err).source()) {
                write_plain(buf, "  ");
                try_write_fg(buf, "caused by", lvl);
                write_plain(buf, format_args!(": {cause}\n"));
            }
        }
    }

    if let (Some(scale), Some((count, buckets))) = (
        evt.props().pull::<i32, _>(KEY_DIST_EXP_SCALE),
        evt.props().get(KEY_DIST_EXP_BUCKETS).and_then(distribution),
    ) {
        let error = ((2.0f64.powf(2.0f64.powi(-scale)) - 1.0)
            / (1.0 + 2.0f64.powf(2.0f64.powi(-scale))))
            * 100.0;

        let default = buckets.last_key_value().unwrap().0.get();

        let mut qs = [
            ("Q1", count / 4, default),
            ("Q2", count / 2, default),
            ("Q3", count - (count / 4), default),
        ];

        let mut seen = 0;
        let mut fill = 0;
        for (bucket, count) in &buckets {
            seen += count;

            let qs = &mut qs[fill..];
            if qs.len() == 0 {
                break;
            }

            for (_, q, v) in qs {
                if seen >= *q {
                    *v = bucket.get();
                    fill += 1;
                }
            }
        }

        for (q, _, v) in qs {
            write_fg(buf, format_args!("{q}"), FIELD);
            write_plain(buf, format_args!(": "));
            write_fg(buf, format_args!("{v}"), NUMBER);
            write_plain(buf, ", ");
        }

        write_fg(buf, "error", FIELD);
        write_plain(buf, format_args!(": "));
        write_fg(buf, format_args!("{error:.2}%"), NUMBER);
        write_plain(buf, "\n");
    }
}

fn hex_slice<'a>(hex: &'a [u8], len: usize) -> impl fmt::Display + 'a {
    struct HexSlice<'a>(&'a [u8], usize);

    impl<'a> fmt::Display for HexSlice<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(str::from_utf8(&self.0[..self.1]).unwrap())
        }
    }

    HexSlice(hex, len)
}

struct LocalTime {
    h: u8,
    m: u8,
    s: u8,
    ms: u16,
}

fn local_ts(ts: emit::Timestamp) -> Option<LocalTime> {
    #[cfg(test)]
    {
        // In tests it's easier just to use full RFC3339 timestamps
        // since we don't know exactly what platforms `time` supports
        let _ = ts;

        None
    }
    #[cfg(not(test))]
    {
        // See: https://github.com/rust-lang/rust/issues/27970
        //
        // On Linux and OSX, this will fail to get the local offset in
        // any multi-threaded program. It needs to be fixed in the standard
        // library and propagated through libraries like `time`. Until then,
        // you probably won't get local timestamps outside of Windows.
        let local = time::OffsetDateTime::from_unix_timestamp_nanos(
            ts.to_unix().as_nanos().try_into().ok()?,
        )
        .ok()?;
        let local = local.checked_to_offset(time::UtcOffset::local_offset_at(local).ok()?)?;

        let (h, m, s, ms) = local.time().as_hms_milli();

        Some(LocalTime { h, m, s, ms })
    }
}

fn write_timestamp(buf: &mut Buffer, ts: emit::Timestamp) {
    if let Some(LocalTime { h, m, s, ms }) = local_ts(ts) {
        write_plain(
            buf,
            format_args!("{:>02}:{:>02}:{:>02}.{:>03}", h, m, s, ms),
        );
    } else {
        write_plain(buf, format_args!("{:.0}", ts));
    }
}

struct FriendlyDuration {
    pub value: u128,
    pub unit: &'static str,
}

fn friendly_duration(duration: Duration) -> FriendlyDuration {
    const NANOS_PER_MICRO: u128 = 1000;
    const NANOS_PER_MILLI: u128 = NANOS_PER_MICRO * 1000;
    const NANOS_PER_SEC: u128 = NANOS_PER_MILLI * 1000;
    const NANOS_PER_MIN: u128 = NANOS_PER_SEC * 60;

    let nanos = duration.as_nanos();

    if nanos < NANOS_PER_MICRO * 2 {
        FriendlyDuration {
            value: nanos,
            unit: "ns",
        }
    } else if nanos < NANOS_PER_MILLI * 2 {
        FriendlyDuration {
            value: nanos / NANOS_PER_MICRO,
            unit: "μs",
        }
    } else if nanos < NANOS_PER_SEC * 2 {
        FriendlyDuration {
            value: nanos / NANOS_PER_MILLI,
            unit: "ms",
        }
    } else if nanos < NANOS_PER_MIN * 2 {
        FriendlyDuration {
            value: nanos / NANOS_PER_SEC,
            unit: "s",
        }
    } else {
        FriendlyDuration {
            value: nanos / NANOS_PER_MIN,
            unit: "m",
        }
    }
}

fn write_duration(buf: &mut Buffer, duration: Duration) {
    let FriendlyDuration { value, unit } = friendly_duration(duration);

    write_fg(buf, value, NUMBER);
    write_fg(buf, unit, TEXT);
}

fn distribution(value: emit::Value) -> Option<(u64, BTreeMap<Point, u64>)> {
    #[derive(Default)]
    struct Extract {
        depth: usize,
        buckets: BTreeMap<Point, u64>,
        count: u64,
        next_midpoint: Option<f64>,
        next_count: Option<u64>,
    }

    impl Extract {
        fn push(
            &mut self,
            midpoint: impl FnOnce() -> Option<f64>,
            count: impl FnOnce() -> Option<u64>,
        ) -> sval::Result {
            if self.depth == 2 {
                if self.next_midpoint.is_none() {
                    self.next_midpoint = midpoint();

                    return Ok(());
                }

                if self.next_count.is_none() {
                    self.next_count = count();

                    return Ok(());
                }
            }

            sval::error()
        }

        fn apply(&mut self) -> sval::Result {
            if self.depth == 2 {
                let midpoint = self
                    .next_midpoint
                    .take()
                    .ok_or_else(|| sval::Error::new())?;
                let count = self.next_count.take().ok_or_else(|| sval::Error::new())?;

                *self.buckets.entry(Point::new(midpoint)).or_default() += count;
                self.count += count;

                Ok(())
            } else {
                Ok(())
            }
        }
    }

    impl<'sval> sval::Stream<'sval> for Extract {
        fn null(&mut self) -> sval::Result {
            sval::error()
        }

        fn bool(&mut self, _: bool) -> sval::Result {
            sval::error()
        }

        fn text_begin(&mut self, _: Option<usize>) -> sval::Result {
            sval::error()
        }

        fn text_fragment_computed(&mut self, _: &str) -> sval::Result {
            sval::error()
        }

        fn text_end(&mut self) -> sval::Result {
            sval::error()
        }

        fn i64(&mut self, value: i64) -> sval::Result {
            self.push(|| Some(value as f64), || value.try_into().ok())
        }

        fn u64(&mut self, value: u64) -> sval::Result {
            self.push(|| Some(value as f64), || Some(value))
        }

        fn f64(&mut self, value: f64) -> sval::Result {
            self.push(|| Some(value), || Some(value as u64))
        }

        fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
            self.depth += 1;

            if self.depth > 2 {
                sval::error()
            } else {
                Ok(())
            }
        }

        fn seq_value_begin(&mut self) -> sval::Result {
            Ok(())
        }

        fn seq_value_end(&mut self) -> sval::Result {
            Ok(())
        }

        fn seq_end(&mut self) -> sval::Result {
            self.apply()?;
            self.depth -= 1;

            Ok(())
        }
    }

    let mut extract = Extract::default();
    sval::stream(&mut extract, &value).ok()?;

    if extract.buckets.len() == 0 {
        return None;
    }

    Some((extract.count, extract.buckets))
}

struct TokenWriter<'a> {
    buf: &'a mut Buffer,
}

impl<'a> sval_fmt::TokenWrite for TokenWriter<'a> {
    fn write_text_quote(&mut self) -> fmt::Result {
        Ok(())
    }

    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.write(text, TEXT);

        Ok(())
    }

    fn write_number<N: fmt::Display>(&mut self, num: N) -> fmt::Result {
        self.write(num, NUMBER);

        Ok(())
    }

    fn write_atom<A: fmt::Display>(&mut self, atom: A) -> fmt::Result {
        self.write(atom, ATOM);

        Ok(())
    }

    fn write_ident(&mut self, ident: &str) -> fmt::Result {
        self.write(ident, IDENT);

        Ok(())
    }

    fn write_field(&mut self, field: &str) -> fmt::Result {
        self.write(field, FIELD);

        Ok(())
    }
}

impl<'a> fmt::Write for TokenWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(&mut self.buf, "{}", s).map_err(|_| fmt::Error)
    }
}

impl<'a> emit::template::Write for TokenWriter<'a> {
    fn write_hole_value(&mut self, _: &str, value: emit::Value) -> fmt::Result {
        sval_fmt::stream_to_token_write(self, value)
    }

    fn write_hole_fmt(
        &mut self,
        _: &str,
        value: emit::Value,
        formatter: emit::template::Formatter,
    ) -> fmt::Result {
        use sval::Value as _;

        match value.tag() {
            Some(sval::tags::NUMBER) => self.write(formatter.apply(value), NUMBER),
            _ => self.write(formatter.apply(value), TEXT),
        }

        Ok(())
    }
}

const KIND: Color = Color::Ansi256(174);
const MDL_FIRST: Color = Color::Ansi256(248);
const MDL_LAST: Color = Color::Ansi256(244);

const TEXT: Color = Color::Ansi256(69);
const NUMBER: Color = Color::Ansi256(135);
const ATOM: Color = Color::Ansi256(168);
const IDENT: Color = Color::Ansi256(170);
const FIELD: Color = Color::Ansi256(174);

fn trace_id_color(trace_id: &emit::TraceId) -> u8 {
    let mut hash = 0;

    for b in trace_id.to_u128().to_le_bytes() {
        hash ^= b;
    }

    hash
}

fn span_id_color(span_id: &emit::SpanId) -> u8 {
    let mut hash = 0;

    for b in span_id.to_u64().to_le_bytes() {
        hash ^= b;
    }

    hash
}

fn level_color(level: &emit::Level) -> Option<u8> {
    match level {
        emit::Level::Debug => Some(244),
        emit::Level::Info => None,
        emit::Level::Warn => Some(202),
        emit::Level::Error => Some(124),
    }
}

fn write_fg(buf: &mut Buffer, v: impl fmt::Display, color: Color) {
    let _ = buf.set_color(ColorSpec::new().set_fg(Some(color)));
    let _ = write!(buf, "{}", v);
    let _ = buf.reset();
}

fn try_write_fg(buf: &mut Buffer, v: impl fmt::Display, color: Option<Color>) {
    if let Some(color) = color {
        write_fg(buf, v, color);
    } else {
        write_plain(buf, v);
    }
}

fn write_plain(buf: &mut Buffer, v: impl fmt::Display) {
    let _ = write!(buf, "{}", v);
}

impl<'a> TokenWriter<'a> {
    fn write(&mut self, v: impl fmt::Display, color: Color) {
        write_fg(&mut *self.buf, v, color);
    }
}

fn with_shared_buf(writer: &BufferWriter, with_buf: impl FnOnce(&BufferWriter, &mut Buffer)) {
    thread_local! {
        static STDOUT: RefCell<Option<Buffer>> = RefCell::new(None);
    }

    STDOUT.with(|buf| {
        match buf.try_borrow_mut() {
            // If there are no overlapping references then use the cached buffer
            Ok(mut slot) => {
                match &mut *slot {
                    // If there's a cached buffer then clear it and print using it
                    Some(buf) => {
                        buf.clear();
                        with_buf(&writer, buf);
                    }
                    // If there's no cached buffer then create one and use it
                    // It'll be cached for future callers on this thread
                    None => {
                        let mut buf = writer.buffer();
                        with_buf(&writer, &mut buf);

                        *slot = Some(buf);
                    }
                }
            }
            // If there are overlapping references then just create a
            // buffer on-demand to use
            Err(_) => {
                with_buf(&writer, &mut writer.buffer());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str;

    #[test]
    fn write_log() {
        let mut buf = Buffer::no_color();

        write_event(
            &mut buf,
            emit::evt!(
                extent: emit::Timestamp::try_from_str("2024-01-01T01:02:03.000Z").unwrap(),
                "Hello, {user}",
                user: "Rust",
                extra: true,
            ),
        );

        assert_eq!(
            "2024-01-01T01:02:03Z emit_term tests Hello, Rust\n",
            str::from_utf8(buf.as_slice()).unwrap()
        );
    }

    #[test]
    fn write_log_err() {
        let mut buf = Buffer::no_color();

        write_event(
            &mut buf,
            emit::evt!(
                extent: emit::Timestamp::try_from_str("2024-01-01T01:02:03.000Z").unwrap(),
                "An error",
                lvl: "error",
                err: std::io::Error::new(std::io::ErrorKind::Other, "Something went wrong"),
            ),
        );

        assert_eq!(
            "2024-01-01T01:02:03Z error emit_term tests An error\n  err: Something went wrong\n",
            str::from_utf8(buf.as_slice()).unwrap()
        );
    }

    #[test]
    fn write_span() {
        let mut buf = Buffer::no_color();

        write_event(
            &mut buf,
            emit::evt!(
                extent:
                    emit::Timestamp::try_from_str("2024-01-01T01:02:03.000Z").unwrap()..
                    emit::Timestamp::try_from_str("2024-01-01T01:02:04.000Z").unwrap(),
                "Hello, {user}",
                user: "Rust",
                evt_kind: "span",
                trace_id: "4bf92f3577b34da6a3ce929d0e0e4736",
                span_id: "00f067aa0ba902b7",
                extra: true,
            ),
        );

        assert_eq!(
            "▓ 4bf92f ▓ 00f0 2024-01-01T01:02:04Z 1000ms span emit_term tests Hello, Rust\n",
            str::from_utf8(buf.as_slice()).unwrap()
        );
    }

    #[test]
    fn write_metric() {
        let mut buf = Buffer::no_color();

        write_event(
            &mut buf,
            emit::evt!(
                extent: emit::Timestamp::try_from_str("2024-01-01T01:02:03.000Z").unwrap(),
                "{metric_agg} of {metric_name} is {metric_value}",
                user: "Rust",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                metric_value: 42,
            ),
        );

        assert_eq!(
            "2024-01-01T01:02:03Z metric emit_term tests count of test is 42\n",
            str::from_utf8(buf.as_slice()).unwrap()
        );
    }
}
