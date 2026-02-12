# Logging data model

The data model of logs is an extension of [`emit`'s events](../../reference/events.md). Log events include the following [well-known properties](https://docs.rs/emit/1.17.2/emit/well_known/index.html):

- `lvl`: a severity level assigned to the event.
    - `"debug"`: a weakly informative event for live debugging.
    - `"info"`: an informative event.
    - `"warn"`: a weakly erroneous event for non-critical operations.
    - `"error"`: an erroneous event.
- `err`: an error associated with the event.

There's some overlap between the logs data model and other extensions. [Span events](../tracing.md), for example, also support attaching levels and errors through `lvl` and `err`.
