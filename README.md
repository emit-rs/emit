> This crate implements a structured logging API similar to the one found in [Serilog](http://serilog.net). In systems programming, this style of logging is most often found in Windows' [ETW](https://msdn.microsoft.com/en-us/library/windows/desktop/aa363668(v=vs.85).aspx). Web and distributed applications use similar techniques to improve machine-readabililty when dealing with large event volumes.

"Emitted" log events consist of a _format_ and list of _named properties_, as in the `eminfo!()` call below.

```rust
#[macro_use]

[![Join the chat at https://gitter.im/emit-rs/emit](https://badges.gitter.im/emit-rs/emit.svg)](https://gitter.im/emit-rs/emit?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
extern crate emit;

use std::env;
use emit::PipelineBuilder;
use emit::collectors::seq;

fn main() {
    let _flush = PipelineBuilder::new()
        .at_level(emit::LogLevel::Info)
        .send_to(seq::SeqCollector::new_local())
        .init();
            
    eminfo!("Hello, {}!", name: env::var("USERNAME").unwrap());
}
```

The named arguments are captured as key/value properties that can be rendered in a structured format such as JSON:

```json
{
  "Timestamp": "2016-03-17T00:17:01Z",
  "Level": "Information",
  "MessageTemplate": "Hello, {name}!",
  "Properties": {
    "name": "nblumhardt",
    "target": "web_we_are"
  }
}
```

This makes log searches in an appropriate back-end collector much simpler:

![Event in Seq](https://raw.githubusercontent.com/nblumhardt/emit/master/asset/event_in_seq.png)

[Seq](https://getseq.net) and its JSON format are implemented for the work-in-progress here, but support for other log collectors and formats is on its way.

If you don't have Seq running, events can be written to `io::stdout` instead:

```rust
use emit::collectors::stdio::StdioCollector;
let _flush = PipelineBuilder::new()
    .write_to(StdioCollector::new())
    .init();
```

Produces:

```
emit 2016-03-24T05:03:36Z INFO  Hello, {name}!
  name: "nblumhardt"
  target: "web_we_are"
```

**What about the `log` crate?**

The `log!()` macros are the established way to capture diagnostic events in Rust today. However, `log` destructively renders events into text:

```rust
info!("Hello, {}!", env::var("USERNAME").unwrap());
```

There's no way for a log processing system to later pull the username value from this message, except through handwritten parsers/regular expressions.

The idea of `emit` is that rendering _can_ happen at any point - but the original values are preserved for easy machine processing as well.

To keep these two worlds in harmony, `emit` will be able to mirror events to `log` (#7).
