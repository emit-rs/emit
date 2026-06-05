# Key metric types

The key types involved in tracing are:

- [`Metric`](https://docs.rs/emit/1.20.0/emit/metric/struct.Metric.html): A kind of event representing a metric sample.
- [`Source`](https://docs.rs/emit/1.20.0/emit/metric/source/trait.Source.html): A producer of metric events that can be sampled.
- [`Reporter`](https://docs.rs/emit/1.20.0/emit/metric/struct.Reporter.html): A set of `Source`s that are sampled together. A `Reporter` will normalize timestamps on metric events so they line up. See [Reporting sources](./reporting-sources.md) for more details.
- [`Sampler`](https://docs.rs/emit/1.20.0/emit/metric/sampler/trait.Sampler.html): A receiver of metric events. A `Sampler` will typically forward on to an [`Emitter`](https://docs.rs/emit/1.20.0/emit/trait.Emitter.html).
- [`Delta`](https://docs.rs/emit/1.20.0/emit/metric/struct.Delta.html): A utility for simplifying the construction of metric events that report deltas instead of cumulative values. See [Delta metrics](./delta-metrics.md) for more details.
- [`Distribution`](https://docs.rs/emit/1.20.0/emit/metric/exp/struct.Distribution.html): A utility for simplifying the construction of exponential histograms. See [Distributions](./distributions.md) for more details.
