# emit  [![Join the chat at https://gitter.im/emit-rs/emit](https://img.shields.io/gitter/room/emit/emit-rs.svg)](https://gitter.im/emit-rs/emit) [![Crates.io](https://img.shields.io/crates/v/emit.svg)](https://crates.io/crates/emit) [![Build status](https://travis-ci.org/emit-rs/emit.svg?branch=master)](https://travis-ci.org/emit-rs/emit) [![Documentation](https://img.shields.io/badge/docs-rustdoc-orange.svg)](http://emit-rs.github.io/emit/emit/index.html)

This crate implements a structured logging API similar to the one in [Serilog](http://serilog.net). Web and distributed applications use structured logging to improve machine-readabililty when dealing with large event volumes. Unlike many structured logging APIs, `emit`'s does this without sacrificing human-friendliness.


"Emitted" log events consist of a _format_ and list of _named properties_, as in the `info!()` call below.

```rust
#[macro_use]
extern crate emit;

use std::env;
use emit::PipelineBuilder;
use emit::collectors::seq;

fn main() {
    let _flush = PipelineBuilder::new()
        .at_level(emit::LogLevel::Info)
        .send_to(seq::SeqCollector::new_local())
        .init();
            
    info!("Hello, {}!", name: env::var("USERNAME").unwrap());
}
```

The event can be rendered into human-friendly text, while the named arguments are also captured as key/value properties when rendered in a structured format like JSON:

```json
{
  "@t": "2016-03-17T00:17:01Z",
  "@mt": "Hello, {name}!",
  "name": "nblumhardt",
  "target": "web_we_are"
}
```

This makes log searches in an appropriate back-end collector much simpler:

![Event in Seq](https://raw.githubusercontent.com/nblumhardt/emit/master/asset/event_in_seq.png)

### Collectors

Collectors render or store events to a wide range of targets. A `StdioCollector` is included in the `emit` crate and supports plain text or JSON formatting:

```rust
use emit::collectors::stdio::StdioCollector;
use formatters::text::PlainTextFormatter;

let _flush = PipelineBuilder::new()
    .write_to(StdioCollector::new(PlainTextFormatter::new()))
    .init();

eminfo!("Hello, {}!", name: env::var("USERNAME").unwrap());
```

Produces:

```
2016-03-24T05:03:36Z INFO  Hello, nblumhardt!
```

**All collectors**

| Description | Crate | Repository |
| ----------- | ----- | ---------- |
| ANSI (colored) terminal | [emit_ansi_term](https://crates.io/crates/emit_ansi_term) | [emit-rs/emit_ansi_term](https://github.com/emit-rs/emit_ansi_term) |
| [Elasticsearch](https://elastic.co) | [emit_elasticsearch](https://crates.io/crates/emit_elasticsearch) | [emit-rs/emit_elasticsearch](https://github.com/emit-rs/emit_elasticsearch) |
| [Seq](https://getseq.net) | [emit_seq](https://crates.io/crates/emit_seq) | [emit-rs/emit_seq](https://github.com/emit-rs/emit_seq) |
| STDIO | [emit](https://crates.io/crates/emit) | [emit-rs/emit](https://github.com/emit-rs/emit) |

### FAQ

**What's the status of `emit`?**

The project is undergoing rapid development and thus a fair amount of churn is still anticipated. If you're excited about this style of logging in Rust, we'd love for you to give it a go and share your feedback! Or, join the [Gitter channel](https://gitter.im/emit-rs/emit) to keep an eye on progress.

**How can I contribute?**

Contributions are welcome and appreciated. Check out our [issue list](https://github.com/emit-rs/emit/issues) to see if anything catches your interest, or raise a ticket to discuss anything else you would like to take on.

**What about the `log` crate?**

The `log!()` macros are the established way to capture diagnostic events in Rust today. However, `log` destructively renders events into text:

```rust
info!("Hello, {}!", env::var("USERNAME").unwrap());
```

There's no way for a log processing system to later pull the username value from this message, except through handwritten parsers/regular expressions.

The idea of `emit` is that rendering _can_ happen at any point - but the original values are preserved for easy machine processing as well.

To keep these two worlds in harmony, `emit` may eventually be able to mirror events to `log` in future ( #7).
