#[test]
fn metric() {
    let my_metric = 42;

    match emit::metric!(value: my_metric) {
        evt => {
            assert_eq!(42, evt.value().by_ref().cast::<usize>().unwrap());
            assert_eq!("my_metric", evt.name());
            assert_eq!("last", evt.agg());
        }
    }

    match emit::count_metric!(value: my_metric) {
        evt => {
            assert_eq!("count", evt.agg());
        }
    }

    match emit::sum_metric!(value: my_metric) {
        evt => {
            assert_eq!("sum", evt.agg());
        }
    }

    match emit::min_metric!(value: my_metric) {
        evt => {
            assert_eq!("min", evt.agg());
        }
    }

    match emit::max_metric!(value: my_metric) {
        evt => {
            assert_eq!("max", evt.agg());
        }
    }

    match emit::last_metric!(value: my_metric) {
        evt => {
            assert_eq!("last", evt.agg());
        }
    }
}
