/*!
Emit diagnostic events to rolling files.

All file IO is performed on batches in a dedicated background thread.

This library writes newline delimited JSON by default, like:

```text
{"ts_start":"2024-05-29T03:35:13.922768000Z","ts":"2024-05-29T03:35:13.943506000Z","msg":"in_ctxt failed with `a` is odd","tpl":"in_ctxt failed with `err`","a":1,"err":"`a` is odd","lvl":"warn","span_id":"0a3686d1b788b277","span_parent":"1a50b58f2ef93f3b","trace_id":"8dd5d1f11af6ba1db4124072024933cb"}
```

# Getting started

Add `emit` and `emit_file` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.11.0-alpha.3"

[dependencies.emit_file]
version = "0.11.0-alpha.3"
```

Initialize `emit` using a rolling file set:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit_file::set("./target/logs/my_app.txt").spawn().unwrap())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

The input to [`set`] is a template for log file naming. The example earlier used `./target/logs/my_app.txt`. From this template, log files will be written to `./target/logs`, each log file name will start with `my_app`, and use `.txt` as its extension.

# File naming

Log files are created using the following naming scheme:

```text
{prefix}.{date}.{counter}.{id}.{ext}
```

where:

- `prefix`: A user-defined name that groups all log files related to the same application together.
- `date`: The rollover interval the file was created in. This isn't necessarily related to the timestamps of events within the file.
- `counter`: The number of milliseconds since the start of the current rollover interval when the file was created.
- `id`: A unique identifier for the file in the interval.
- `ext`: A user-defined file extension.

In the following log file:

```text
log.2024-05-27-03-00.00012557.37c57fa1.txt
```

the parts are:

- `prefix`: `log`.
- `date`: `2024-05-27-03-00`.
- `counter`: `00012557`.
- `id`: `37c57fa1`.
- `ext`: `txt`.

# When files roll

Diagnostic events are only ever written to a single file at a time. That file changes when:

1. The application restarts and [`FileSetBuilder::reuse_files`] is false.
2. The rollover period changes. This is set by [`FileSetBuilder::roll_by_day`], [`FileSetBuilder::roll_by_hour`], and [`FileSetBuilder::roll_by_minute`].
3. The size of the file exceeds [`FileSetBuilder::max_file_size_bytes`].
4. Writing to the file fails.

# Durability

Diagnostic events are written to files in asynchronous batches. Under normal operation, after a call to [`emit::Emitter::blocking_flush`], all events emitted before the call are guaranteed to be written and synced via Rust's [`std::fs::File::sync_all`] method. This is usually enough to guarantee durability.

# Handling IO failures

If writing a batch fails while attempting to write to a file then the file being written to is considered poisoned and no future attempts will be made to write to it. The batch will instead be retried on a new file. Batches that fail attempting to sync are not retried. Since batches don't have explicit transactions, it's possible on failure for part or all of the failed batch to actually be present in the original file. That means diagnostic events may be duplicated in the case of an IO error while writing them.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]

#![deny(missing_docs)]

mod internal_metrics;

use std::{
    fmt,
    fs::{self, File},
    io::{self, Write},
    mem,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use emit_batcher::BatchError;
use internal_metrics::InternalMetrics;

pub use internal_metrics::*;

/**
An error attempting to create a [`FileSet`].
*/
pub struct Error(Box<dyn std::error::Error + Send + Sync>);

impl Error {
    fn new(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Error(e.into())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/**
Create a builder for a [`FileSet`] using the default newline-delimited JSON format.

The builder will use `file_set` as its template for naming log files. See the crate root documentation for details on how this argument is interpreted.

It will use the other following defaults:

- Roll by hour.
- 32 max files.
- 1GiB max file size.

The [`FileSetBuilder`] has configuration options for managing the number and size of log files.

Once configured, call [`FileSetBuilder::spawn`] to complete the builder, passing the resulting [`FileSet`] to [`emit::Setup::emit_to`].
*/
#[cfg(feature = "default_writer")]
pub fn set(file_set: impl AsRef<Path>) -> FileSetBuilder {
    FileSetBuilder::new(file_set.as_ref())
}

/**
Create a builder for a [`FileSet`].

The builder will use `file_set` as its template for naming log files. See the crate root documentation for details on how this argument is interpreted.

The `writer` is used to format incoming [`emit::Event`]s into their on-disk format. If formatting fails then the event will be discarded.

The `separator` is written between individual events.
*/
pub fn set_with_writer(
    file_set: impl AsRef<Path>,
    writer: impl Fn(&mut FileBuf, &emit::Event<&dyn emit::props::ErasedProps>) -> io::Result<()>
        + Send
        + Sync
        + 'static,
    separator: &'static [u8],
) -> FileSetBuilder {
    FileSetBuilder::new_with_writer(file_set.as_ref(), writer, separator)
}

/**
A builder for a [`FileSet`].

Use [`set`] or [`set_with_writer`] to begin a [`FileSetBuilder`].

The [`FileSetBuilder`] has configuration options for managing the number and size of log files.

Once configured, call [`FileSetBuilder::spawn`] to complete the builder, passing the resulting [`FileSet`] to [`emit::Setup::emit_to`].
*/
pub struct FileSetBuilder {
    file_set: PathBuf,
    roll_by: RollBy,
    max_files: usize,
    max_file_size_bytes: usize,
    reuse_files: bool,
    writer: Box<
        dyn Fn(&mut FileBuf, &emit::Event<&dyn emit::props::ErasedProps>) -> io::Result<()>
            + Send
            + Sync,
    >,
    separator: &'static [u8],
}

#[derive(Debug, Clone, Copy)]
enum RollBy {
    Day,
    Hour,
    Minute,
}

const DEFAULT_ROLL_BY: RollBy = RollBy::Hour;
const DEFAULT_MAX_FILES: usize = 32;
const DEFAULT_MAX_FILE_SIZE_BYTES: usize = 1024 * 1024 * 1024; // 1GiB
const DEFAULT_REUSE_FILES: bool = false;

impl FileSetBuilder {
    /**
    Create a new [`FileSetBuilder`] using the default newline-delimited JSON format.

    The builder will use `file_set` as its template for naming log files. See the crate root documentation for details on how this argument is interpreted.

    It will use the other following defaults:

    - Roll by hour.
    - 32 max files.
    - 1GiB max file size.
    */
    #[cfg(feature = "default_writer")]
    pub fn new(file_set: impl Into<PathBuf>) -> Self {
        Self::new_with_writer(file_set, default_writer, b"\n")
    }

    /**
    Create a builder for a [`FileSet`].

    The builder will use `file_set` as its template for naming log files. See the crate root documentation for details on how this argument is interpreted.

    The `writer` is used to format incoming [`emit::Event`]s into their on-disk format. If formatting fails then the event will be discarded.

    The `separator` is written between individual events.

    It will use the other following defaults:

    - Roll by hour.
    - 32 max files.
    - 1GiB max file size.
    */
    pub fn new_with_writer(
        file_set: impl Into<PathBuf>,
        writer: impl Fn(&mut FileBuf, &emit::Event<&dyn emit::props::ErasedProps>) -> io::Result<()>
            + Send
            + Sync
            + 'static,
        separator: &'static [u8],
    ) -> Self {
        FileSetBuilder {
            file_set: file_set.into(),
            roll_by: DEFAULT_ROLL_BY,
            max_files: DEFAULT_MAX_FILES,
            max_file_size_bytes: DEFAULT_MAX_FILE_SIZE_BYTES,
            reuse_files: DEFAULT_REUSE_FILES,
            writer: Box::new(writer),
            separator,
        }
    }

    /**
    Create rolling log files based on the calendar day of when they're written to.
    */
    pub fn roll_by_day(mut self) -> Self {
        self.roll_by = RollBy::Day;
        self
    }

    /**
    Create rolling log files based on the calendar day and hour of when they're written to.
    */
    pub fn roll_by_hour(mut self) -> Self {
        self.roll_by = RollBy::Hour;
        self
    }

    /**
    Create rolling log files based on the calendar day, hour, and minute of when they're written to.
    */
    pub fn roll_by_minute(mut self) -> Self {
        self.roll_by = RollBy::Minute;
        self
    }

    /**
    The maximum number of log files to keep.

    Files are deleted from oldest first whenever a new file is created. Older files are determined based on the time period they belong to.
    */
    pub fn max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /**
    The maximum size of a file before new writes will roll over to a new one.

    The same time period can contain multiple log files. More recently created log files will sort ahead of older ones.
    */
    pub fn max_file_size_bytes(mut self, max_file_size_bytes: usize) -> Self {
        self.max_file_size_bytes = max_file_size_bytes;
        self
    }

    /**
    Whether to re-use log files when first attempting to write to them.

    This method can be used for applications that are started a lot and may result in lots of active log files.

    Before writing new events to the log file, it will have the configured separator defensively written to it. This ensures any previous partial write doesn't corrupt any new writes.
    */
    pub fn reuse_files(mut self, reuse_files: bool) -> Self {
        self.reuse_files = reuse_files;
        self
    }

    /**
    Specify a writer for incoming [`emit::Event`]s.

    The `writer` is used to format incoming [`emit::Event`]s into their on-disk format. If formatting fails then the event will be discarded.

    The `separator` is written between individual events.
    */
    pub fn writer(
        mut self,
        writer: impl Fn(&mut FileBuf, &emit::Event<&dyn emit::props::ErasedProps>) -> io::Result<()>
            + Send
            + Sync
            + 'static,
        separator: &'static [u8],
    ) -> Self {
        self.writer = Box::new(writer);
        self.separator = separator;
        self
    }

    /**
    Complete the builder, returning a [`FileSet`] to pass to [`emit::Setup::emit_to`].
    */
    pub fn spawn(self) -> Result<FileSet, Error> {
        let (dir, file_prefix, file_ext) = dir_prefix_ext(self.file_set).map_err(Error::new)?;

        let metrics = Arc::new(InternalMetrics::default());

        let mut worker = Worker::new(
            metrics.clone(),
            dir,
            file_prefix,
            file_ext,
            self.roll_by,
            self.reuse_files,
            self.max_files,
            self.max_file_size_bytes,
            self.separator,
        );

        let (sender, receiver) = emit_batcher::bounded(10_000);

        let handle = thread::spawn(move || {
            let _ = receiver.blocking_exec(|batch| worker.on_batch(batch));
        });

        Ok(FileSet {
            sender,
            metrics,
            writer: self.writer,
            separator: self.separator,
            _handle: handle,
        })
    }
}

/**
A handle to an asynchronous, background, rolling file writer.

Create a file set through the [`set`] function, calling [`FileSetBuilder::spawn`] to complete configuration. Pass the resulting [`FileSet`] to [`emit::Setup::emit_to`] to configure `emit` to write diagnostic events through it.
*/
pub struct FileSet {
    sender: emit_batcher::Sender<EventBatch>,
    metrics: Arc<InternalMetrics>,
    writer: Box<
        dyn Fn(&mut FileBuf, &emit::Event<&dyn emit::props::ErasedProps>) -> io::Result<()>
            + Send
            + Sync,
    >,
    separator: &'static [u8],
    _handle: thread::JoinHandle<()>,
}

impl emit::Emitter for FileSet {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        let mut buf = FileBuf::new();

        match (self.writer)(&mut buf, &evt.erase()) {
            Ok(()) => {
                if !buf.0.ends_with(self.separator) {
                    buf.extend_from_slice(self.separator);
                }

                self.sender.send(buf.into_boxed_slice());
            }
            Err(err) => {
                self.metrics.event_format_failed.increment();

                emit::warn!(
                    rt: emit::runtime::internal(),
                    "failed to format file event payload: {err}",
                )
            }
        }
    }

    fn blocking_flush(&self, timeout: std::time::Duration) -> bool {
        emit_batcher::sync::blocking_flush(&self.sender, timeout)
    }
}

impl FileSet {
    /**
    Get an [`emit::metric::Source`] for instrumentation produced by the file set.

    These metrics can be used to monitor the running health of your diagnostic pipeline.
    */
    pub fn metric_source(&self) -> FileSetMetrics {
        FileSetMetrics {
            channel_metrics: self.sender.metric_source(),
            metrics: self.metrics.clone(),
        }
    }
}

/**
A buffer to format [`emit::Event`]s into before writing them to a file.
*/
pub struct FileBuf(Vec<u8>);

impl FileBuf {
    fn new() -> Self {
        FileBuf(Vec::new())
    }

    /**
    Push a byte onto the end of the buffer.
    */
    pub fn push(&mut self, byte: u8) {
        self.0.push(byte)
    }

    /**
    Push a slice of bytes onto the end of the buffer.
    */
    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes)
    }

    fn into_boxed_slice(self) -> Box<[u8]> {
        self.0.into_boxed_slice()
    }
}

impl io::Write for FileBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[cfg(feature = "default_writer")]
fn default_writer(
    buf: &mut FileBuf,
    evt: &emit::Event<&dyn emit::props::ErasedProps>,
) -> io::Result<()> {
    use std::ops::ControlFlow;

    use emit::{
        well_known::{KEY_MSG, KEY_TPL, KEY_TS, KEY_TS_START},
        Props as _,
    };

    struct EventValue<'a, P>(&'a emit::Event<'a, P>);

    impl<'a, P: emit::Props> sval::Value for EventValue<'a, P> {
        fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(
            &'sval self,
            stream: &mut S,
        ) -> sval::Result {
            stream.record_begin(None, None, None, None)?;

            if let Some(extent) = self.0.extent() {
                let range = extent.as_range();

                if range.end != range.start {
                    stream.record_value_begin(None, &sval::Label::new(KEY_TS_START))?;
                    sval::stream_display(&mut *stream, &range.start)?;
                    stream.record_value_end(None, &sval::Label::new(KEY_TS_START))?;
                }

                stream.record_value_begin(None, &sval::Label::new(KEY_TS))?;
                sval::stream_display(&mut *stream, &range.end)?;
                stream.record_value_end(None, &sval::Label::new(KEY_TS))?;
            }

            stream.record_value_begin(None, &sval::Label::new(KEY_MSG))?;
            sval::stream_display(&mut *stream, self.0.msg())?;
            stream.record_value_end(None, &sval::Label::new(KEY_MSG))?;

            stream.record_value_begin(None, &sval::Label::new(KEY_TPL))?;
            sval::stream_display(&mut *stream, self.0.tpl())?;
            stream.record_value_end(None, &sval::Label::new(KEY_TPL))?;

            self.0.props().dedup().for_each(|k, v| {
                match (|| {
                    stream.record_value_begin(None, &sval::Label::new_computed(k.get()))?;
                    stream.value_computed(&v)?;
                    stream.record_value_end(None, &sval::Label::new_computed(k.get()))?;

                    Ok::<(), sval::Error>(())
                })() {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(_) => ControlFlow::Break(()),
                }
            });

            stream.record_end(None, None, None)
        }
    }

    sval_json::stream_to_io_write(buf, EventValue(evt))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}

struct EventBatch {
    bufs: Vec<Box<[u8]>>,
    remaining_bytes: usize,
    index: usize,
}

impl emit_batcher::Channel for EventBatch {
    type Item = Box<[u8]>;

    fn new() -> Self {
        EventBatch {
            bufs: Vec::new(),
            remaining_bytes: 0,
            index: 0,
        }
    }

    fn push<'a>(&mut self, item: Self::Item) {
        self.remaining_bytes += item.len();
        self.bufs.push(item);
    }

    fn len(&self) -> usize {
        self.bufs.len() - self.index
    }

    fn clear(&mut self) {
        self.bufs.clear()
    }
}

impl EventBatch {
    fn current(&self) -> Option<&[u8]> {
        self.bufs.get(self.index).map(|buf| &**buf)
    }

    fn advance(&mut self) {
        let advanced = mem::take(&mut self.bufs[self.index]);

        self.index += 1;
        self.remaining_bytes -= advanced.len();
    }
}

struct Worker {
    metrics: Arc<InternalMetrics>,
    active_file: Option<ActiveFile>,
    roll_by: RollBy,
    max_files: usize,
    max_file_size_bytes: usize,
    reuse_files: bool,
    dir: String,
    file_prefix: String,
    file_ext: String,
    separator: &'static [u8],
}

impl Worker {
    fn new(
        metrics: Arc<InternalMetrics>,
        dir: String,
        file_prefix: String,
        file_ext: String,
        roll_by: RollBy,
        reuse_files: bool,
        max_files: usize,
        max_file_size_bytes: usize,
        separator: &'static [u8],
    ) -> Self {
        Worker {
            metrics,
            active_file: None,
            roll_by,
            max_files,
            max_file_size_bytes,
            reuse_files,
            dir,
            file_prefix,
            file_ext,
            separator,
        }
    }

    #[emit::span(rt: emit::runtime::internal(), arg: span, "write file batch")]
    fn on_batch(&mut self, mut batch: EventBatch) -> Result<(), BatchError<EventBatch>> {
        let now = std::time::UNIX_EPOCH.elapsed().unwrap();
        let ts = emit::Timestamp::from_unix(now).unwrap();
        let parts = ts.to_parts();

        let file_ts = file_ts(self.roll_by, parts);

        let mut file = self.active_file.take();
        let mut file_set = ActiveFileSet::empty(&self.metrics, &self.dir);

        if file.is_none() {
            if let Err(err) = fs::create_dir_all(&self.dir) {
                span.complete_with(|span| {
                    emit::warn!(
                        rt: emit::runtime::internal(),
                        extent: span.extent(),
                        props: span.props(),
                        "failed to create root directory {path}: {err}",
                        #[emit::as_debug]
                        path: &self.dir,
                        err,
                    )
                });

                return Err(emit_batcher::BatchError::retry(err, batch));
            }

            let _ = file_set
                .read(&self.file_prefix, &self.file_ext)
                .map_err(|err| {
                    self.metrics.file_set_read_failed.increment();

                    emit::warn!(
                        rt: emit::runtime::internal(),
                        "failed to files in read {path}: {err}",
                        #[emit::as_debug]
                        path: &file_set.dir,
                        err,
                    );

                    err
                });

            if self.reuse_files {
                if let Some(file_name) = file_set.current_file_name() {
                    let mut path = PathBuf::from(&self.dir);
                    path.push(file_name);

                    file = ActiveFile::try_open_reuse(&path)
                        .map_err(|err| {
                            self.metrics.file_open_failed.increment();

                            emit::warn!(
                                rt: emit::runtime::internal(),
                                "failed to open {path}: {err}",
                                #[emit::as_debug]
                                path,
                                err,
                            );

                            err
                        })
                        .ok()
                }
            }
        }

        file = file.filter(|file| {
            file.file_size_bytes + batch.remaining_bytes <= self.max_file_size_bytes
                && file.file_ts == file_ts
        });

        let mut file = if let Some(file) = file {
            file
        } else {
            // Leave room for the file we're about to create
            file_set.apply_retention(self.max_files.saturating_sub(1));

            let mut path = PathBuf::from(self.dir.clone());

            let file_id = file_id(rolling_millis(self.roll_by, ts, parts), rolling_id());

            path.push(file_name(
                &self.file_prefix,
                &self.file_ext,
                &file_ts,
                &file_id,
            ));

            match ActiveFile::try_open_create(&path) {
                Ok(file) => {
                    self.metrics.file_create.increment();

                    emit::debug!(
                        rt: emit::runtime::internal(),
                        "created {path}",
                        #[emit::as_debug]
                        path: file.file_path,
                    );

                    file
                }
                Err(err) => {
                    self.metrics.file_create_failed.increment();

                    emit::warn!(
                        rt: emit::runtime::internal(),
                        "failed to create {path}: {err}",
                        #[emit::as_debug]
                        path,
                        err,
                    );

                    return Err(emit_batcher::BatchError::retry(err, batch));
                }
            }
        };

        let written_bytes = batch.remaining_bytes;

        while let Some(buf) = batch.current() {
            if let Err(err) = file.write_event(buf, self.separator) {
                self.metrics.file_write_failed.increment();

                span.complete_with(|span| {
                    emit::warn!(
                        rt: emit::runtime::internal(),
                        extent: span.extent(),
                        props: span.props(),
                        "failed to write event to {path}: {err}",
                        #[emit::as_debug]
                        path: file.file_path,
                        err,
                    )
                });

                return Err(emit_batcher::BatchError::retry(err, batch));
            }

            batch.advance();
        }

        file.file
            .flush()
            .map_err(|e| emit_batcher::BatchError::no_retry(e))?;
        file.file
            .sync_all()
            .map_err(|e| emit_batcher::BatchError::no_retry(e))?;

        span.complete_with(|span| {
            emit::debug!(
                rt: emit::runtime::internal(),
                extent: span.extent(),
                props: span.props(),
                "wrote {written_bytes} bytes to {path}",
                written_bytes,
                #[emit::as_debug]
                path: file.file_path,
            )
        });

        // Set the active file so the next batch can attempt to use it
        // At this point the file is expected to be valid
        self.active_file = Some(file);

        Ok(())
    }
}

struct ActiveFileSet<'a> {
    dir: &'a str,
    metrics: &'a InternalMetrics,
    file_set: Vec<String>,
}

impl<'a> ActiveFileSet<'a> {
    fn empty(metrics: &'a InternalMetrics, dir: &'a str) -> Self {
        ActiveFileSet {
            metrics,
            dir,
            file_set: Vec::new(),
        }
    }

    fn read(&mut self, file_prefix: &str, file_ext: &str) -> Result<(), io::Error> {
        self.file_set = Vec::new();

        let read_dir = fs::read_dir(&self.dir)?;

        let mut file_set = Vec::new();

        for entry in read_dir {
            let Ok(entry) = entry else {
                continue;
            };

            if let Ok(file_type) = entry.file_type() {
                if !file_type.is_file() {
                    continue;
                }
            }

            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                continue;
            };

            if file_name.starts_with(&file_prefix) && file_name.ends_with(&file_ext) {
                file_set.push(file_name.to_owned());
            }
        }

        file_set.sort_by(|a, b| a.cmp(b).reverse());

        self.file_set = file_set;

        Ok(())
    }

    fn current_file_name(&self) -> Option<&str> {
        // NOTE: If the clock shifts back (either jitters or through daylight savings)
        // Then we may return a file from the future here instead of one that better
        // matches the current timestamp. In these cases we'll end up creating a new file
        // instead of potentially reusing one that does match.
        self.file_set.first().map(|file_name| &**file_name)
    }

    fn apply_retention(&mut self, max_files: usize) {
        while self.file_set.len() >= max_files {
            let mut path = PathBuf::from(self.dir);
            path.push(self.file_set.pop().unwrap());

            if let Err(err) = fs::remove_file(&path) {
                self.metrics.file_delete_failed.increment();

                emit::warn!(
                    rt: emit::runtime::internal(),
                    "failed to delete {path}: {err}",
                    #[emit::as_debug]
                    path,
                    err,
                );
            } else {
                self.metrics.file_delete.increment();

                emit::debug!(
                    rt: emit::runtime::internal(),
                    "deleted {path}",
                    #[emit::as_debug]
                    path,
                );
            }
        }
    }
}

struct ActiveFile {
    file: File,
    file_path: PathBuf,
    file_ts: String,
    file_needs_recovery: bool,
    file_size_bytes: usize,
}

impl ActiveFile {
    fn try_open_reuse(file_path: impl AsRef<Path>) -> Result<ActiveFile, io::Error> {
        let file_path = file_path.as_ref();

        let file_ts = read_file_path_ts(file_path)?.to_owned();

        let file = fs::OpenOptions::new()
            .read(false)
            .append(true)
            .open(file_path)?;

        let file_size_bytes = file.metadata()?.len() as usize;

        Ok(ActiveFile {
            file,
            file_ts,
            file_path: file_path.into(),
            // The file is in an unknown state, so defensively assume
            // it needs to be recovered
            file_needs_recovery: true,
            file_size_bytes,
        })
    }

    fn try_open_create(file_path: impl AsRef<Path>) -> Result<ActiveFile, io::Error> {
        let file_path = file_path.as_ref();

        let file_ts = read_file_path_ts(file_path)?.to_owned();

        let file = fs::OpenOptions::new()
            .create_new(true)
            .read(false)
            .append(true)
            .open(file_path)?;

        Ok(ActiveFile {
            file,
            file_ts,
            file_path: file_path.into(),
            file_needs_recovery: false,
            file_size_bytes: 0,
        })
    }

    fn write_event(&mut self, event_buf: &[u8], separator: &'static [u8]) -> Result<(), io::Error> {
        // If the file may be corrupted then terminate
        // any previously written content with a separator.
        // This ensures the event that's about to be written
        // isn't mangled together with an incomplete one written
        // previously
        if self.file_needs_recovery {
            self.file_size_bytes += separator.len();
            self.file.write_all(separator)?;
        }

        self.file_needs_recovery = true;

        self.file_size_bytes += event_buf.len();
        self.file.write_all(event_buf)?;

        self.file_needs_recovery = false;
        Ok(())
    }
}

fn dir_prefix_ext(file_set: impl AsRef<Path>) -> Result<(String, String, String), Error> {
    let file_set = file_set.as_ref();

    let dir = if let Some(parent) = file_set.parent() {
        parent
            .to_str()
            .ok_or_else(|| "paths must be valid UTF8")
            .map_err(Error::new)?
            .to_owned()
    } else {
        String::new()
    };

    let prefix = file_set
        .file_stem()
        .ok_or_else(|| "paths must include a file name")
        .map_err(Error::new)?
        .to_str()
        .ok_or_else(|| "paths must be valid UTF8")
        .map_err(Error::new)?
        .to_owned();

    let ext = if let Some(ext) = file_set.extension() {
        ext.to_str()
            .ok_or_else(|| "paths must be valid UTF8")
            .map_err(Error::new)?
            .to_owned()
    } else {
        String::from("log")
    };

    Ok((dir, prefix, ext))
}

fn rolling_millis(roll_by: RollBy, ts: emit::Timestamp, parts: emit::timestamp::Parts) -> u32 {
    let truncated = match roll_by {
        RollBy::Day => emit::Timestamp::from_parts(emit::timestamp::Parts {
            years: parts.years,
            months: parts.months,
            days: parts.days,
            ..Default::default()
        })
        .unwrap(),
        RollBy::Hour => emit::Timestamp::from_parts(emit::timestamp::Parts {
            years: parts.years,
            months: parts.months,
            days: parts.days,
            hours: parts.hours,
            ..Default::default()
        })
        .unwrap(),
        RollBy::Minute => emit::Timestamp::from_parts(emit::timestamp::Parts {
            years: parts.years,
            months: parts.months,
            days: parts.days,
            hours: parts.hours,
            minutes: parts.minutes,
            ..Default::default()
        })
        .unwrap(),
    };

    ts.duration_since(truncated).unwrap().as_millis() as u32
}

fn rolling_id() -> u32 {
    rand::random()
}

fn file_ts(roll_by: RollBy, parts: emit::timestamp::Parts) -> String {
    match roll_by {
        RollBy::Day => format!(
            "{:>04}-{:>02}-{:>02}",
            parts.years, parts.months, parts.days,
        ),
        RollBy::Hour => format!(
            "{:>04}-{:>02}-{:>02}-{:>02}",
            parts.years, parts.months, parts.days, parts.hours,
        ),
        RollBy::Minute => format!(
            "{:>04}-{:>02}-{:>02}-{:>02}-{:>02}",
            parts.years, parts.months, parts.days, parts.hours, parts.minutes,
        ),
    }
}

fn file_id(rolling_millis: u32, rolling_id: u32) -> String {
    format!("{:<08}.{:<08x}", rolling_millis, rolling_id)
}

fn read_file_name_ts(file_name: &str) -> Result<&str, io::Error> {
    file_name.split('.').skip(1).next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "could not determine timestamp from filename",
        )
    })
}

fn read_file_path_ts(path: &Path) -> Result<&str, io::Error> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "unable to determine filename"))?
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "file names must be valid UTF8"))?;

    read_file_name_ts(file_name)
}

fn file_name(file_prefix: &str, file_ext: &str, ts: &str, id: &str) -> String {
    format!("{}.{}.{}.{}", file_prefix, ts, id, file_ext)
}
