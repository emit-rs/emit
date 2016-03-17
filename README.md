I'm learning Rust by implementing a structured logging API similar to the one found in [Serilog](http://serilog.net).

At the moment, just "making it work" end-to-end is the goal, with events like:

```rust
emit!("Hello, {}!", name: "nblumhardt");
```

Being sent to [Seq](https://getseq.net) as JSON payloads like:

```json
{
  "Timestamp": "2016-03-17T00:17:01Z",
  "MessageTemplate": "Hello, {name}!",
  "Properties": {
    "name": "nblumhardt"
  }
}
```

At present, the `emit!` macro mostly works, as does JSON payload formatting. Everything else is work-in progress :-)
