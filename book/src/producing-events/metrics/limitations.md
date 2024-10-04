# Metrics limitations

`emit`'s metric model is intended to be simple, covering most key use-cases, but has some limitations compared to the OpenTelemetry model:

- No percentile histograms.
- Only one metric per event.
