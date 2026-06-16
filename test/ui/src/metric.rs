use emit::value::ToValue as _;

#[test]
fn metric_basic() {
    let my_metric = 42;

    let evt = emit::metric!(value: my_metric);
    assert_eq!(42, evt.value().to_value().cast::<usize>().unwrap());
    assert_eq!("my_metric", evt.name().unwrap());
    assert_eq!("last", evt.agg().unwrap());

    let evt = emit::count_metric!(value: my_metric);
    assert_eq!("count", evt.agg().unwrap());

    let evt = emit::sum_metric!(value: my_metric);
    assert_eq!("sum", evt.agg().unwrap());

    let evt = emit::min_metric!(value: my_metric);
    assert_eq!("min", evt.agg().unwrap());

    let evt = emit::max_metric!(value: my_metric);
    assert_eq!("max", evt.agg().unwrap());

    let evt = emit::last_metric!(value: my_metric);
    assert_eq!("last", evt.agg().unwrap());
}

#[test]
fn metric_description() {
    let my_metric = 42;
    let evt = emit::metric!(value: my_metric, description: "The number of requests");
    assert_eq!("The number of requests", evt.description().unwrap());
}

#[test]
fn metric_unit() {
    let my_metric = 42;
    let evt = emit::metric!(value: my_metric, unit: "milliseconds");
    assert_eq!("milliseconds", evt.unit().unwrap());
}

#[test]
fn metric_props() {
    let my_metric = 42;

    let evt = emit::metric!(
        value: my_metric,
        props: emit::props! {
            file: "./my_file",
        }
    );

    assert_eq!("./my_file", evt.props().file);
}
