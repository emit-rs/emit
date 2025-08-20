# Syntax

```text
(control_param),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).

# Control parameters

This macro accepts the following optional control parameters:

| name                                                  | type                          | description                                                                                                                                     |
|-------------------------------------------------------| ----------------------------- |-------------------------------------------------------------------------------------------------------------------------------------------------|
| `rt`                                                  | `impl emit::runtime::Runtime` | The runtime to emit the metric sample through if `sampler` is unspecified.                                                                      |
| `sampler`                                             | `impl emit::sampler::Sampler` | The `Sampler` to pass the metric sample to.                                                                                                     |
| `mdl`                                                 | `impl Into<emit::Path>`       | The module the metric sample belongs to. If unspecified the current module path is used.                                                        |
| `extent`                                              | `impl emit::ToExtent`         | The extent to use on the metric sample. If unspecified the extent is left empty.                                                                |
| `props`                                               | `impl emit::Props`            | A set of properties to add to the metric sample.                                                                                                |
| `value` (**required**)                                | `impl emit::ToValue`          | The value of the metric sample. If the value is an identifier then `name` will be inferred to be that identifier.                        |
| `name` (**required** if `value` is not an identifier) | `impl emit::ToStr`            | The name of the metric being sampled. If unspecified, and `value` is an identifier, then the stringified identifier is used as the name. |
| `agg`                                                 | `impl emit::ToStr`            | The aggregation of the metric sample. If unspecified, the default for the macro is used.                                                        |
