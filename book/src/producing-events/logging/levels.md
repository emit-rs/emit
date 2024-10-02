# Log levels

Levels describing the significance of a log event are a coarse-grained way to organize and filter them. `emit` doesn't bake in the concept of log levels directly, but supports them through the `lvl` [well-known property](https://docs.rs/emit/0.11.0-alpha.17/emit/well_known/index.html). The well-known levels are:

- `"debug"`: 
- `"info"`: 
- `"warn"`: 
- `"error"`: 

When emitting events, you can use the following macros to have an appropriate level attached to your events automatically:

- [`debug!`]()
- [`info!`]()
- [`warn!`]()
- [`error!`]()
