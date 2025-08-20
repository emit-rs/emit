#[test]
fn new_sample() {
    let my_metric = 42;

    match emit::new_sample!(value: my_metric) {
        evt => {
            assert_eq!(42, evt.value().by_ref().cast::<usize>().unwrap());
            assert_eq!("my_metric", evt.name());
            assert_eq!("last", evt.agg());
        }
    }

    match emit::new_count_sample!(value: my_metric) {
        evt => {
            assert_eq!("count", evt.agg());
        }
    }

    match emit::new_sum_sample!(value: my_metric) {
        evt => {
            assert_eq!("sum", evt.agg());
        }
    }

    match emit::new_min_sample!(value: my_metric) {
        evt => {
            assert_eq!("min", evt.agg());
        }
    }

    match emit::new_max_sample!(value: my_metric) {
        evt => {
            assert_eq!("max", evt.agg());
        }
    }

    match emit::new_last_sample!(value: my_metric) {
        evt => {
            assert_eq!("last", evt.agg());
        }
    }
}
