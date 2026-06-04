# Key logging types

The key types involved in `emit`'s logging API are:

- [`Event`](https://docs.rs/emit/1.19.0/emit/struct.Event.html): Logs are represented as events. See [Events](../../reference/events.md) for more details.
- [`Level`](https://docs.rs/emit/1.19.0/emit/enum.Level.html): A simplified severity enum associated with log events. See [Log levels](./levels.md) for more details.
- [`Emitter`](https://docs.rs/emit/1.19.0/emit/trait.Emitter.html): The receiver of events.
