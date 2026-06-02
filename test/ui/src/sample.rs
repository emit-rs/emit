use crate::util::{Called, simple_runtime};
use emit::{Kind, Props, Str};

#[allow(unused_imports)]
use crate::shadow::*;

#[test]
fn sample_basic() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                Kind::Metric,
                evt.props().pull::<Kind, _>("evt_kind").unwrap()
            );
            assert_eq!(42, evt.props().pull::<usize, _>("metric_value").unwrap());
            assert_eq!(
                "my_metric",
                evt.props().pull::<Str, _>("metric_name").unwrap()
            );
            assert_eq!("last", evt.props().pull::<Str, _>("metric_agg").unwrap());

            called.record();
        },
        |_| true,
    );

    let my_metric = 42;
    {
        match {
            #[allow(unused_imports)]
            use emit::__private::{
                __PrivateCaptureHook as _, __PrivateInterpolatedHook as _,
                __PrivateKeyExternalHook as _, __PrivateOptionalCaptureHook as _,
                __PrivateOptionalHook as _,
            };
            (my_metric)
                .__private_capture_as_default()
                .__private_key_external()
                .__private_uninterpolated()
                .__private_captured()
        } {
            __tmp_value => emit::__private::__private_sample(
                emit::__private::__private_default_sampler(&(rt)),
                ::emit::Path::new_raw("emit_test_ui::sample"),
                &(emit::Empty),
                &(emit::Empty),
                "my_metric",
                "last",
                __tmp_value,
            ),
        }
    };

    assert!(called.was_called());
}

#[test]
fn sample_value_capture() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                "MyValue",
                evt.props().get("metric_value").unwrap().to_string()
            );

            called.record();
        },
        |_| true,
    );

    #[derive(Debug)]
    struct MyValue;

    let my_metric = MyValue;
    emit::sample!(rt, #[emit::as_debug] value: my_metric);

    assert!(called.was_called());
}

#[test]
fn sample_agg() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            assert_eq!("count", evt.props().pull::<Str, _>("metric_agg").unwrap());

            called.record();
        },
        |_| true,
    );

    emit::sample!(rt, name: "my_metric", value: 42, agg: "count");

    assert!(called.was_called());
}

#[test]
fn sample_name() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                "my_other_metric",
                evt.props().pull::<Str, _>("metric_name").unwrap()
            );

            called.record();
        },
        |_| true,
    );

    let my_metric = 42;
    emit::sample!(rt, name: "my_other_metric", value: my_metric);

    assert!(called.was_called());
}

#[test]
fn sample_props() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            assert_eq!(true, evt.props().pull::<bool, _>("a").unwrap());
            assert_eq!(1, evt.props().pull::<i32, _>("b").unwrap());

            called.record();
        },
        |_| true,
    );

    emit::sample!(
        rt,
        name: "my_metric",
        value: 42,
        props: emit::props! {
            a: true,
            b: 1,
        },
    );

    assert!(called.was_called());
}

#[test]
fn sample_well_known_props_precedence() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                Kind::Metric,
                evt.props().pull::<Kind, _>("evt_kind").unwrap()
            );
            assert_eq!(42, evt.props().pull::<usize, _>("metric_value").unwrap());
            assert_eq!(
                "my_metric",
                evt.props().pull::<Str, _>("metric_name").unwrap()
            );
            assert_eq!("count", evt.props().pull::<Str, _>("metric_agg").unwrap());

            called.record();
        },
        |_| true,
    );

    emit::sample!(
        rt,
        name: "my_metric",
        value: 42,
        agg: "count",
        props: emit::props! {
            metric_name: "my_other_metric",
            metric_agg: "sum",
            metric_value: 13,
            evt_kind: "custom_kind",
        },
    );

    assert!(called.was_called());
}

#[test]
fn sample_agg_specific() {
    let called = Called::new();

    let rt = simple_runtime(
        |evt| {
            let agg = evt.props().pull::<Str, _>("metric_agg").unwrap();
            let expected = evt.props().pull::<Str, _>("expected_agg").unwrap();

            assert_eq!(agg, expected);
            called.record();
        },
        |_| true,
    );

    emit::count_sample!(rt, name: "my_metric", value: 42, props: emit::props! { expected_agg: "count" });
    emit::sum_sample!(rt, name: "my_metric", value: 42, props: emit::props! { expected_agg: "sum" });
    emit::min_sample!(rt, name: "my_metric", value: 42, props: emit::props! { expected_agg: "min" });
    emit::max_sample!(rt, name: "my_metric", value: 42, props: emit::props! { expected_agg: "max" });
    emit::last_sample!(rt, name: "my_metric", value: 42, props: emit::props! { expected_agg: "last" });

    assert!(called.was_called());
}
