> I'm learning Rust by implementing a structured logging API similar to the one found in [Serilog](http://serilog.net).

Log events consist of a format and list of *named* arguments:

```rust
#[macro_use]
extern crate emit;

use std::env;

fn main() {
    let _flush = emit::pipeline::init("http://localhost:5341/", None);
            
    emit!("Hello, {}!", name: env::var("USERNAME").unwrap());
}
```

These end up in JSON payloads like:

```json
{
  "Timestamp": "2016-03-17T00:17:01Z",
  "MessageTemplate": "Hello, {name}!",
  "Properties": {
    "name": "nblumhardt"
  }
}
```

Which can be rendered out to text or searched/sorted/filtered based on the event properties:

![Event in Seq](https://raw.githubusercontent.com/nblumhardt/emit/master/asset/event_in_seq.png)

I'm using Seq and its JSON format while I design the crate, but like Serilog this should eventually be pluggable to other log collectors and formats.

**What about the `log` crate?**

The `log!()` macros are obviously the best way to capture diagnostic events in Rust as it stands. However, `log` destructively renders events into text:

```rust
info!("Hello, {}!", env::var("USERNAME").unwrap());
```

There's no way for a log processing system to later pull the username value from this message except through handwritten parsers/regular expressions.

The idea of `emit` is that rendering _can_ happen at any point - but the original values are preserved for easy machine processing as well.
