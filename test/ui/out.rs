#![feature(prelude_import)]
/*!
Integration tests for `emit`'s macros.

Compile-pass tests mostly live in top-level modules here. Compile-fail tests live under the `compile_fail` module.
*/
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
mod util {
    use emit::{Clock, Ctxt, Event, Props, Rng, Str, Timestamp, Value};
    use emit::props::ErasedProps;
    use emit::runtime::Runtime;
    use std::{
        cell::RefCell, cmp, collections::HashMap, mem, ops::ControlFlow,
        sync::{Arc, LazyLock, Mutex, atomic::{AtomicU64, Ordering}},
        time::Duration,
    };
    pub type SimpleRuntime<E, F> = Runtime<E, F, SimpleCtxt, CountingClock, CountingRng>;
    pub type StaticRuntime = SimpleRuntime<emit::emitter::FromFn, emit::filter::FromFn>;
    pub const fn static_runtime(
        emitter: fn(Event<&dyn ErasedProps>),
        filter: fn(Event<&dyn ErasedProps>) -> bool,
    ) -> StaticRuntime {
        Runtime::build(
            emit::emitter::FromFn::new(emitter),
            emit::filter::FromFn::new(filter),
            SimpleCtxt::new(),
            CountingClock::new(),
            CountingRng::new(),
        )
    }
    pub const fn simple_runtime<
        E: Fn(Event<&dyn ErasedProps>),
        F: Fn(Event<&dyn ErasedProps>) -> bool,
    >(
        emitter: E,
        filter: F,
    ) -> SimpleRuntime<emit::emitter::FromFn<E>, emit::filter::FromFn<F>> {
        Runtime::build(
            emit::emitter::FromFn::new(emitter),
            emit::filter::FromFn::new(filter),
            SimpleCtxt::new(),
            CountingClock::new(),
            CountingRng::new(),
        )
    }
    pub(crate) struct Called(Arc<Mutex<usize>>);
    #[automatically_derived]
    impl ::core::clone::Clone for Called {
        #[inline]
        fn clone(&self) -> Called {
            Called(::core::clone::Clone::clone(&self.0))
        }
    }
    impl Called {
        pub(crate) fn new() -> Self {
            Called(Arc::new(Mutex::new(0)))
        }
        pub(crate) fn record(&self) {
            *self.0.lock().unwrap() += 1;
        }
        pub(crate) fn called_times(&self) -> usize {
            *self.0.lock().unwrap()
        }
        pub(crate) fn was_called(&self) -> bool {
            self.called_times() > 0
        }
    }
    pub(crate) struct StaticCalled(LazyLock<Called>);
    impl StaticCalled {
        pub(crate) const fn new() -> Self {
            StaticCalled(LazyLock::new(Called::new))
        }
        pub(crate) fn record(&self) {
            self.0.record()
        }
        pub(crate) fn called_times(&self) -> usize {
            self.0.called_times()
        }
        pub(crate) fn was_called(&self) -> bool {
            self.0.was_called()
        }
    }
    pub struct CountingClock(AtomicU64);
    impl CountingClock {
        pub const fn new() -> Self {
            CountingClock(AtomicU64::new(0))
        }
    }
    impl Clock for CountingClock {
        fn now(&self) -> Option<Timestamp> {
            Timestamp::from_unix(
                Duration::from_secs(self.0.fetch_add(1, Ordering::Relaxed)),
            )
        }
    }
    pub struct CountingRng(AtomicU64);
    impl CountingRng {
        pub const fn new() -> Self {
            CountingRng(AtomicU64::new(0))
        }
    }
    impl Rng for CountingRng {
        fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
            let mut buf = arr.as_mut();
            while buf.len() > 0 {
                let v = self.0.fetch_add(1, Ordering::Relaxed).to_le_bytes();
                let len = cmp::min(buf.len(), v.len());
                buf[..len].copy_from_slice(&v[..len]);
                buf = &mut buf[len..];
            }
            Some(arr)
        }
    }
    pub struct SimpleCtxt {}
    const SIMPLE_CTXT_CURRENT: ::std::thread::LocalKey<RefCell<SimpleCtxtProps>> = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<SimpleCtxtProps> {
            RefCell::new(SimpleCtxtProps(HashMap::new()))
        }
        unsafe {
            ::std::thread::LocalKey::new(const {
                if ::std::mem::needs_drop::<RefCell<SimpleCtxtProps>>() {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            RefCell<SimpleCtxtProps>,
                            (),
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(
                                __rust_std_internal_init,
                                __rust_std_internal_init_fn,
                            )
                    }
                } else {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            RefCell<SimpleCtxtProps>,
                            !,
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(
                                __rust_std_internal_init,
                                __rust_std_internal_init_fn,
                            )
                    }
                }
            })
        }
    };
    impl SimpleCtxt {
        pub const fn new() -> Self {
            SimpleCtxt {}
        }
        fn current(&self) -> SimpleCtxtProps {
            SIMPLE_CTXT_CURRENT.with(|current| current.borrow().clone())
        }
        fn swap(&self, incoming: &mut SimpleCtxtProps) {
            SIMPLE_CTXT_CURRENT
                .with(|current| mem::swap(&mut *current.borrow_mut(), incoming))
        }
    }
    pub struct SimpleCtxtFrame(SimpleCtxtProps);
    pub struct SimpleCtxtProps(HashMap<String, String>);
    #[automatically_derived]
    impl ::core::clone::Clone for SimpleCtxtProps {
        #[inline]
        fn clone(&self) -> SimpleCtxtProps {
            SimpleCtxtProps(::core::clone::Clone::clone(&self.0))
        }
    }
    impl Props for SimpleCtxtProps {
        fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            mut for_each: F,
        ) -> ControlFlow<()> {
            for (key, value) in &self.0 {
                for_each(Str::new_ref(key), Value::from(&**value))?;
            }
            ControlFlow::Continue(())
        }
    }
    impl Ctxt for SimpleCtxt {
        type Current = SimpleCtxtProps;
        type Frame = SimpleCtxtFrame;
        fn open_root<P: Props>(&self, props: P) -> Self::Frame {
            let mut serialized = HashMap::new();
            let _ = props
                .for_each(|k, v| {
                    if !serialized.contains_key(k.get()) {
                        serialized.insert(k.get().into(), v.to_string());
                    }
                    ControlFlow::Continue(())
                });
            SimpleCtxtFrame(SimpleCtxtProps(serialized))
        }
        fn enter(&self, local: &mut Self::Frame) {
            self.swap(&mut local.0);
        }
        fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
            with(&self.current())
        }
        fn exit(&self, local: &mut Self::Frame) {
            self.swap(&mut local.0)
        }
        fn close(&self, _: Self::Frame) {}
    }
}
mod emit {
    use ::std::time::Duration;
    use emit::{Emitter, Props};
    use crate::util::{Called, simple_runtime};
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "emit::emit_basic"]
    #[doc(hidden)]
    pub const emit_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 11usize,
            start_col: 4usize,
            end_line: 11usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_basic()),
        ),
    };
    fn emit_basic() {
        for lvl in [
            ::std::option::Option::Some(emit::Level::Debug),
            ::std::option::Option::Some(emit::Level::Info),
            ::std::option::Option::Some(emit::Level::Warn),
            ::std::option::Option::Some(emit::Level::Error),
            ::std::option::Option::None,
        ] {
            let called = Called::new();
            let rt = simple_runtime(
                |evt| {
                    match (&"Hello, Rust", &evt.msg().to_string()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&"Hello, {user}", &evt.tpl().to_string()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&"emit_test_ui::emit", &evt.mdl()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    if !evt.extent().is_some() {
                        ::core::panicking::panic(
                            "assertion failed: evt.extent().is_some()",
                        )
                    }
                    match (&"Rust", &evt.props().pull::<&str, _>("user").unwrap()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&lvl, &evt.props().pull::<emit::Level, _>("lvl")) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    called.record();
                },
                |_| true,
            );
            let user = "Rust";
            match lvl {
                ::std::option::Option::None => {
                    match ({
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("user")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                (user)
                                    .__private_capture_as_default()
                                    .__private_key_external()
                                    .__private_interpolated()
                                    .__private_captured()
                            },
                        )
                    }) {
                        (__tmp0) => {
                            emit::__private::__private_emit(
                                &(rt),
                                &(::emit::Path::new_raw("emit_test_ui::emit")),
                                emit::__private::core::option::Option::None::<&emit::Empty>,
                                &(emit::Empty),
                                &(emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                })),
                                &(emit::Empty),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    (__tmp0.0, __tmp0.1),
                                ])),
                            );
                        }
                    }
                }
                ::std::option::Option::Some(emit::Level::Debug) => {
                    match (
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("user")
                                        .__private_key_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (user)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            )
                        },
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("lvl")
                                        .__private_key_as_default()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (emit::Level::Debug)
                                        .__private_capture_as_level()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            )
                        },
                    ) {
                        (__tmp0, __tmp1) => {
                            emit::__private::__private_emit(
                                &(rt),
                                &(::emit::Path::new_raw("emit_test_ui::emit")),
                                emit::__private::core::option::Option::None::<&emit::Empty>,
                                &(emit::Empty),
                                &(emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                })),
                                &(emit::Empty),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    (__tmp1.0, __tmp1.1),
                                    (__tmp0.0, __tmp0.1),
                                ])),
                            );
                        }
                    }
                }
                ::std::option::Option::Some(emit::Level::Info) => {
                    match (
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("user")
                                        .__private_key_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (user)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            )
                        },
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("lvl")
                                        .__private_key_as_default()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (emit::Level::Info)
                                        .__private_capture_as_level()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            )
                        },
                    ) {
                        (__tmp0, __tmp1) => {
                            emit::__private::__private_emit(
                                &(rt),
                                &(::emit::Path::new_raw("emit_test_ui::emit")),
                                emit::__private::core::option::Option::None::<&emit::Empty>,
                                &(emit::Empty),
                                &(emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                })),
                                &(emit::Empty),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    (__tmp1.0, __tmp1.1),
                                    (__tmp0.0, __tmp0.1),
                                ])),
                            );
                        }
                    }
                }
                ::std::option::Option::Some(emit::Level::Warn) => {
                    match (
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("user")
                                        .__private_key_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (user)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            )
                        },
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("lvl")
                                        .__private_key_as_default()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (emit::Level::Warn)
                                        .__private_capture_as_level()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            )
                        },
                    ) {
                        (__tmp0, __tmp1) => {
                            emit::__private::__private_emit(
                                &(rt),
                                &(::emit::Path::new_raw("emit_test_ui::emit")),
                                emit::__private::core::option::Option::None::<&emit::Empty>,
                                &(emit::Empty),
                                &(emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                })),
                                &(emit::Empty),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    (__tmp1.0, __tmp1.1),
                                    (__tmp0.0, __tmp0.1),
                                ])),
                            );
                        }
                    }
                }
                ::std::option::Option::Some(emit::Level::Error) => {
                    match (
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("user")
                                        .__private_key_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (user)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            )
                        },
                        {
                            (
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("lvl")
                                        .__private_key_as_default()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (emit::Level::Error)
                                        .__private_capture_as_level()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            )
                        },
                    ) {
                        (__tmp0, __tmp1) => {
                            emit::__private::__private_emit(
                                &(rt),
                                &(::emit::Path::new_raw("emit_test_ui::emit")),
                                emit::__private::core::option::Option::None::<&emit::Empty>,
                                &(emit::Empty),
                                &(emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                })),
                                &(emit::Empty),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    (__tmp1.0, __tmp1.1),
                                    (__tmp0.0, __tmp0.1),
                                ])),
                            );
                        }
                    }
                }
            }
            rt.emitter().blocking_flush(Duration::from_secs(1));
            if !called.was_called() {
                ::core::panicking::panic("assertion failed: called.was_called()")
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_interpolation"]
    #[doc(hidden)]
    pub const emit_interpolation: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_interpolation"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 55usize,
            start_col: 4usize,
            end_line: 55usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_interpolation()),
        ),
    };
    fn emit_interpolation() {
        let rt = simple_runtime(
            |evt| {
                match (&"Rust", &evt.props().get("user").unwrap().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        let user = "Rust";
        {
            match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                "Rust",
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                ::std::string::String::from("Rust"),
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                "Rust",
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                ::std::string::String::from("Rust"),
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                { user },
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                { "Rust" },
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                { "Rust" },
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                { ::std::string::String::from("Rust") },
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
        let user = ::std::string::String::from("Rust");
        {
            match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as_default()
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                    );
                }
            }
        };
        drop(user);
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_cfg"]
    #[doc(hidden)]
    pub const emit_cfg: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_cfg"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 83usize,
            start_col: 4usize,
            end_line: 83usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_cfg()),
        ),
    };
    fn emit_cfg() {
        let rt = simple_runtime(
            |evt| {
                match (&"Hello, , true", &evt.msg().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        {
            match (
                (),
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("enabled")
                                .__private_key_as_default()
                                .__private_interpolated()
                                .__private_captured()
                        },
                        true,
                    )
                },
            ) {
                (__tmp0, __tmp1) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                emit::template::Part::text(", ")
                                    .with_needs_escaping_raw(false),
                                {
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("enabled")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    }
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp1.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp1.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_key"]
    #[doc(hidden)]
    pub const emit_key: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_key"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 100usize,
            start_col: 4usize,
            end_line: 100usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_key()),
        ),
    };
    fn emit_key() {
        let rt = simple_runtime(
            |evt| {
                match (&"Hello, {user.name}", &evt.tpl().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"Hello, Rust", &evt.msg().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as(emit::Str::new("user.name"))
                        .__private_interpolated()
                        .__private_captured()
                },
                "Rust",
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as(emit::Str::new("user.name"))
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_key_exotic"]
    #[doc(hidden)]
    pub const emit_key_exotic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_key_exotic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 117usize,
            start_col: 4usize,
            end_line: 117usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_key_exotic()),
        ),
    };
    fn emit_key_exotic() {
        let rt = simple_runtime(
            |evt| {
                match (&"Hello, {{user}}", &evt.tpl().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"Hello, Rust", &evt.msg().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        {
            match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as(emit::Str::new("{user}"))
                        .__private_interpolated()
                        .__private_captured()
                },
                "Rust",
            )) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("Hello, ")
                                    .with_needs_escaping_raw(false),
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::template::Part::hole_str(
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("user")
                                                    .__private_key_as(emit::Str::new("{user}"))
                                                    .__private_interpolated()
                                                    .__private_captured()
                                            },
                                        )
                                        .__private_fmt_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_empty"]
    #[doc(hidden)]
    pub const emit_empty: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_empty"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 134usize,
            start_col: 4usize,
            end_line: 134usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_empty()),
        ),
    };
    fn emit_empty() {
        let rt = simple_runtime(
            |evt| {
                match (&"Rust", &evt.props().get("user").unwrap().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"", &evt.msg().to_string()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        let user = "Rust";
        {
            match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_rt_ref"]
    #[doc(hidden)]
    pub const emit_rt_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_rt_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 149usize,
            start_col: 4usize,
            end_line: 149usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_rt_ref()),
        ),
    };
    fn emit_rt_ref() {
        let called = Called::new();
        let rt = simple_runtime(|_| called.record(), |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(&rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        rt.emitter().blocking_flush(Duration::from_secs(1));
        match (&1, &called.called_times()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_filter"]
    #[doc(hidden)]
    pub const emit_filter: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_filter"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 161usize,
            start_col: 4usize,
            end_line: 161usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_filter()),
        ),
    };
    fn emit_filter() {
        let called = Called::new();
        let rt = simple_runtime(|_| called.record(), |evt| evt.mdl() == "true");
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(emit::Path::new_raw("false")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(emit::Path::new_raw("true")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        rt.emitter().blocking_flush(Duration::from_secs(1));
        match (&1, &called.called_times()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_when"]
    #[doc(hidden)]
    pub const emit_when: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_when"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 174usize,
            start_col: 4usize,
            end_line: 174usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_when()),
        ),
    };
    fn emit_when() {
        let called = Called::new();
        let rt = simple_runtime(|_| called.record(), |evt| evt.mdl() == "true");
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(emit::Path::new_raw("false")),
                        emit::__private::core::option::Option::Some(
                            &(emit::filter::from_fn(|_| true)),
                        ),
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        rt.emitter().blocking_flush(Duration::from_secs(1));
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_when_ref"]
    #[doc(hidden)]
    pub const emit_when_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_when_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 186usize,
            start_col: 4usize,
            end_line: 186usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_when_ref()),
        ),
    };
    fn emit_when_ref() {
        let rt = simple_runtime(|_| {}, |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::Some(
                            &(&emit::filter::from_fn(|_| true)),
                        ),
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_extent_point"]
    #[doc(hidden)]
    pub const emit_extent_point: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_extent_point"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 193usize,
            start_col: 4usize,
            end_line: 193usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_extent_point()),
        ),
    };
    fn emit_extent_point() {
        let rt = simple_runtime(
            |evt| {
                match (
                    &emit::Timestamp::from_unix(Duration::from_secs(42)).unwrap(),
                    &evt.extent().unwrap().as_point(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Timestamp::from_unix(Duration::from_secs(42))),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_extent_point_ref"]
    #[doc(hidden)]
    pub const emit_extent_point_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_extent_point_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 212usize,
            start_col: 4usize,
            end_line: 212usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_extent_point_ref()),
        ),
    };
    fn emit_extent_point_ref() {
        let rt = simple_runtime(|_| {}, |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(&emit::Timestamp::from_unix(Duration::from_secs(42))),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_extent_span"]
    #[doc(hidden)]
    pub const emit_extent_span: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_extent_span"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 223usize,
            start_col: 4usize,
            end_line: 223usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_extent_span()),
        ),
    };
    fn emit_extent_span() {
        let rt = simple_runtime(
            |evt| {
                match (
                    &(emit::Timestamp::from_unix(Duration::from_secs(42))
                        .unwrap()..emit::Timestamp::from_unix(Duration::from_secs(47))
                        .unwrap()),
                    &evt.extent().unwrap().as_range().unwrap().clone(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Timestamp::from_unix(
                            Duration::from_secs(42),
                        )..emit::Timestamp::from_unix(Duration::from_secs(47))),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_extent_span_ref"]
    #[doc(hidden)]
    pub const emit_extent_span_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_extent_span_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 243usize,
            start_col: 4usize,
            end_line: 243usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_extent_span_ref()),
        ),
    };
    fn emit_extent_span_ref() {
        let rt = simple_runtime(|_| {}, |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(&(emit::Timestamp::from_unix(
                            Duration::from_secs(42),
                        )..emit::Timestamp::from_unix(Duration::from_secs(47)))),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_mdl"]
    #[doc(hidden)]
    pub const emit_mdl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_mdl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 254usize,
            start_col: 4usize,
            end_line: 254usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_mdl()),
        ),
    };
    fn emit_mdl() {
        let rt = simple_runtime(
            |evt| {
                match (&"custom_module", &evt.mdl()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |evt| {
                match (&"custom_module", &evt.mdl()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(emit::Path::new_raw("custom_module")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_mdl_ref"]
    #[doc(hidden)]
    pub const emit_mdl_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_mdl_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 270usize,
            start_col: 4usize,
            end_line: 270usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_mdl_ref()),
        ),
    };
    fn emit_mdl_ref() {
        let rt = simple_runtime(|_| {}, |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(&emit::Path::new_raw("custom_module")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_props"]
    #[doc(hidden)]
    pub const emit_props: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_props"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 277usize,
            start_col: 4usize,
            end_line: 277usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_props()),
        ),
    };
    fn emit_props() {
        fn assert_props(evt: &emit::Event<impl emit::Props>) {
            match (&1, &evt.props().pull::<i32, _>("ambient_prop1").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&2, &evt.props().pull::<i32, _>("ambient_prop2").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&1, &evt.props().pull::<i32, _>("evt_prop1").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&2, &evt.props().pull::<i32, _>("evt_prop2").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        }
        let rt = simple_runtime(
            |evt| assert_props(&evt),
            |evt| {
                assert_props(&evt);
                true
            },
        );
        {
            match (
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("evt_prop1")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    1,
                ),
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("evt_prop2")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    2,
                ),
            ) {
                (__tmp0, __tmp1) => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("ambient_prop1")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (1)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("ambient_prop2")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (2)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                            (
                                __tmp1.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp1.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_props_ref"]
    #[doc(hidden)]
    pub const emit_props_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_props_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 308usize,
            start_col: 4usize,
            end_line: 308usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_props_ref()),
        ),
    };
    fn emit_props_ref() {
        let rt = simple_runtime(|_| {}, |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit(
                        &(rt),
                        &(::emit::Path::new_raw("emit_test_ui::emit")),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::Empty),
                        &(emit::Template::new_ref({
                            const __TPL_PARTS: &[emit::template::Part] = &[
                                emit::template::Part::text("test")
                                    .with_needs_escaping_raw(false),
                            ];
                            __TPL_PARTS
                        })),
                        &(&emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("ambient_prop1")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (1)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("ambient_prop2")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (2)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_evt"]
    #[doc(hidden)]
    pub const emit_evt: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_evt"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 322usize,
            start_col: 4usize,
            end_line: 322usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_evt()),
        ),
    };
    fn emit_evt() {
        fn assert_evt(evt: &emit::Event<impl emit::Props>) {
            match (&"Hello, Rust", &evt.msg().to_string()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&"Hello, {user}", &evt.tpl().to_string()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&"emit_test_ui::emit", &evt.mdl()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            if !evt.extent().is_some() {
                ::core::panicking::panic("assertion failed: evt.extent().is_some()")
            }
            match (&"Rust", &evt.props().pull::<&str, _>("user").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        }
        let rt = simple_runtime(
            |evt| assert_evt(&evt),
            |evt| {
                assert_evt(&evt);
                true
            },
        );
        {
            match () {
                () => {
                    emit::__private::__private_emit_event(
                        &(rt),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__private_evt(
                            ::emit::Path::new_raw("emit_test_ui::emit"),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            &(emit::Empty),
                            &(emit::Empty),
                            emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            ("Rust")
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ]),
                        )),
                        emit::__private::core::option::Option::None::<&emit::Template>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_evt_ref"]
    #[doc(hidden)]
    pub const emit_evt_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_evt_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 352usize,
            start_col: 4usize,
            end_line: 352usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_evt_ref()),
        ),
    };
    fn emit_evt_ref() {
        let rt = simple_runtime(|_| {}, |_| true);
        {
            match () {
                () => {
                    emit::__private::__private_emit_event(
                        &(rt),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(&emit::__private::__private_evt(
                            ::emit::Path::new_raw("emit_test_ui::emit"),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            &(emit::Empty),
                            &(emit::Empty),
                            emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            ("Rust")
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ]),
                        )),
                        emit::__private::core::option::Option::None::<&emit::Template>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_event_filter"]
    #[doc(hidden)]
    pub const emit_event_filter: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_event_filter"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 365usize,
            start_col: 4usize,
            end_line: 365usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_event_filter()),
        ),
    };
    fn emit_event_filter() {
        let called = Called::new();
        let rt = simple_runtime(|_| called.record(), |evt| evt.mdl() == "true");
        {
            match () {
                () => {
                    emit::__private::__private_emit_event(
                        &(rt),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__private_evt(
                            emit::Path::new_raw("false"),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            &(emit::Empty),
                            &(emit::Empty),
                            emit::__private::__PrivateMacroProps::from_array([]),
                        )),
                        emit::__private::core::option::Option::None::<&emit::Template>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        {
            match () {
                () => {
                    emit::__private::__private_emit_event(
                        &(rt),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__private_evt(
                            emit::Path::new_raw("true"),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            &(emit::Empty),
                            &(emit::Empty),
                            emit::__private::__PrivateMacroProps::from_array([]),
                        )),
                        emit::__private::core::option::Option::None::<&emit::Template>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        rt.emitter().blocking_flush(Duration::from_secs(1));
        match (&1, &called.called_times()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_event_when"]
    #[doc(hidden)]
    pub const emit_event_when: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_event_when"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 378usize,
            start_col: 4usize,
            end_line: 378usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_event_when()),
        ),
    };
    fn emit_event_when() {
        let called = Called::new();
        let rt = simple_runtime(|_| called.record(), |evt| evt.mdl() == "true");
        {
            match () {
                () => {
                    emit::__private::__private_emit_event(
                        &(rt),
                        emit::__private::core::option::Option::Some(
                            &(emit::filter::from_fn(|_| true)),
                        ),
                        &(emit::__private::__private_evt(
                            emit::Path::new_raw("false"),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            &(emit::Empty),
                            &(emit::Empty),
                            emit::__private::__PrivateMacroProps::from_array([]),
                        )),
                        emit::__private::core::option::Option::None::<&emit::Template>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                    );
                }
            }
        };
        rt.emitter().blocking_flush(Duration::from_secs(1));
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "emit::emit_props_precedence"]
    #[doc(hidden)]
    pub const emit_props_precedence: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("emit::emit_props_precedence"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/emit.rs",
            start_line: 390usize,
            start_col: 4usize,
            end_line: 390usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emit_props_precedence()),
        ),
    };
    fn emit_props_precedence() {
        let rt = simple_runtime(
            |evt| {
                match (&"evt", &evt.props().pull::<&str, _>("ctxt_props_evt").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"props", &evt.props().pull::<&str, _>("ctxt_props").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"ctxt", &evt.props().pull::<&str, _>("ctxt").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"evt", &evt.props().pull::<&str, _>("props_evt").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"props", &evt.props().pull::<&str, _>("props").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"evt", &evt.props().pull::<&str, _>("evt").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |_| true,
        );
        emit::Frame::push(
                rt.ctxt(),
                emit::__private::__PrivateMacroProps::from_array([
                    {
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("ctxt")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                ("ctxt")
                                    .__private_capture_as_default()
                                    .__private_key_external()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                        )
                    },
                    {
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("ctxt_props")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                ("ctxt")
                                    .__private_capture_as_default()
                                    .__private_key_external()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                        )
                    },
                    {
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("ctxt_props_evt")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                ("ctxt")
                                    .__private_capture_as_default()
                                    .__private_key_external()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                        )
                    },
                ]),
            )
            .call(|| {
                {
                    match (
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("ctxt_props_evt")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            "evt",
                        ),
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("evt")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            "evt",
                        ),
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("props_evt")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            "evt",
                        ),
                    ) {
                        (__tmp0, __tmp1, __tmp2) => {
                            emit::__private::__private_emit(
                                &(rt),
                                &(::emit::Path::new_raw("emit_test_ui::emit")),
                                emit::__private::core::option::Option::None::<&emit::Empty>,
                                &(emit::Empty),
                                &(emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("test")
                                            .with_needs_escaping_raw(false),
                                    ];
                                    __TPL_PARTS
                                })),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    {
                                        (
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("ctxt_props")
                                                    .__private_key_as_default()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateCaptureHook as _,
                                                    __PrivateOptionalCaptureHook as _,
                                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                    __PrivateKeyExternalHook as _,
                                                };
                                                ("props")
                                                    .__private_capture_as_default()
                                                    .__private_key_external()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                        )
                                    },
                                    {
                                        (
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("ctxt_props_evt")
                                                    .__private_key_as_default()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateCaptureHook as _,
                                                    __PrivateOptionalCaptureHook as _,
                                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                    __PrivateKeyExternalHook as _,
                                                };
                                                ("props")
                                                    .__private_capture_as_default()
                                                    .__private_key_external()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                        )
                                    },
                                    {
                                        (
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("props")
                                                    .__private_key_as_default()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateCaptureHook as _,
                                                    __PrivateOptionalCaptureHook as _,
                                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                    __PrivateKeyExternalHook as _,
                                                };
                                                ("props")
                                                    .__private_capture_as_default()
                                                    .__private_key_external()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                        )
                                    },
                                    {
                                        (
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                };
                                                emit::__private::Key("props_evt")
                                                    .__private_key_as_default()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                            #[allow(unused_imports)]
                                            {
                                                use emit::__private::{
                                                    __PrivateCaptureHook as _,
                                                    __PrivateOptionalCaptureHook as _,
                                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                    __PrivateKeyExternalHook as _,
                                                };
                                                ("props")
                                                    .__private_capture_as_default()
                                                    .__private_key_external()
                                                    .__private_uninterpolated()
                                                    .__private_captured()
                                            },
                                        )
                                    },
                                ])),
                                &(emit::__private::__PrivateMacroProps::from_array([
                                    (
                                        __tmp0.0,
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (__tmp0.1)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_uninterpolated()
                                                .__private_captured()
                                        },
                                    ),
                                    (
                                        __tmp1.0,
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (__tmp1.1)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_uninterpolated()
                                                .__private_captured()
                                        },
                                    ),
                                    (
                                        __tmp2.0,
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (__tmp2.1)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_uninterpolated()
                                                .__private_captured()
                                        },
                                    ),
                                ])),
                            );
                        }
                    }
                };
            });
    }
}
mod event {
    use emit::Props;
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "event::event_basic"]
    #[doc(hidden)]
    pub const event_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("event::event_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/event.rs",
            start_line: 7usize,
            start_col: 4usize,
            end_line: 7usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(event_basic()),
        ),
    };
    fn event_basic() {
        let evt = emit::__private::__private_evt(
            ::emit::Path::new_raw("emit_test_ui::event"),
            emit::Template::new_ref({
                const __TPL_PARTS: &[emit::template::Part] = &[
                    emit::template::Part::text("Hello, ").with_needs_escaping_raw(false),
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::template::Part::hole_str(
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                    };
                                    emit::__private::Key("user")
                                        .__private_key_as_default()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            )
                            .__private_fmt_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                ];
                __TPL_PARTS
            }),
            &(emit::Empty),
            &(emit::Empty),
            emit::__private::__PrivateMacroProps::from_array([
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("user")
                                .__private_key_as_default()
                                .__private_interpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            ("Rust")
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_interpolated()
                                .__private_captured()
                        },
                    )
                },
            ]),
        );
        match (&"Hello, Rust", &evt.msg().to_string()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"Hello, {user}", &evt.tpl().to_string()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"emit_test_ui::event", &evt.mdl()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        if !evt.extent().is_none() {
            ::core::panicking::panic("assertion failed: evt.extent().is_none()")
        }
        match (&"Rust", &evt.props().pull::<&str, _>("user").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "event::event_mdl"]
    #[doc(hidden)]
    pub const event_mdl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("event::event_mdl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/event.rs",
            start_line: 23usize,
            start_col: 4usize,
            end_line: 23usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(event_mdl()),
        ),
    };
    fn event_mdl() {
        let evt = emit::__private::__private_evt(
            emit::Path::new_raw("x"),
            emit::Template::new_ref({
                const __TPL_PARTS: &[emit::template::Part] = &[
                    emit::template::Part::text("template").with_needs_escaping_raw(false),
                ];
                __TPL_PARTS
            }),
            &(emit::Empty),
            &(emit::Empty),
            emit::__private::__PrivateMacroProps::from_array([]),
        );
        match (&emit::Path::new_raw("x"), &evt.mdl()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "event::event_extent"]
    #[doc(hidden)]
    pub const event_extent: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("event::event_extent"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/event.rs",
            start_line: 33usize,
            start_col: 4usize,
            end_line: 33usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(event_extent()),
        ),
    };
    fn event_extent() {
        let evt = emit::__private::__private_evt(
            ::emit::Path::new_raw("emit_test_ui::event"),
            emit::Template::new_ref({
                const __TPL_PARTS: &[emit::template::Part] = &[
                    emit::template::Part::text("template").with_needs_escaping_raw(false),
                ];
                __TPL_PARTS
            }),
            &(emit::Timestamp::MIN),
            &(emit::Empty),
            emit::__private::__PrivateMacroProps::from_array([]),
        );
        match (&emit::Timestamp::MIN, &evt.ts().unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "event::event_base_props"]
    #[doc(hidden)]
    pub const event_base_props: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("event::event_base_props"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/event.rs",
            start_line: 43usize,
            start_col: 4usize,
            end_line: 43usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(event_base_props()),
        ),
    };
    fn event_base_props() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("a")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("base")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        let evt = emit::__private::__private_evt(
            ::emit::Path::new_raw("emit_test_ui::event"),
            emit::Template::new_ref({
                const __TPL_PARTS: &[emit::template::Part] = &[
                    emit::template::Part::text("template").with_needs_escaping_raw(false),
                ];
                __TPL_PARTS
            }),
            &(emit::Empty),
            &(props),
            emit::__private::__PrivateMacroProps::from_array([
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("b")
                                .__private_key_as_default()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            ("evt")
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                    )
                },
            ]),
        );
        match (&"base", &evt.props().pull::<&str, _>("a").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"evt", &evt.props().pull::<&str, _>("b").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
mod metric {
    extern crate test;
    #[rustc_test_marker = "metric::metric"]
    #[doc(hidden)]
    pub const metric: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("metric::metric"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/metric.rs",
            start_line: 2usize,
            start_col: 4usize,
            end_line: 2usize,
            end_col: 10usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(metric()),
        ),
    };
    fn metric() {
        let my_metric = 42;
        match emit::__private::__private_metric(
            ::emit::Path::new_raw("emit_test_ui::metric"),
            &(emit::Empty),
            &(emit::Empty),
            "my_metric",
            "last",
            #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            },
        ) {
            evt => {
                match (&42, &evt.value().by_ref().cast::<usize>().unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"my_metric", &evt.name()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"last", &evt.agg()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
        }
        match emit::__private::__private_metric(
            ::emit::Path::new_raw("emit_test_ui::metric"),
            &(emit::Empty),
            &(emit::Empty),
            "my_metric",
            "count",
            #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            },
        ) {
            evt => {
                match (&"count", &evt.agg()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
        }
        match emit::__private::__private_metric(
            ::emit::Path::new_raw("emit_test_ui::metric"),
            &(emit::Empty),
            &(emit::Empty),
            "my_metric",
            "sum",
            #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            },
        ) {
            evt => {
                match (&"sum", &evt.agg()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
        }
        match emit::__private::__private_metric(
            ::emit::Path::new_raw("emit_test_ui::metric"),
            &(emit::Empty),
            &(emit::Empty),
            "my_metric",
            "min",
            #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            },
        ) {
            evt => {
                match (&"min", &evt.agg()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
        }
        match emit::__private::__private_metric(
            ::emit::Path::new_raw("emit_test_ui::metric"),
            &(emit::Empty),
            &(emit::Empty),
            "my_metric",
            "max",
            #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            },
        ) {
            evt => {
                match (&"max", &evt.agg()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
        }
        match emit::__private::__private_metric(
            ::emit::Path::new_raw("emit_test_ui::metric"),
            &(emit::Empty),
            &(emit::Empty),
            "my_metric",
            "last",
            #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            },
        ) {
            evt => {
                match (&"last", &evt.agg()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            }
        }
    }
}
mod props {
    use ::std::fmt;
    use emit::Props;
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "props::props_basic"]
    #[doc(hidden)]
    pub const props_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 9usize,
            start_col: 4usize,
            end_line: 9usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_basic()),
        ),
    };
    fn props_basic() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("a")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (true)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("b")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (1)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("c")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (2.0)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("d")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("text")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        if !props.is_unique() {
            ::core::panicking::panic("assertion failed: props.is_unique()")
        }
        match (&1, &props.pull::<i32, _>("b").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&true, &props.pull::<bool, _>("a").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&2.0, &props.pull::<f64, _>("c").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"text", &props.pull::<&str, _>("d").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_uncooked"]
    #[doc(hidden)]
    pub const props_uncooked: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_uncooked"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 26usize,
            start_col: 4usize,
            end_line: 26usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_uncooked()),
        ),
    };
    fn props_uncooked() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("type")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (1)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&1, &props.pull::<i32, _>("type").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_external"]
    #[doc(hidden)]
    pub const props_external: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_external"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 35usize,
            start_col: 4usize,
            end_line: 35usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_external()),
        ),
    };
    fn props_external() {
        let x = 42;
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("x")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (x)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&42, &props.pull::<i32, _>("x").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_event_meta"]
    #[doc(hidden)]
    pub const props_event_meta: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_event_meta"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 46usize,
            start_col: 4usize,
            end_line: 46usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_event_meta()),
        ),
    };
    fn props_event_meta() {
        let _ = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("mdl")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("module")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("msg")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("message")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("tpl")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("template")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("ts")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("2024-01-01T00:00:01.000Z")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("ts_start")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("2024-01-01T00:00:00.000Z")
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
    }
    extern crate test;
    #[rustc_test_marker = "props::props_cfg"]
    #[doc(hidden)]
    pub const props_cfg: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_cfg"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 57usize,
            start_col: 4usize,
            end_line: 57usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_cfg()),
        ),
    };
    fn props_cfg() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("enabled")
                                .__private_key_as_default()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            ("enabled")
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                    )
                }
            },
        ]);
        match (&"enabled", &props.pull::<&str, _>("enabled").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        if !props.get("disabled").is_none() {
            ::core::panicking::panic(
                "assertion failed: props.get(\"disabled\").is_none()",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_err_string"]
    #[doc(hidden)]
    pub const props_capture_err_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_err_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 90usize,
            start_col: 4usize,
            end_line: 90usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_err_string()),
        ),
    };
    fn props_capture_err_string() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("err")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("Some error")
                            .__private_capture_as_error()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        let err = props.pull::<&str, _>("err").unwrap();
        match (&"Some error", &err) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_err_as_non_err"]
    #[doc(hidden)]
    pub const props_capture_err_as_non_err: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_err_as_non_err"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 121usize,
            start_col: 4usize,
            end_line: 121usize,
            end_col: 32usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_err_as_non_err()),
        ),
    };
    fn props_capture_err_as_non_err() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("err")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (true)
                            .__private_capture_as_display()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        let err = props.pull::<bool, _>("err").unwrap();
        match (&true, &err) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_lvl"]
    #[doc(hidden)]
    pub const props_capture_lvl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_lvl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 132usize,
            start_col: 4usize,
            end_line: 132usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_lvl()),
        ),
    };
    fn props_capture_lvl() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("lvl")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (emit::Level::Info)
                            .__private_capture_as_level()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&emit::Level::Info, &props.pull::<emit::Level, _>("lvl").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_lvl_string"]
    #[doc(hidden)]
    pub const props_capture_lvl_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_lvl_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 144usize,
            start_col: 4usize,
            end_line: 144usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_lvl_string()),
        ),
    };
    fn props_capture_lvl_string() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("lvl")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("info")
                            .__private_capture_as_level()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&emit::Level::Info, &props.pull::<emit::Level, _>("lvl").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_lvl_as_non_lvl"]
    #[doc(hidden)]
    pub const props_capture_lvl_as_non_lvl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_lvl_as_non_lvl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 156usize,
            start_col: 4usize,
            end_line: 156usize,
            end_col: 32usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_lvl_as_non_lvl()),
        ),
    };
    fn props_capture_lvl_as_non_lvl() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("lvl")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (true)
                            .__private_capture_as_display()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&true, &props.pull::<bool, _>("lvl").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_trace_id"]
    #[doc(hidden)]
    pub const props_capture_trace_id: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_trace_id"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 165usize,
            start_col: 4usize,
            end_line: 165usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_trace_id()),
        ),
    };
    fn props_capture_trace_id() {
        let trace_id = emit::TraceId::from_u128(1);
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("trace_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (trace_id)
                            .__private_capture_as_trace_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::TraceId::from_u128(1).unwrap(),
            &props.pull::<emit::TraceId, _>("trace_id").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_trace_id_string"]
    #[doc(hidden)]
    pub const props_capture_trace_id_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_trace_id_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 179usize,
            start_col: 4usize,
            end_line: 179usize,
            end_col: 33usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_trace_id_string()),
        ),
    };
    fn props_capture_trace_id_string() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("trace_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("00000000000000000000000000000001")
                            .__private_capture_as_trace_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::TraceId::from_u128(1).unwrap(),
            &props.pull::<emit::TraceId, _>("trace_id").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_trace_id_u128"]
    #[doc(hidden)]
    pub const props_capture_trace_id_u128: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_trace_id_u128"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 191usize,
            start_col: 4usize,
            end_line: 191usize,
            end_col: 31usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_trace_id_u128()),
        ),
    };
    fn props_capture_trace_id_u128() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("trace_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (0x00000000000000000000000000000001u128)
                            .__private_capture_as_trace_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::TraceId::from_u128(1).unwrap(),
            &props.pull::<emit::TraceId, _>("trace_id").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_trace_id_as_non_trace_id"]
    #[doc(hidden)]
    pub const props_capture_trace_id_as_non_trace_id: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_trace_id_as_non_trace_id"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 203usize,
            start_col: 4usize,
            end_line: 203usize,
            end_col: 42usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_trace_id_as_non_trace_id()),
        ),
    };
    fn props_capture_trace_id_as_non_trace_id() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("trace_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (true)
                            .__private_capture_as_display()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&true, &props.pull::<bool, _>("trace_id").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_id"]
    #[doc(hidden)]
    pub const props_capture_span_id: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_span_id"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 212usize,
            start_col: 4usize,
            end_line: 212usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_id()),
        ),
    };
    fn props_capture_span_id() {
        let span_id = emit::SpanId::from_u64(1);
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (span_id)
                            .__private_capture_as_span_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::SpanId::from_u64(1).unwrap(),
            &props.pull::<emit::SpanId, _>("span_id").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_id_string"]
    #[doc(hidden)]
    pub const props_capture_span_id_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_span_id_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 226usize,
            start_col: 4usize,
            end_line: 226usize,
            end_col: 32usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_id_string()),
        ),
    };
    fn props_capture_span_id_string() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("0000000000000001")
                            .__private_capture_as_span_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::SpanId::from_u64(1).unwrap(),
            &props.pull::<emit::SpanId, _>("span_id").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_id_u64"]
    #[doc(hidden)]
    pub const props_capture_span_id_u64: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_span_id_u64"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 238usize,
            start_col: 4usize,
            end_line: 238usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_id_u64()),
        ),
    };
    fn props_capture_span_id_u64() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (0x0000000000000001u64)
                            .__private_capture_as_span_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::SpanId::from_u64(1).unwrap(),
            &props.pull::<emit::SpanId, _>("span_id").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_id_as_non_span_id"]
    #[doc(hidden)]
    pub const props_capture_span_id_as_non_span_id: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_span_id_as_non_span_id"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 250usize,
            start_col: 4usize,
            end_line: 250usize,
            end_col: 40usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_id_as_non_span_id()),
        ),
    };
    fn props_capture_span_id_as_non_span_id() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (true)
                            .__private_capture_as_display()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&true, &props.pull::<bool, _>("span_id").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_parent"]
    #[doc(hidden)]
    pub const props_capture_span_parent: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_span_parent"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 259usize,
            start_col: 4usize,
            end_line: 259usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_parent()),
        ),
    };
    fn props_capture_span_parent() {
        let span_parent = emit::SpanId::from_u64(1);
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_parent")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (span_parent)
                            .__private_capture_as_span_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::SpanId::from_u64(1).unwrap(),
            &props.pull::<emit::SpanId, _>("span_parent").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_parent_string"]
    #[doc(hidden)]
    pub const props_capture_span_parent_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_capture_span_parent_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 273usize,
            start_col: 4usize,
            end_line: 273usize,
            end_col: 36usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_parent_string()),
        ),
    };
    fn props_capture_span_parent_string() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_parent")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        ("0000000000000001")
                            .__private_capture_as_span_id()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &emit::SpanId::from_u64(1).unwrap(),
            &props.pull::<emit::SpanId, _>("span_parent").unwrap(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_capture_span_parent_as_non_span_id"]
    #[doc(hidden)]
    pub const props_capture_span_parent_as_non_span_id: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName(
                "props::props_capture_span_parent_as_non_span_id",
            ),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 285usize,
            start_col: 4usize,
            end_line: 285usize,
            end_col: 44usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_capture_span_parent_as_non_span_id()),
        ),
    };
    fn props_capture_span_parent_as_non_span_id() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_parent")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (true)
                            .__private_capture_as_display()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&true, &props.pull::<bool, _>("span_parent").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_key"]
    #[doc(hidden)]
    pub const props_key: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_key"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 294usize,
            start_col: 4usize,
            end_line: 294usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_key()),
        ),
    };
    fn props_key() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("a")
                            .__private_key_as(emit::Str::new("not an identifier"))
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (1)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&1, &props.pull::<i32, _>("not an identifier").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_key_expr_str"]
    #[doc(hidden)]
    pub const props_key_expr_str: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_key_expr_str"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 303usize,
            start_col: 4usize,
            end_line: 303usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_key_expr_str()),
        ),
    };
    fn props_key_expr_str() {
        let name = "not an identifier";
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("a")
                            .__private_key_as(
                                emit::__private::__private_capture_key(name),
                            )
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (1)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&1, &props.pull::<i32, _>("not an identifier").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_optional"]
    #[doc(hidden)]
    pub const props_optional: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_optional"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 324usize,
            start_col: 4usize,
            end_line: 324usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_optional()),
        ),
    };
    fn props_optional() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("none")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (::std::option::Option::None::<&i32>)
                            .__private_optional(|v| v.__private_capture_as_default())
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("some")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (::std::option::Option::Some(&1))
                            .__private_optional(|v| v.__private_capture_as_default())
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&1, &props.pull::<i32, _>("some").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        if !props.get("none").is_none() {
            ::core::panicking::panic("assertion failed: props.get(\"none\").is_none()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "props::props_optional_ref"]
    #[doc(hidden)]
    pub const props_optional_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_optional_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 335usize,
            start_col: 4usize,
            end_line: 335usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_optional_ref()),
        ),
    };
    fn props_optional_ref() {
        let s = ::std::string::String::from("short lived");
        let some: Option<&str> = ::std::option::Option::Some(&s);
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("some")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (some)
                            .__private_optional(|v| v.__private_capture_as_default())
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&"short lived", &props.pull::<&str, _>("some").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_optional_multi_attr"]
    #[doc(hidden)]
    pub const props_optional_multi_attr: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_optional_multi_attr"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 348usize,
            start_col: 4usize,
            end_line: 348usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_optional_multi_attr()),
        ),
    };
    fn props_optional_multi_attr() {
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("none")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (::std::option::Option::None::<&i32>)
                            .__private_optional(|v| v.__private_capture_anon_as_debug())
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("some")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (::std::option::Option::Some(&1))
                            .__private_optional(|v| v.__private_capture_anon_as_debug())
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        if !props.get("some").is_some() {
            ::core::panicking::panic("assertion failed: props.get(\"some\").is_some()")
        }
        if !props.get("none").is_none() {
            ::core::panicking::panic("assertion failed: props.get(\"none\").is_none()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "props::props_as_debug"]
    #[doc(hidden)]
    pub const props_as_debug: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_as_debug"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 359usize,
            start_col: 4usize,
            end_line: 359usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_as_debug()),
        ),
    };
    fn props_as_debug() {
        struct Data;
        #[automatically_derived]
        impl ::core::fmt::Debug for Data {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "Data")
            }
        }
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("a")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (Data)
                            .__private_capture_anon_as_debug()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0:?}", Data))
            }),
            &props.get("a").unwrap().to_string(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_as_display"]
    #[doc(hidden)]
    pub const props_as_display: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_as_display"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 371usize,
            start_col: 4usize,
            end_line: 371usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_as_display()),
        ),
    };
    fn props_as_display() {
        struct Data;
        impl fmt::Display for Data {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_fmt(format_args!("Data"))
            }
        }
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("a")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (Data)
                            .__private_capture_anon_as_display()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0}", Data))
            }),
            &props.get("a").unwrap().to_string(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "props::props_as_value"]
    #[doc(hidden)]
    pub const props_as_value: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("props::props_as_value"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/props.rs",
            start_line: 424usize,
            start_col: 4usize,
            end_line: 424usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(props_as_value()),
        ),
    };
    fn props_as_value() {
        struct Data;
        impl emit::value::ToValue for Data {
            fn to_value(&self) -> emit::Value<'_> {
                "Data".to_value()
            }
        }
        let props = emit::__private::__PrivateMacroProps::from_array([
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("data")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (Data)
                            .__private_capture_anon_as_value()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("none")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (::std::option::Option::None::<Data>)
                            .__private_capture_anon_as_value()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
            {
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("some")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (::std::option::Option::Some(Data))
                            .__private_capture_anon_as_value()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            },
        ]);
        match (&"Data", &props.pull::<&str, _>("data").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"Data", &props.pull::<&str, _>("some").unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        if !props.get("none").unwrap().is_null() {
            ::core::panicking::panic(
                "assertion failed: props.get(\"none\").unwrap().is_null()",
            )
        }
    }
}
mod sample {
    use crate::util::{Called, simple_runtime};
    use emit::{Kind, Props, Str};
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "sample::sample_basic"]
    #[doc(hidden)]
    pub const sample_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 8usize,
            start_col: 4usize,
            end_line: 8usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_basic()),
        ),
    };
    fn sample_basic() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                match (
                    &Kind::Metric,
                    &evt.props().pull::<Kind, _>("evt_kind").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&42, &evt.props().pull::<usize, _>("metric_value").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (
                    &"my_metric",
                    &evt.props().pull::<Str, _>("metric_name").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"last", &evt.props().pull::<Str, _>("metric_agg").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        let my_metric = 42;
        #[allow(unused_imports)]
        {
            use emit::__private::{
                __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                __PrivateKeyExternalHook as _,
            };
            
            emit::__private::__private_sample(
                emit::__private::__private_default_sampler(&(rt)),
                ::emit::Path::new_raw("emit_test_ui::sample"),
                &(emit::Empty),
                &(emit::Empty),
                "my_metric",
                "last",
                (my_metric)
                .__private_capture_as_default()
                .__private_key_external()
                .__private_uninterpolated()
                .__private_captured(),
            )
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "sample::sample_value_capture"]
    #[doc(hidden)]
    pub const sample_value_capture: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_value_capture"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 36usize,
            start_col: 4usize,
            end_line: 36usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_value_capture()),
        ),
    };
    fn sample_value_capture() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                match (
                    &"MyValue",
                    &evt.props().get("metric_value").unwrap().to_string(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        struct MyValue;
        #[automatically_derived]
        impl ::core::fmt::Debug for MyValue {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "MyValue")
            }
        }
        let my_metric = MyValue;
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_anon_as_debug()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::Empty),
                        "my_metric",
                        "last",
                        __tmp_value,
                    )
                }
            }
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "sample::sample_agg"]
    #[doc(hidden)]
    pub const sample_agg: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_agg"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 61usize,
            start_col: 4usize,
            end_line: 61usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_agg()),
        ),
    };
    fn sample_agg() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                match (&"count", &evt.props().pull::<Str, _>("metric_agg").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::Empty),
                        "my_metric",
                        "count",
                        __tmp_value,
                    )
                }
            }
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "sample::sample_name"]
    #[doc(hidden)]
    pub const sample_name: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_name"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 79usize,
            start_col: 4usize,
            end_line: 79usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_name()),
        ),
    };
    fn sample_name() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                match (
                    &"my_other_metric",
                    &evt.props().pull::<Str, _>("metric_name").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        let my_metric = 42;
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (my_metric)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::Empty),
                        "my_other_metric",
                        "last",
                        __tmp_value,
                    )
                }
            }
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "sample::sample_props"]
    #[doc(hidden)]
    pub const sample_props: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_props"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 101usize,
            start_col: 4usize,
            end_line: 101usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_props()),
        ),
    };
    fn sample_props() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                match (&true, &evt.props().pull::<bool, _>("a").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&1, &evt.props().pull::<i32, _>("b").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("a")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (true)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("b")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (1)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "last",
                        __tmp_value,
                    )
                }
            }
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "sample::sample_well_known_props_precedence"]
    #[doc(hidden)]
    pub const sample_well_known_props_precedence: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_well_known_props_precedence"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 128usize,
            start_col: 4usize,
            end_line: 128usize,
            end_col: 38usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_well_known_props_precedence()),
        ),
    };
    fn sample_well_known_props_precedence() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                match (
                    &Kind::Metric,
                    &evt.props().pull::<Kind, _>("evt_kind").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&42, &evt.props().pull::<usize, _>("metric_value").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (
                    &"my_metric",
                    &evt.props().pull::<Str, _>("metric_name").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"count", &evt.props().pull::<Str, _>("metric_agg").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("evt_kind")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("custom_kind")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("metric_agg")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("sum")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("metric_name")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("my_other_metric")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("metric_value")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (13)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "count",
                        __tmp_value,
                    )
                }
            }
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "sample::sample_agg_specific"]
    #[doc(hidden)]
    pub const sample_agg_specific: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sample::sample_agg_specific"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/sample.rs",
            start_line: 166usize,
            start_col: 4usize,
            end_line: 166usize,
            end_col: 23usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(sample_agg_specific()),
        ),
    };
    fn sample_agg_specific() {
        let called = Called::new();
        let rt = simple_runtime(
            |evt| {
                let agg = evt.props().pull::<Str, _>("metric_agg").unwrap();
                let expected = evt.props().pull::<Str, _>("expected_agg").unwrap();
                match (&agg, &expected) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                called.record();
            },
            |_| true,
        );
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("expected_agg")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("count")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "count",
                        __tmp_value,
                    )
                }
            }
        };
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("expected_agg")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("sum")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "sum",
                        __tmp_value,
                    )
                }
            }
        };
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("expected_agg")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("min")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "min",
                        __tmp_value,
                    )
                }
            }
        };
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("expected_agg")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("max")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "max",
                        __tmp_value,
                    )
                }
            }
        };
        {
            match #[allow(unused_imports)]
            {
                use emit::__private::{
                    __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                    __PrivateKeyExternalHook as _,
                };
                (42)
                    .__private_capture_as_default()
                    .__private_key_external()
                    .__private_uninterpolated()
                    .__private_captured()
            } {
                __tmp_value => {
                    emit::__private::__private_sample(
                        emit::__private::__private_default_sampler(&(rt)),
                        ::emit::Path::new_raw("emit_test_ui::sample"),
                        &(emit::Empty),
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("expected_agg")
                                            .__private_key_as_default()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        ("last")
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_uninterpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        "my_metric",
                        "last",
                        __tmp_value,
                    )
                }
            }
        };
        if !called.was_called() {
            ::core::panicking::panic("assertion failed: called.was_called()")
        }
    }
}
mod span {
    use ::std::time::Duration;
    use emit::{Ctxt, Emitter, Kind, Props, Str};
    use crate::util::{StaticCalled, StaticRuntime, static_runtime};
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "span::span_basic"]
    #[doc(hidden)]
    pub const span_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 17usize,
            start_col: 4usize,
            end_line: 17usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_basic()),
        ),
    };
    fn span_basic() {
        fn assert_event_base(evt: &emit::Event<impl Props>) {
            match (&"emit_test_ui::span", &evt.mdl()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            if !evt.props().get("user").is_some() {
                ::core::panicking::panic(
                    "assertion failed: evt.props().get(\"user\").is_some()",
                )
            }
            match (&"greet {user}", &evt.props().pull::<&str, _>("span_name").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&Kind::Span, &evt.props().pull::<Kind, _>("evt_kind").unwrap()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            if !evt.props().pull::<emit::TraceId, _>("trace_id").is_some() {
                ::core::panicking::panic(
                    "assertion failed: evt.props().pull::<emit::TraceId, _>(\"trace_id\").is_some()",
                )
            }
            if !evt.props().pull::<emit::SpanId, _>("span_id").is_some() {
                ::core::panicking::panic(
                    "assertion failed: evt.props().pull::<emit::SpanId, _>(\"span_id\").is_some()",
                )
            }
        }
        fn assert_event(evt: &emit::Event<impl Props>) {
            assert_event_base(&evt);
            if !evt.extent().unwrap().is_range() {
                ::core::panicking::panic(
                    "assertion failed: evt.extent().unwrap().is_range()",
                )
            }
            match (&"greet Rust", &evt.msg().to_string()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
            match (&"greet {user}", &evt.tpl().to_string()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        }
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |evt| {
                assert_event(&evt);
                CALLED.record();
            },
            |evt| {
                assert_event_base(&evt);
                true
            },
        );
        static DEBUG_CALLED: StaticCalled = StaticCalled::new();
        static DEBUG_RT: StaticRuntime = static_runtime(
            |evt| {
                assert_event(&evt);
                match (
                    &emit::Level::Debug,
                    &evt.props().pull::<emit::Level, _>("lvl").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                DEBUG_CALLED.record();
            },
            |evt| {
                assert_event_base(&evt);
                true
            },
        );
        static INFO_CALLED: StaticCalled = StaticCalled::new();
        static INFO_RT: StaticRuntime = static_runtime(
            |evt| {
                assert_event(&evt);
                match (
                    &emit::Level::Info,
                    &evt.props().pull::<emit::Level, _>("lvl").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                INFO_CALLED.record();
            },
            |evt| {
                assert_event_base(&evt);
                true
            },
        );
        static WARN_CALLED: StaticCalled = StaticCalled::new();
        static WARN_RT: StaticRuntime = static_runtime(
            |evt| {
                assert_event(&evt);
                match (
                    &emit::Level::Warn,
                    &evt.props().pull::<emit::Level, _>("lvl").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                WARN_CALLED.record();
            },
            |evt| {
                assert_event_base(&evt);
                true
            },
        );
        static ERROR_CALLED: StaticCalled = StaticCalled::new();
        static ERROR_RT: StaticRuntime = static_runtime(
            |evt| {
                assert_event(&evt);
                match (
                    &emit::Level::Error,
                    &evt.props().pull::<emit::Level, _>("lvl").unwrap(),
                ) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                ERROR_CALLED.record();
            },
            |evt| {
                assert_event_base(&evt);
                true
            },
        );
        fn exec(user: &str) {
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        RT.ctxt()
                            .with_current(|props| {
                                match (&user, &props.pull::<&str, _>("user").unwrap()) {
                                    (left_val, right_val) => {
                                        if !(*left_val == *right_val) {
                                            let kind = ::core::panicking::AssertKind::Eq;
                                            ::core::panicking::assert_failed(
                                                kind,
                                                &*left_val,
                                                &*right_val,
                                                ::core::option::Option::None,
                                            );
                                        }
                                    }
                                };
                            });
                        let _ = user;
                    }
                })
        }
        fn exec_debug(user: &str) {
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(DEBUG_RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Debug),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(DEBUG_RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Debug),
                            ),
                            &(emit::Level::Debug),
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        DEBUG_RT
                            .ctxt()
                            .with_current(|props| {
                                match (&user, &props.pull::<&str, _>("user").unwrap()) {
                                    (left_val, right_val) => {
                                        if !(*left_val == *right_val) {
                                            let kind = ::core::panicking::AssertKind::Eq;
                                            ::core::panicking::assert_failed(
                                                kind,
                                                &*left_val,
                                                &*right_val,
                                                ::core::option::Option::None,
                                            );
                                        }
                                    }
                                };
                            });
                        let _ = user;
                    }
                })
        }
        fn exec_info(user: &str) {
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(INFO_RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Info),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(INFO_RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Info),
                            ),
                            &(emit::Level::Info),
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        INFO_RT
                            .ctxt()
                            .with_current(|props| {
                                match (&user, &props.pull::<&str, _>("user").unwrap()) {
                                    (left_val, right_val) => {
                                        if !(*left_val == *right_val) {
                                            let kind = ::core::panicking::AssertKind::Eq;
                                            ::core::panicking::assert_failed(
                                                kind,
                                                &*left_val,
                                                &*right_val,
                                                ::core::option::Option::None,
                                            );
                                        }
                                    }
                                };
                            });
                        let _ = user;
                    }
                })
        }
        fn exec_info_temporary() {
            let (mut __span_guard, __ctxt) = match ((
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::__private::Key("user")
                        .__private_key_as_default()
                        .__private_interpolated()
                        .__private_captured()
                },
                ::std::string::String::from("Rust"),
            )) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(INFO_RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Info),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_interpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(INFO_RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Info),
                            ),
                            &(emit::Level::Info),
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        INFO_RT
                            .ctxt()
                            .with_current(|props| {
                                match (&"Rust", &props.pull::<&str, _>("user").unwrap()) {
                                    (left_val, right_val) => {
                                        if !(*left_val == *right_val) {
                                            let kind = ::core::panicking::AssertKind::Eq;
                                            ::core::panicking::assert_failed(
                                                kind,
                                                &*left_val,
                                                &*right_val,
                                                ::core::option::Option::None,
                                            );
                                        }
                                    }
                                };
                            });
                    }
                })
        }
        fn exec_warn(user: &str) {
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(WARN_RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Warn),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(WARN_RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Warn),
                            ),
                            &(emit::Level::Warn),
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        WARN_RT
                            .ctxt()
                            .with_current(|props| {
                                match (&user, &props.pull::<&str, _>("user").unwrap()) {
                                    (left_val, right_val) => {
                                        if !(*left_val == *right_val) {
                                            let kind = ::core::panicking::AssertKind::Eq;
                                            ::core::panicking::assert_failed(
                                                kind,
                                                &*left_val,
                                                &*right_val,
                                                ::core::option::Option::None,
                                            );
                                        }
                                    }
                                };
                            });
                        let _ = user;
                    }
                })
        }
        fn exec_error(user: &str) {
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(ERROR_RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Error),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(ERROR_RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Error),
                            ),
                            &(emit::Level::Error),
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        ERROR_RT
                            .ctxt()
                            .with_current(|props| {
                                match (&user, &props.pull::<&str, _>("user").unwrap()) {
                                    (left_val, right_val) => {
                                        if !(*left_val == *right_val) {
                                            let kind = ::core::panicking::AssertKind::Eq;
                                            ::core::panicking::assert_failed(
                                                kind,
                                                &*left_val,
                                                &*right_val,
                                                ::core::option::Option::None,
                                            );
                                        }
                                    }
                                };
                            });
                        let _ = user;
                    }
                })
        }
        exec("Rust");
        exec_debug("Rust");
        exec_info_temporary();
        exec_info("Rust");
        exec_warn("Rust");
        exec_error("Rust");
        RT.emitter().blocking_flush(Duration::from_secs(1));
        DEBUG_RT.emitter().blocking_flush(Duration::from_secs(1));
        INFO_RT.emitter().blocking_flush(Duration::from_secs(1));
        WARN_RT.emitter().blocking_flush(Duration::from_secs(1));
        ERROR_RT.emitter().blocking_flush(Duration::from_secs(1));
        if !CALLED.was_called() {
            ::core::panicking::panic("assertion failed: CALLED.was_called()")
        }
        if !DEBUG_CALLED.was_called() {
            ::core::panicking::panic("assertion failed: DEBUG_CALLED.was_called()")
        }
        if !INFO_CALLED.was_called() {
            ::core::panicking::panic("assertion failed: INFO_CALLED.was_called()")
        }
        if !WARN_CALLED.was_called() {
            ::core::panicking::panic("assertion failed: WARN_CALLED.was_called()")
        }
        if !ERROR_CALLED.was_called() {
            ::core::panicking::panic("assertion failed: ERROR_CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_basic_async"]
    #[doc(hidden)]
    pub const span_basic_async: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_basic_async"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 206usize,
            start_col: 10usize,
            end_line: 206usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_basic_async()),
        ),
    };
    fn span_basic_async() {
        let body = async {
            static CALLED: StaticCalled = StaticCalled::new();
            static RT: StaticRuntime = static_runtime(
                |evt| {
                    match (&"greet Rust", &evt.msg().to_string()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&"greet {user}", &evt.tpl().to_string()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&"emit_test_ui::span", &evt.mdl()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    if !evt.extent().unwrap().is_range() {
                        ::core::panicking::panic(
                            "assertion failed: evt.extent().unwrap().is_range()",
                        )
                    }
                    if !evt.props().pull::<emit::TraceId, _>("trace_id").is_some() {
                        ::core::panicking::panic(
                            "assertion failed: evt.props().pull::<emit::TraceId, _>(\"trace_id\").is_some()",
                        )
                    }
                    if !evt.props().pull::<emit::SpanId, _>("span_id").is_some() {
                        ::core::panicking::panic(
                            "assertion failed: evt.props().pull::<emit::SpanId, _>(\"span_id\").is_some()",
                        )
                    }
                    CALLED.record();
                },
                |_| true,
            );
            async fn exec(user: &str) {
                let (mut __span_guard, __ctxt) = match ({
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("user")
                                .__private_key_as_default()
                                .__private_interpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (user)
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_interpolated()
                                .__private_captured()
                        },
                    )
                }) {
                    (__tmp0) => {
                        emit::__private::__private_begin_span(
                            &(RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "greet {user}",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                (__tmp0.0, __tmp0.1),
                            ])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                emit::Empty,
                            ),
                            emit::__private::__private_complete_span(
                                &(RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("greet ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {
                            let _ = user;
                        }
                    })
                    .await
            }
            async fn exec_temporary() {
                let (mut __span_guard, __ctxt) = match ((
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    ::std::string::String::from("Rust"),
                )) {
                    (__tmp0) => {
                        emit::__private::__private_begin_span(
                            &(RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "greet {user}",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                (
                                    __tmp0.0,
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (__tmp0.1)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ),
                            ])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                emit::Empty,
                            ),
                            emit::__private::__private_complete_span(
                                &(RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("greet ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {}
                    })
                    .await
            }
            exec("Rust").await;
            exec_temporary().await;
            RT.emitter().blocking_flush(Duration::from_secs(1));
            if !CALLED.was_called() {
                ::core::panicking::panic("assertion failed: CALLED.was_called()")
            }
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_rt_ref"]
    #[doc(hidden)]
    pub const span_rt_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_rt_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 240usize,
            start_col: 4usize,
            end_line: 240usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_rt_ref()),
        ),
    };
    fn span_rt_ref() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |_| {
                CALLED.record();
            },
            |_| true,
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(&RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(&RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        exec();
        RT.emitter().blocking_flush(Duration::from_secs(1));
        match (&1, &CALLED.called_times()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "span::span_by_value_arg"]
    #[doc(hidden)]
    pub const span_by_value_arg: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_by_value_arg"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 260usize,
            start_col: 4usize,
            end_line: 260usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_by_value_arg()),
        ),
    };
    fn span_by_value_arg() {
        static RT: StaticRuntime = static_runtime(|_| {}, |_| true);
        fn take_string(_: ::std::string::String) {}
        fn exec(arg: ::std::string::String) {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(&RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(&RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        take_string(arg);
                    }
                })
        }
        exec("Owned".to_owned());
        RT.emitter().blocking_flush(Duration::from_secs(1));
    }
    extern crate test;
    #[rustc_test_marker = "span::async_span_by_value_arg"]
    #[doc(hidden)]
    pub const async_span_by_value_arg: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::async_span_by_value_arg"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 276usize,
            start_col: 10usize,
            end_line: 276usize,
            end_col: 33usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(async_span_by_value_arg()),
        ),
    };
    fn async_span_by_value_arg() {
        let body = async {
            static RT: StaticRuntime = static_runtime(|_| {}, |_| true);
            fn take_string(_: ::std::string::String) {}
            async fn exec(arg: ::std::string::String) {
                let (mut __span_guard, __ctxt) = match () {
                    () => {
                        emit::__private::__private_begin_span(
                            &(&RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "test",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                emit::Empty,
                            ),
                            emit::__private::__private_complete_span(
                                &(&RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("test")
                                            .with_needs_escaping_raw(false),
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {
                            take_string(arg);
                            tokio::time::sleep(Default::default()).await;
                        }
                    })
                    .await
            }
            exec("Owned".to_owned()).await;
            RT.emitter().blocking_flush(Duration::from_secs(1));
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_guard"]
    #[doc(hidden)]
    pub const span_guard: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_guard"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 294usize,
            start_col: 4usize,
            end_line: 294usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_guard()),
        ),
    };
    fn span_guard() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |_| {
                CALLED.record();
            },
            |_| true,
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    let mut span = __span_guard;
                    {
                        let span: emit::span::SpanGuard<_, _, _> = span;
                        span.complete();
                    }
                })
        }
        exec();
        if !CALLED.was_called() {
            ::core::panicking::panic("assertion failed: CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_guard_props"]
    #[doc(hidden)]
    pub const span_guard_props: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_guard_props"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 315usize,
            start_col: 4usize,
            end_line: 315usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_guard_props()),
        ),
    };
    fn span_guard_props() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&true, &evt.props().pull::<bool, _>("extra").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                CALLED.record();
            },
            |_| true,
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    let mut span = __span_guard;
                    {
                        let span = span.push_prop("extra", true);
                        span.complete();
                    }
                })
        }
        exec();
        if !CALLED.was_called() {
            ::core::panicking::panic("assertion failed: CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_name_escape"]
    #[doc(hidden)]
    pub const span_name_escape: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_name_escape"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 339usize,
            start_col: 4usize,
            end_line: 339usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_name_escape()),
        ),
    };
    fn span_name_escape() {
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&"{{test}}", &evt.props().pull::<&str, _>("span_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |evt| {
                match (&"{{test}}", &evt.props().pull::<&str, _>("span_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "{{test}}",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("{test}")
                                        .with_needs_escaping_raw(true),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        exec();
        RT.emitter().blocking_flush(Duration::from_secs(1));
    }
    extern crate test;
    #[rustc_test_marker = "span::span_mdl"]
    #[doc(hidden)]
    pub const span_mdl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_mdl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 366usize,
            start_col: 4usize,
            end_line: 366usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_mdl()),
        ),
    };
    fn span_mdl() {
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&"custom_module", &evt.mdl()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |evt| {
                match (&"custom_module", &evt.mdl()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        emit::Path::new_raw("custom_module"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        exec();
        RT.emitter().blocking_flush(Duration::from_secs(1));
    }
    extern crate test;
    #[rustc_test_marker = "span::span_fn_name"]
    #[doc(hidden)]
    pub const span_fn_name: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_fn_name"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 387usize,
            start_col: 4usize,
            end_line: 387usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_fn_name()),
        ),
    };
    fn span_fn_name() {
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&"exec", &evt.props().pull::<&str, _>("fn_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"exec", &evt.props().pull::<&str, _>("other_fn_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |evt| {
                match (&"exec", &evt.props().pull::<&str, _>("fn_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"exec", &evt.props().pull::<&str, _>("other_fn_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        fn exec() {
            let fn_name = "exec";
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("other_fn_name")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (fn_name)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            ("fn_name", fn_name),
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        let _: &'static str = fn_name;
                        RT.ctxt()
                            .with_current(|props| {
                                if !props.get("fn_name").is_none() {
                                    ::core::panicking::panic(
                                        "assertion failed: props.get(\"fn_name\").is_none()",
                                    )
                                }
                            })
                    }
                })
        }
        exec();
        RT.emitter().blocking_flush(Duration::from_secs(1));
    }
    extern crate test;
    #[rustc_test_marker = "span::async_span_fn_name"]
    #[doc(hidden)]
    pub const async_span_fn_name: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::async_span_fn_name"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 422usize,
            start_col: 10usize,
            end_line: 422usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(async_span_fn_name()),
        ),
    };
    fn async_span_fn_name() {
        let body = async {
            static RT: StaticRuntime = static_runtime(
                |evt| {
                    match (&"exec", &evt.props().pull::<&str, _>("fn_name").unwrap()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (
                        &"exec",
                        &evt.props().pull::<&str, _>("other_fn_name").unwrap(),
                    ) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                },
                |evt| {
                    match (&"exec", &evt.props().pull::<&str, _>("fn_name").unwrap()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (
                        &"exec",
                        &evt.props().pull::<&str, _>("other_fn_name").unwrap(),
                    ) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    true
                },
            );
            async fn exec() {
                let fn_name = "exec";
                let (mut __span_guard, __ctxt) = match ({
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("other_fn_name")
                                .__private_key_as_default()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (fn_name)
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                    )
                }) {
                    (__tmp0) => {
                        emit::__private::__private_begin_span(
                            &(RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "test",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                (__tmp0.0, __tmp0.1),
                            ])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                ("fn_name", fn_name),
                            ),
                            emit::__private::__private_complete_span(
                                &(RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("test")
                                            .with_needs_escaping_raw(false),
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {
                            let _: &'static str = fn_name;
                            tokio::time::sleep(Default::default()).await;
                            RT.ctxt()
                                .with_current(|props| {
                                    if !props.get("fn_name").is_none() {
                                        ::core::panicking::panic(
                                            "assertion failed: props.get(\"fn_name\").is_none()",
                                        )
                                    }
                                })
                        }
                    })
                    .await
            }
            exec().await;
            RT.emitter().blocking_flush(Duration::from_secs(1));
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_evt_props_basic"]
    #[doc(hidden)]
    pub const span_evt_props_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_evt_props_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 459usize,
            start_col: 4usize,
            end_line: 459usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_evt_props_basic()),
        ),
    };
    fn span_evt_props_basic() {
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&42, &evt.props().pull::<i32, _>("a").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&true, &evt.props().pull::<bool, _>("b").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |evt| {
                match (&42, &evt.props().pull::<i32, _>("a").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                if !evt.props().get("b").is_none() {
                    ::core::panicking::panic(
                        "assertion failed: evt.props().get(\"b\").is_none()",
                    )
                }
                true
            },
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            [("a", 42)],
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    let mut span = __span_guard;
                    {
                        let _span = span.push_prop("b", true);
                    }
                })
        }
        exec();
        RT.emitter().blocking_flush(Duration::from_secs(1));
    }
    extern crate test;
    #[rustc_test_marker = "span::span_filter"]
    #[doc(hidden)]
    pub const span_filter: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_filter"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 632usize,
            start_col: 4usize,
            end_line: 632usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_filter()),
        ),
    };
    fn span_filter() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |_| CALLED.record(),
            |evt| evt.mdl() == "true",
        );
        fn exec_true() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        emit::Path::new_raw("true"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        fn exec_false() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        emit::Path::new_raw("false"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        exec_true();
        exec_false();
        RT.emitter().blocking_flush(Duration::from_secs(1));
        match (&1, &CALLED.called_times()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "span::span_when"]
    #[doc(hidden)]
    pub const span_when: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_when"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 651usize,
            start_col: 4usize,
            end_line: 651usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_when()),
        ),
    };
    fn span_when() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |_| CALLED.record(),
            |evt| evt.mdl() == "tralse",
        );
        fn exec_true() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        emit::Path::new_raw("true"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::Some(
                            &(emit::filter::from_fn(|evt| evt.mdl() == "false")),
                        ),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        fn exec_false() {
            let (mut __span_guard, __ctxt) = match () {
                () => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        emit::Path::new_raw("false"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::Some(
                            &(emit::filter::from_fn(|evt| evt.mdl() == "false")),
                        ),
                        &(emit::__private::__PrivateMacroProps::from_array([])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        exec_true();
        exec_false();
        RT.emitter().blocking_flush(Duration::from_secs(1));
        match (&1, &CALLED.called_times()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "span::span_explicit_ids"]
    #[doc(hidden)]
    pub const span_explicit_ids: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_explicit_ids"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 670usize,
            start_col: 4usize,
            end_line: 670usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_explicit_ids()),
        ),
    };
    fn span_explicit_ids() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&emit::TraceId::from_u128(1), &evt.props().pull("trace_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(2), &evt.props().pull("span_parent")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(3), &evt.props().pull("span_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                CALLED.record();
            },
            |evt| {
                match (&emit::TraceId::from_u128(1), &evt.props().pull("trace_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(2), &evt.props().pull("span_parent")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(3), &evt.props().pull("span_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        fn exec(trace_id: &str, span_parent: &str, span_id: &str) {
            let (mut __span_guard, __ctxt) = match (
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("span_id")
                                .__private_key_as_default()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (span_id)
                                .__private_capture_as_span_id()
                                .__private_key_external()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                    )
                },
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("span_parent")
                                .__private_key_as_default()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (span_parent)
                                .__private_capture_as_span_id()
                                .__private_key_external()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                    )
                },
                {
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("trace_id")
                                .__private_key_as_default()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (trace_id)
                                .__private_capture_as_trace_id()
                                .__private_key_external()
                                .__private_uninterpolated()
                                .__private_captured()
                        },
                    )
                },
            ) {
                (__tmp0, __tmp1, __tmp2) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                            (__tmp1.0, __tmp1.1),
                            (__tmp2.0, __tmp2.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        let ctxt = emit::SpanCtxt::current(RT.ctxt());
                        match (&emit::TraceId::from_u128(1), &ctxt.trace_id().copied()) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        match (
                            &emit::SpanId::from_u64(2),
                            &ctxt.span_parent().copied(),
                        ) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        match (&emit::SpanId::from_u64(3), &ctxt.span_id().copied()) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                    }
                })
        }
        exec("00000000000000000000000000000001", "0000000000000002", "0000000000000003");
        if !CALLED.was_called() {
            ::core::panicking::panic("assertion failed: CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::async_span_explicit_ids"]
    #[doc(hidden)]
    pub const async_span_explicit_ids: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::async_span_explicit_ids"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 708usize,
            start_col: 10usize,
            end_line: 708usize,
            end_col: 33usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(async_span_explicit_ids()),
        ),
    };
    fn async_span_explicit_ids() {
        let body = async {
            static CALLED: StaticCalled = StaticCalled::new();
            static RT: StaticRuntime = static_runtime(
                |evt| {
                    match (&emit::TraceId::from_u128(1), &evt.props().pull("trace_id")) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (
                        &emit::SpanId::from_u64(2),
                        &evt.props().pull("span_parent"),
                    ) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&emit::SpanId::from_u64(3), &evt.props().pull("span_id")) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    CALLED.record();
                },
                |evt| {
                    match (&emit::TraceId::from_u128(1), &evt.props().pull("trace_id")) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (
                        &emit::SpanId::from_u64(2),
                        &evt.props().pull("span_parent"),
                    ) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&emit::SpanId::from_u64(3), &evt.props().pull("span_id")) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    true
                },
            );
            async fn exec(trace_id: &str, span_parent: &str, span_id: &str) {
                let (mut __span_guard, __ctxt) = match (
                    {
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("span_id")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                (span_id)
                                    .__private_capture_as_span_id()
                                    .__private_key_external()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                        )
                    },
                    {
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("span_parent")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                (span_parent)
                                    .__private_capture_as_span_id()
                                    .__private_key_external()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                        )
                    },
                    {
                        (
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("trace_id")
                                    .__private_key_as_default()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateCaptureHook as _,
                                    __PrivateOptionalCaptureHook as _,
                                    __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                    __PrivateKeyExternalHook as _,
                                };
                                (trace_id)
                                    .__private_capture_as_trace_id()
                                    .__private_key_external()
                                    .__private_uninterpolated()
                                    .__private_captured()
                            },
                        )
                    },
                ) {
                    (__tmp0, __tmp1, __tmp2) => {
                        emit::__private::__private_begin_span(
                            &(RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "test",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                (__tmp0.0, __tmp0.1),
                                (__tmp1.0, __tmp1.1),
                                (__tmp2.0, __tmp2.1),
                            ])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                emit::Empty,
                            ),
                            emit::__private::__private_complete_span(
                                &(RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("test")
                                            .with_needs_escaping_raw(false),
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {
                            let ctxt = emit::SpanCtxt::current(RT.ctxt());
                            tokio::time::sleep(Default::default()).await;
                            match (
                                &emit::TraceId::from_u128(1),
                                &ctxt.trace_id().copied(),
                            ) {
                                (left_val, right_val) => {
                                    if !(*left_val == *right_val) {
                                        let kind = ::core::panicking::AssertKind::Eq;
                                        ::core::panicking::assert_failed(
                                            kind,
                                            &*left_val,
                                            &*right_val,
                                            ::core::option::Option::None,
                                        );
                                    }
                                }
                            };
                            match (
                                &emit::SpanId::from_u64(2),
                                &ctxt.span_parent().copied(),
                            ) {
                                (left_val, right_val) => {
                                    if !(*left_val == *right_val) {
                                        let kind = ::core::panicking::AssertKind::Eq;
                                        ::core::panicking::assert_failed(
                                            kind,
                                            &*left_val,
                                            &*right_val,
                                            ::core::option::Option::None,
                                        );
                                    }
                                }
                            };
                            match (
                                &emit::SpanId::from_u64(3),
                                &ctxt.span_id().copied(),
                            ) {
                                (left_val, right_val) => {
                                    if !(*left_val == *right_val) {
                                        let kind = ::core::panicking::AssertKind::Eq;
                                        ::core::panicking::assert_failed(
                                            kind,
                                            &*left_val,
                                            &*right_val,
                                            ::core::option::Option::None,
                                        );
                                    }
                                }
                            };
                        }
                    })
                    .await
            }
            exec(
                    "00000000000000000000000000000001",
                    "0000000000000002",
                    "0000000000000003",
                )
                .await;
            if !CALLED.was_called() {
                ::core::panicking::panic("assertion failed: CALLED.was_called()")
            }
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_explicit_ids_ctxt"]
    #[doc(hidden)]
    pub const span_explicit_ids_ctxt: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_explicit_ids_ctxt"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 749usize,
            start_col: 4usize,
            end_line: 749usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_explicit_ids_ctxt()),
        ),
    };
    fn span_explicit_ids_ctxt() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&emit::TraceId::from_u128(1), &evt.props().pull("trace_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(2), &evt.props().pull("span_parent")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(3), &evt.props().pull("span_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                CALLED.record();
            },
            |evt| {
                match (&emit::TraceId::from_u128(1), &evt.props().pull("trace_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(2), &evt.props().pull("span_parent")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&emit::SpanId::from_u64(3), &evt.props().pull("span_id")) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        fn exec(ctxt: emit::SpanCtxt) {
            let (mut __span_guard, __ctxt) = match (
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    ctxt.span_id(),
                ),
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_parent")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    ctxt.span_parent(),
                ),
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("trace_id")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    ctxt.trace_id(),
                ),
            ) {
                (__tmp0, __tmp1, __tmp2) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_span_id()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                            (
                                __tmp1.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp1.1)
                                        .__private_capture_as_span_id()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                            (
                                __tmp2.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp2.1)
                                        .__private_capture_as_trace_id()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        let current = emit::SpanCtxt::current(RT.ctxt());
                        match (&ctxt, &current) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                    }
                })
        }
        exec(
            emit::SpanCtxt::new(
                emit::TraceId::from_u128(1),
                emit::SpanId::from_u64(2),
                emit::SpanId::from_u64(3),
            ),
        );
        if !CALLED.was_called() {
            ::core::panicking::panic("assertion failed: CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_setup"]
    #[doc(hidden)]
    pub const span_setup: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_setup"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 1598usize,
            start_col: 4usize,
            end_line: 1598usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_setup()),
        ),
    };
    fn span_setup() {
        static SETUP_CALLED: StaticCalled = StaticCalled::new();
        static DROP_CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(|_| {}, |_| true);
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                DROP_CALLED.record();
            }
        }
        fn setup() -> Guard {
            SETUP_CALLED.record();
            Guard
        }
        fn exec(user: &str) {
            let __setup = (setup)();
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        if !SETUP_CALLED.was_called() {
                            ::core::panicking::panic(
                                "assertion failed: SETUP_CALLED.was_called()",
                            )
                        }
                        if !!DROP_CALLED.was_called() {
                            ::core::panicking::panic(
                                "assertion failed: !DROP_CALLED.was_called()",
                            )
                        }
                    }
                })
        }
        exec("Rust");
        if !SETUP_CALLED.was_called() {
            ::core::panicking::panic("assertion failed: SETUP_CALLED.was_called()")
        }
        if !DROP_CALLED.was_called() {
            ::core::panicking::panic("assertion failed: DROP_CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_setup_async"]
    #[doc(hidden)]
    pub const span_setup_async: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_setup_async"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 1630usize,
            start_col: 10usize,
            end_line: 1630usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_setup_async()),
        ),
    };
    fn span_setup_async() {
        let body = async {
            static SETUP_CALLED: StaticCalled = StaticCalled::new();
            static DROP_CALLED: StaticCalled = StaticCalled::new();
            static RT: StaticRuntime = static_runtime(|_| {}, |_| true);
            struct Guard;
            impl Drop for Guard {
                fn drop(&mut self) {
                    DROP_CALLED.record();
                }
            }
            fn setup() -> Guard {
                SETUP_CALLED.record();
                Guard
            }
            async fn exec(user: &str) {
                let __setup = (setup)();
                let (mut __span_guard, __ctxt) = match ({
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("user")
                                .__private_key_as_default()
                                .__private_interpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (user)
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_interpolated()
                                .__private_captured()
                        },
                    )
                }) {
                    (__tmp0) => {
                        emit::__private::__private_begin_span(
                            &(RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "greet {user}",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                (__tmp0.0, __tmp0.1),
                            ])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                emit::Empty,
                            ),
                            emit::__private::__private_complete_span(
                                &(RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("greet ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            if !SETUP_CALLED.was_called() {
                                ::core::panicking::panic(
                                    "assertion failed: SETUP_CALLED.was_called()",
                                )
                            }
                            if !!DROP_CALLED.was_called() {
                                ::core::panicking::panic(
                                    "assertion failed: !DROP_CALLED.was_called()",
                                )
                            }
                        }
                    })
                    .await
            }
            exec("Rust").await;
            if !SETUP_CALLED.was_called() {
                ::core::panicking::panic("assertion failed: SETUP_CALLED.was_called()")
            }
            if !DROP_CALLED.was_called() {
                ::core::panicking::panic("assertion failed: DROP_CALLED.was_called()")
            }
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_well_known_props_precedence"]
    #[doc(hidden)]
    pub const span_well_known_props_precedence: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_well_known_props_precedence"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 1664usize,
            start_col: 4usize,
            end_line: 1664usize,
            end_col: 36usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_well_known_props_precedence()),
        ),
    };
    fn span_well_known_props_precedence() {
        static RT: StaticRuntime = static_runtime(
            |evt| {
                match (&Kind::Span, &evt.props().pull::<Kind, _>("evt_kind").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"test", &evt.props().pull::<Str, _>("span_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
            },
            |evt| {
                match (&Kind::Span, &evt.props().pull::<Kind, _>("evt_kind").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                match (&"test", &evt.props().pull::<Str, _>("span_name").unwrap()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::None,
                            );
                        }
                    }
                };
                true
            },
        );
        fn exec() {
            let (mut __span_guard, __ctxt) = match (
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("evt_kind")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    "custom",
                ),
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("span_name")
                            .__private_key_as_default()
                            .__private_uninterpolated()
                            .__private_captured()
                    },
                    "custom_name",
                ),
            ) {
                (__tmp0, __tmp1) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "test",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (
                                __tmp0.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp0.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                            (
                                __tmp1.0,
                                #[allow(unused_imports)]
                                {
                                    use emit::__private::{
                                        __PrivateCaptureHook as _,
                                        __PrivateOptionalCaptureHook as _,
                                        __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                        __PrivateKeyExternalHook as _,
                                    };
                                    (__tmp1.1)
                                        .__private_capture_as_default()
                                        .__private_key_external()
                                        .__private_uninterpolated()
                                        .__private_captured()
                                },
                            ),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("test")
                                        .with_needs_escaping_raw(false),
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {}
                })
        }
        exec();
        RT.emitter().blocking_flush(Duration::from_secs(1));
    }
    extern crate test;
    #[rustc_test_marker = "span::span_impl_trait_return"]
    #[doc(hidden)]
    pub const span_impl_trait_return: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_impl_trait_return"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 1760usize,
            start_col: 4usize,
            end_line: 1760usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_impl_trait_return()),
        ),
    };
    fn span_impl_trait_return() {
        static CALLED: StaticCalled = StaticCalled::new();
        static RT: StaticRuntime = static_runtime(
            |_| {
                CALLED.record();
            },
            |_| true,
        );
        fn exec(user: &str) -> impl ::std::fmt::Display {
            let (mut __span_guard, __ctxt) = match ({
                (
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                        };
                        emit::__private::Key("user")
                            .__private_key_as_default()
                            .__private_interpolated()
                            .__private_captured()
                    },
                    #[allow(unused_imports)]
                    {
                        use emit::__private::{
                            __PrivateCaptureHook as _, __PrivateOptionalCaptureHook as _,
                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                            __PrivateKeyExternalHook as _,
                        };
                        (user)
                            .__private_capture_as_default()
                            .__private_key_external()
                            .__private_interpolated()
                            .__private_captured()
                    },
                )
            }) {
                (__tmp0) => {
                    emit::__private::__private_begin_span(
                        &(RT),
                        ::emit::Path::new_raw("emit_test_ui::span"),
                        "greet {user}",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            (__tmp0.0, __tmp0.1),
                        ])),
                        emit::__private::__PrivateSpanEventMacroProps::new(
                            emit::Empty,
                            emit::Empty,
                        ),
                        emit::__private::__private_complete_span(
                            &(RT),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("greet ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    )
                }
            };
            __ctxt
                .call(move || {
                    __span_guard.start();
                    {
                        let _ = user;
                        "done"
                    }
                })
        }
        let _ = exec("Rust");
        RT.emitter().blocking_flush(Duration::from_secs(1));
        if !CALLED.was_called() {
            ::core::panicking::panic("assertion failed: CALLED.was_called()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "span::span_impl_trait_return_async"]
    #[doc(hidden)]
    pub const span_impl_trait_return_async: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span::span_impl_trait_return_async"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span.rs",
            start_line: 1784usize,
            start_col: 10usize,
            end_line: 1784usize,
            end_col: 38usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_impl_trait_return_async()),
        ),
    };
    fn span_impl_trait_return_async() {
        let body = async {
            static CALLED: StaticCalled = StaticCalled::new();
            static RT: StaticRuntime = static_runtime(
                |_| {
                    CALLED.record();
                },
                |_| true,
            );
            async fn exec(user: &str) -> impl ::std::fmt::Display {
                let (mut __span_guard, __ctxt) = match ({
                    (
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                            };
                            emit::__private::Key("user")
                                .__private_key_as_default()
                                .__private_interpolated()
                                .__private_captured()
                        },
                        #[allow(unused_imports)]
                        {
                            use emit::__private::{
                                __PrivateCaptureHook as _,
                                __PrivateOptionalCaptureHook as _,
                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                __PrivateKeyExternalHook as _,
                            };
                            (user)
                                .__private_capture_as_default()
                                .__private_key_external()
                                .__private_interpolated()
                                .__private_captured()
                        },
                    )
                }) {
                    (__tmp0) => {
                        emit::__private::__private_begin_span(
                            &(RT),
                            ::emit::Path::new_raw("emit_test_ui::span"),
                            "greet {user}",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                (__tmp0.0, __tmp0.1),
                            ])),
                            emit::__private::__PrivateSpanEventMacroProps::new(
                                emit::Empty,
                                emit::Empty,
                            ),
                            emit::__private::__private_complete_span(
                                &(RT),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("greet ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        )
                    }
                };
                __ctxt
                    .in_future(async move {
                        __span_guard.start();
                        {
                            let _ = user;
                            "done"
                        }
                    })
                    .await
            }
            let _ = exec("Rust").await;
            RT.emitter().blocking_flush(Duration::from_secs(1));
            if !CALLED.was_called() {
                ::core::panicking::panic("assertion failed: CALLED.was_called()")
            }
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
}
mod span_guard {
    use ::std::time::Duration;
    use emit::{Emitter, Props};
    use crate::util::{Called, simple_runtime};
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "span_guard::span_guard_basic"]
    #[doc(hidden)]
    pub const span_guard_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span_guard::span_guard_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span_guard.rs",
            start_line: 11usize,
            start_col: 4usize,
            end_line: 11usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_guard_basic()),
        ),
    };
    fn span_guard_basic() {
        for lvl in [
            ::std::option::Option::Some(emit::Level::Debug),
            ::std::option::Option::Some(emit::Level::Info),
            ::std::option::Option::Some(emit::Level::Warn),
            ::std::option::Option::Some(emit::Level::Error),
            ::std::option::Option::None,
        ] {
            let called = Called::new();
            let rt = simple_runtime(
                |evt| {
                    match (&"Hello, Rust", &evt.msg().to_string()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&"Hello, {user}", &evt.tpl().to_string()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&"emit_test_ui::span_guard", &evt.mdl()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    if !evt.extent().is_some() {
                        ::core::panicking::panic(
                            "assertion failed: evt.extent().is_some()",
                        )
                    }
                    match (&"Rust", &evt.props().pull::<&str, _>("user").unwrap()) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    match (&lvl, &evt.props().pull::<emit::Level, _>("lvl")) {
                        (left_val, right_val) => {
                            if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(
                                    kind,
                                    &*left_val,
                                    &*right_val,
                                    ::core::option::Option::None,
                                );
                            }
                        }
                    };
                    if !evt.props().get("trace_id").is_some() {
                        ::core::panicking::panic(
                            "assertion failed: evt.props().get(\"trace_id\").is_some()",
                        )
                    }
                    if !evt.props().get("span_id").is_some() {
                        ::core::panicking::panic(
                            "assertion failed: evt.props().get(\"span_id\").is_some()",
                        )
                    }
                    called.record();
                },
                |_| true,
            );
            let user = "Rust";
            match lvl {
                ::std::option::Option::None => {
                    let (mut guard, frame) = emit::__private::__private_begin_span(
                        &(rt),
                        ::emit::Path::new_raw("emit_test_ui::span_guard"),
                        "Hello, {user}",
                        emit::__private::core::option::Option::None::<&emit::Level>,
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("user")
                                            .__private_key_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (user)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        emit::Empty,
                        emit::__private::__private_complete_span(
                            &(rt),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            "error",
                        ),
                    );
                    frame
                        .call(move || {
                            guard.start();
                        });
                }
                ::std::option::Option::Some(emit::Level::Debug) => {
                    let (mut guard, frame) = emit::__private::__private_begin_span(
                        &(rt),
                        ::emit::Path::new_raw("emit_test_ui::span_guard"),
                        "Hello, {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Debug),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("user")
                                            .__private_key_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (user)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        emit::Empty,
                        emit::__private::__private_complete_span(
                            &(rt),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Debug),
                            ),
                            &(emit::Level::Debug),
                        ),
                    );
                    frame
                        .call(move || {
                            guard.start();
                        });
                }
                ::std::option::Option::Some(emit::Level::Info) => {
                    let (mut guard, frame) = emit::__private::__private_begin_span(
                        &(rt),
                        ::emit::Path::new_raw("emit_test_ui::span_guard"),
                        "Hello, {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Info),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("user")
                                            .__private_key_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (user)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        emit::Empty,
                        emit::__private::__private_complete_span(
                            &(rt),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Info),
                            ),
                            &(emit::Level::Info),
                        ),
                    );
                    frame
                        .call(move || {
                            guard.start();
                        });
                }
                ::std::option::Option::Some(emit::Level::Warn) => {
                    let (mut guard, frame) = emit::__private::__private_begin_span(
                        &(rt),
                        ::emit::Path::new_raw("emit_test_ui::span_guard"),
                        "Hello, {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Warn),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("user")
                                            .__private_key_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (user)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        emit::Empty,
                        emit::__private::__private_complete_span(
                            &(rt),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Warn),
                            ),
                            &(emit::Level::Warn),
                        ),
                    );
                    frame
                        .call(move || {
                            guard.start();
                        });
                }
                ::std::option::Option::Some(emit::Level::Error) => {
                    let (mut guard, frame) = emit::__private::__private_begin_span(
                        &(rt),
                        ::emit::Path::new_raw("emit_test_ui::span_guard"),
                        "Hello, {user}",
                        emit::__private::core::option::Option::Some(
                            &(emit::Level::Error),
                        ),
                        emit::__private::core::option::Option::None::<&emit::Empty>,
                        &(emit::__private::__PrivateMacroProps::from_array([
                            {
                                (
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::__private::Key("user")
                                            .__private_key_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateCaptureHook as _,
                                            __PrivateOptionalCaptureHook as _,
                                            __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                            __PrivateKeyExternalHook as _,
                                        };
                                        (user)
                                            .__private_capture_as_default()
                                            .__private_key_external()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                )
                            },
                        ])),
                        emit::Empty,
                        emit::__private::__private_complete_span(
                            &(rt),
                            emit::Template::new_ref({
                                const __TPL_PARTS: &[emit::template::Part] = &[
                                    emit::template::Part::text("Hello, ")
                                        .with_needs_escaping_raw(false),
                                    #[allow(unused_imports)]
                                    {
                                        use emit::__private::{
                                            __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                        };
                                        emit::template::Part::hole_str(
                                                #[allow(unused_imports)]
                                                {
                                                    use emit::__private::{
                                                        __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                    };
                                                    emit::__private::Key("user")
                                                        .__private_key_as_default()
                                                        .__private_interpolated()
                                                        .__private_captured()
                                                },
                                            )
                                            .__private_fmt_as_default()
                                            .__private_interpolated()
                                            .__private_captured()
                                    },
                                ];
                                __TPL_PARTS
                            }),
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Error),
                            ),
                            &(emit::Level::Error),
                        ),
                    );
                    frame
                        .call(move || {
                            guard.start();
                        });
                }
            }
            rt.emitter().blocking_flush(Duration::from_secs(1));
            if !called.was_called() {
                ::core::panicking::panic("assertion failed: called.was_called()")
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "span_guard::span_guard_basic_async"]
    #[doc(hidden)]
    pub const span_guard_basic_async: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("span_guard::span_guard_basic_async"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/span_guard.rs",
            start_line: 88usize,
            start_col: 10usize,
            end_line: 88usize,
            end_col: 32usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(span_guard_basic_async()),
        ),
    };
    fn span_guard_basic_async() {
        let body = async {
            for lvl in [
                ::std::option::Option::Some(emit::Level::Debug),
                ::std::option::Option::Some(emit::Level::Info),
                ::std::option::Option::Some(emit::Level::Warn),
                ::std::option::Option::Some(emit::Level::Error),
                ::std::option::Option::None,
            ] {
                let called = Called::new();
                let rt = simple_runtime(
                    |evt| {
                        match (&"Hello, Rust", &evt.msg().to_string()) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        match (&"Hello, {user}", &evt.tpl().to_string()) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        match (&"emit_test_ui::span_guard", &evt.mdl()) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        if !evt.extent().is_some() {
                            ::core::panicking::panic(
                                "assertion failed: evt.extent().is_some()",
                            )
                        }
                        match (&"Rust", &evt.props().pull::<&str, _>("user").unwrap()) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        match (&lvl, &evt.props().pull::<emit::Level, _>("lvl")) {
                            (left_val, right_val) => {
                                if !(*left_val == *right_val) {
                                    let kind = ::core::panicking::AssertKind::Eq;
                                    ::core::panicking::assert_failed(
                                        kind,
                                        &*left_val,
                                        &*right_val,
                                        ::core::option::Option::None,
                                    );
                                }
                            }
                        };
                        if !evt.props().get("trace_id").is_some() {
                            ::core::panicking::panic(
                                "assertion failed: evt.props().get(\"trace_id\").is_some()",
                            )
                        }
                        if !evt.props().get("span_id").is_some() {
                            ::core::panicking::panic(
                                "assertion failed: evt.props().get(\"span_id\").is_some()",
                            )
                        }
                        called.record();
                    },
                    |_| true,
                );
                let user = "Rust";
                match lvl {
                    ::std::option::Option::None => {
                        let (mut guard, frame) = emit::__private::__private_begin_span(
                            &(rt),
                            ::emit::Path::new_raw("emit_test_ui::span_guard"),
                            "Hello, {user}",
                            emit::__private::core::option::Option::None::<&emit::Level>,
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (user)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ])),
                            emit::Empty,
                            emit::__private::__private_complete_span(
                                &(rt),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::None::<&emit::Level>,
                                "error",
                            ),
                        );
                        frame
                            .in_future(async move {
                                guard.start();
                                tokio::time::sleep(Duration::from_micros(1)).await;
                                guard.complete();
                            })
                            .await;
                    }
                    ::std::option::Option::Some(emit::Level::Debug) => {
                        let (mut guard, frame) = emit::__private::__private_begin_span(
                            &(rt),
                            ::emit::Path::new_raw("emit_test_ui::span_guard"),
                            "Hello, {user}",
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Debug),
                            ),
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (user)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ])),
                            emit::Empty,
                            emit::__private::__private_complete_span(
                                &(rt),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::Some(
                                    &(emit::Level::Debug),
                                ),
                                &(emit::Level::Debug),
                            ),
                        );
                        frame
                            .in_future(async move {
                                guard.start();
                                tokio::time::sleep(Duration::from_micros(1)).await;
                                guard.complete();
                            })
                            .await;
                    }
                    ::std::option::Option::Some(emit::Level::Info) => {
                        let (mut guard, frame) = emit::__private::__private_begin_span(
                            &(rt),
                            ::emit::Path::new_raw("emit_test_ui::span_guard"),
                            "Hello, {user}",
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Info),
                            ),
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (user)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ])),
                            emit::Empty,
                            emit::__private::__private_complete_span(
                                &(rt),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::Some(
                                    &(emit::Level::Info),
                                ),
                                &(emit::Level::Info),
                            ),
                        );
                        frame
                            .in_future(async move {
                                guard.start();
                                tokio::time::sleep(Duration::from_micros(1)).await;
                                guard.complete();
                            })
                            .await;
                    }
                    ::std::option::Option::Some(emit::Level::Warn) => {
                        let (mut guard, frame) = emit::__private::__private_begin_span(
                            &(rt),
                            ::emit::Path::new_raw("emit_test_ui::span_guard"),
                            "Hello, {user}",
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Warn),
                            ),
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (user)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ])),
                            emit::Empty,
                            emit::__private::__private_complete_span(
                                &(rt),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::Some(
                                    &(emit::Level::Warn),
                                ),
                                &(emit::Level::Warn),
                            ),
                        );
                        frame
                            .in_future(async move {
                                guard.start();
                                tokio::time::sleep(Duration::from_micros(1)).await;
                                guard.complete();
                            })
                            .await;
                    }
                    ::std::option::Option::Some(emit::Level::Error) => {
                        let (mut guard, frame) = emit::__private::__private_begin_span(
                            &(rt),
                            ::emit::Path::new_raw("emit_test_ui::span_guard"),
                            "Hello, {user}",
                            emit::__private::core::option::Option::Some(
                                &(emit::Level::Error),
                            ),
                            emit::__private::core::option::Option::None::<&emit::Empty>,
                            &(emit::__private::__PrivateMacroProps::from_array([
                                {
                                    (
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateCaptureHook as _,
                                                __PrivateOptionalCaptureHook as _,
                                                __PrivateOptionalHook as _, __PrivateInterpolatedHook as _,
                                                __PrivateKeyExternalHook as _,
                                            };
                                            (user)
                                                .__private_capture_as_default()
                                                .__private_key_external()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    )
                                },
                            ])),
                            emit::Empty,
                            emit::__private::__private_complete_span(
                                &(rt),
                                emit::Template::new_ref({
                                    const __TPL_PARTS: &[emit::template::Part] = &[
                                        emit::template::Part::text("Hello, ")
                                            .with_needs_escaping_raw(false),
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::template::Part::hole_str(
                                                    #[allow(unused_imports)]
                                                    {
                                                        use emit::__private::{
                                                            __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                                        };
                                                        emit::__private::Key("user")
                                                            .__private_key_as_default()
                                                            .__private_interpolated()
                                                            .__private_captured()
                                                    },
                                                )
                                                .__private_fmt_as_default()
                                                .__private_interpolated()
                                                .__private_captured()
                                        },
                                    ];
                                    __TPL_PARTS
                                }),
                                emit::__private::core::option::Option::Some(
                                    &(emit::Level::Error),
                                ),
                                &(emit::Level::Error),
                            ),
                        );
                        frame
                            .in_future(async move {
                                guard.start();
                                tokio::time::sleep(Duration::from_micros(1)).await;
                                guard.complete();
                            })
                            .await;
                    }
                }
                rt.emitter().blocking_flush(Duration::from_secs(1));
                if !called.was_called() {
                    ::core::panicking::panic("assertion failed: called.was_called()")
                }
            }
        };
        let mut body = body;
        #[allow(unused_mut)]
        let mut body = unsafe {
            ::tokio::macros::support::Pin::new_unchecked(&mut body)
        };
        let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
        #[allow(
            clippy::expect_used,
            clippy::diverging_sub_expression,
            clippy::needless_return,
            clippy::unwrap_in_result
        )]
        {
            use tokio::runtime::Builder;
            return Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body);
        }
    }
}
mod tpl {
    #[allow(unused_imports)]
    use crate::shadow::*;
    extern crate test;
    #[rustc_test_marker = "tpl::tpl_basic"]
    #[doc(hidden)]
    pub const tpl_basic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tpl::tpl_basic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/tpl.rs",
            start_line: 5usize,
            start_col: 4usize,
            end_line: 5usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(tpl_basic()),
        ),
    };
    fn tpl_basic() {
        let tpl = emit::Template::new_ref({
            const __TPL_PARTS: &[emit::template::Part] = &[
                emit::template::Part::text("Hello, ").with_needs_escaping_raw(false),
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("user")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as_default()
                        .__private_interpolated()
                        .__private_uncaptured()
                },
            ];
            __TPL_PARTS
        });
        let parts = tpl.parts().collect::<::std::vec::Vec<_>>();
        match (&"Hello, ", &parts[0].as_text().unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"user", &parts[1].label().unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "tpl::tpl_event_meta"]
    #[doc(hidden)]
    pub const tpl_event_meta: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tpl::tpl_event_meta"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/tpl.rs",
            start_line: 15usize,
            start_col: 4usize,
            end_line: 15usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(tpl_event_meta()),
        ),
    };
    fn tpl_event_meta() {
        let _ = emit::Template::new_ref({
            const __TPL_PARTS: &[emit::template::Part] = &[
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("ts_start")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as_default()
                        .__private_interpolated()
                        .__private_uncaptured()
                },
                emit::template::Part::text("..").with_needs_escaping_raw(false),
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("ts")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as_default()
                        .__private_interpolated()
                        .__private_uncaptured()
                },
                emit::template::Part::text(" ").with_needs_escaping_raw(false),
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("mdl")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as_default()
                        .__private_interpolated()
                        .__private_uncaptured()
                },
                emit::template::Part::text(" ").with_needs_escaping_raw(false),
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("tpl")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as_default()
                        .__private_interpolated()
                        .__private_uncaptured()
                },
                emit::template::Part::text(" ").with_needs_escaping_raw(false),
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("msg")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as_default()
                        .__private_interpolated()
                        .__private_uncaptured()
                },
            ];
            __TPL_PARTS
        });
    }
    extern crate test;
    #[rustc_test_marker = "tpl::tpl_cfg"]
    #[doc(hidden)]
    pub const tpl_cfg: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tpl::tpl_cfg"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/tpl.rs",
            start_line: 20usize,
            start_col: 4usize,
            end_line: 20usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(tpl_cfg()),
        ),
    };
    fn tpl_cfg() {
        match (
            &"Hello, {user}",
            &emit::Template::new_ref({
                    const __TPL_PARTS: &[emit::template::Part] = &[
                        emit::template::Part::text("Hello, ")
                            .with_needs_escaping_raw(false),
                        {
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::template::Part::hole_str(
                                        #[allow(unused_imports)]
                                        {
                                            use emit::__private::{
                                                __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                            };
                                            emit::__private::Key("user")
                                                .__private_key_as_default()
                                                .__private_interpolated()
                                                .__private_uncaptured()
                                        },
                                    )
                                    .__private_fmt_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            }
                        },
                    ];
                    __TPL_PARTS
                })
                .to_string(),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "tpl::tpl_fmt"]
    #[doc(hidden)]
    pub const tpl_fmt: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tpl::tpl_fmt"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/tpl.rs",
            start_line: 29usize,
            start_col: 4usize,
            end_line: 29usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(tpl_fmt()),
        ),
    };
    fn tpl_fmt() {
        let tpl = emit::Template::new_ref({
            const __TPL_PARTS: &[emit::template::Part] = &[
                emit::template::Part::text("Hello, ").with_needs_escaping_raw(false),
                #[allow(unused_imports)]
                {
                    use emit::__private::{
                        __PrivateFmtHook as _, __PrivateInterpolatedHook as _,
                    };
                    emit::template::Part::hole_str(
                            #[allow(unused_imports)]
                            {
                                use emit::__private::{
                                    __PrivateKeyHook as _, __PrivateInterpolatedHook as _,
                                };
                                emit::__private::Key("user")
                                    .__private_key_as_default()
                                    .__private_interpolated()
                                    .__private_uncaptured()
                            },
                        )
                        .__private_fmt_as(
                            emit::template::Formatter::new(|v, f| {
                                f.write_fmt(format_args!("{0:?}", v))
                            }),
                        )
                        .__private_interpolated()
                        .__private_uncaptured()
                },
            ];
            __TPL_PARTS
        });
        let parts = tpl.parts().collect::<::std::vec::Vec<_>>();
        match (&"Hello, ", &parts[0].as_text().unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&"user", &parts[1].label().unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        if !parts[1].formatter().is_some() {
            ::core::panicking::panic("assertion failed: parts[1].formatter().is_some()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "tpl::tpl_escape"]
    #[doc(hidden)]
    pub const tpl_escape: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tpl::tpl_escape"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "test/ui/src/tpl.rs",
            start_line: 44usize,
            start_col: 4usize,
            end_line: 44usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(tpl_escape()),
        ),
    };
    fn tpl_escape() {
        let tpl = emit::Template::new_ref({
            const __TPL_PARTS: &[emit::template::Part] = &[
                emit::template::Part::text("Hello, {user}").with_needs_escaping_raw(true),
            ];
            __TPL_PARTS
        });
        match (&"Hello, {{user}}", &tpl.to_string()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        let parts = tpl.parts().collect::<::std::vec::Vec<_>>();
        match (&"Hello, {user}", &parts[0].as_text().unwrap()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
mod shadow {
    #![allow(dead_code)]
    pub struct Result;
    pub struct Ok;
    pub struct Err;
    pub struct Some;
    pub struct None;
    pub struct String;
    pub struct Vec;
    pub mod core {}
    pub mod std {}
}
#[allow(clippy::incompatible_msrv)]
mod compile {}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &emit_basic,
            &emit_cfg,
            &emit_empty,
            &emit_event_filter,
            &emit_event_when,
            &emit_evt,
            &emit_evt_ref,
            &emit_extent_point,
            &emit_extent_point_ref,
            &emit_extent_span,
            &emit_extent_span_ref,
            &emit_filter,
            &emit_interpolation,
            &emit_key,
            &emit_key_exotic,
            &emit_mdl,
            &emit_mdl_ref,
            &emit_props,
            &emit_props_precedence,
            &emit_props_ref,
            &emit_rt_ref,
            &emit_when,
            &emit_when_ref,
            &event_base_props,
            &event_basic,
            &event_extent,
            &event_mdl,
            &metric,
            &props_as_debug,
            &props_as_display,
            &props_as_value,
            &props_basic,
            &props_capture_err_as_non_err,
            &props_capture_err_string,
            &props_capture_lvl,
            &props_capture_lvl_as_non_lvl,
            &props_capture_lvl_string,
            &props_capture_span_id,
            &props_capture_span_id_as_non_span_id,
            &props_capture_span_id_string,
            &props_capture_span_id_u64,
            &props_capture_span_parent,
            &props_capture_span_parent_as_non_span_id,
            &props_capture_span_parent_string,
            &props_capture_trace_id,
            &props_capture_trace_id_as_non_trace_id,
            &props_capture_trace_id_string,
            &props_capture_trace_id_u128,
            &props_cfg,
            &props_event_meta,
            &props_external,
            &props_key,
            &props_key_expr_str,
            &props_optional,
            &props_optional_multi_attr,
            &props_optional_ref,
            &props_uncooked,
            &sample_agg,
            &sample_agg_specific,
            &sample_basic,
            &sample_name,
            &sample_props,
            &sample_value_capture,
            &sample_well_known_props_precedence,
            &async_span_by_value_arg,
            &async_span_explicit_ids,
            &async_span_fn_name,
            &span_basic,
            &span_basic_async,
            &span_by_value_arg,
            &span_evt_props_basic,
            &span_explicit_ids,
            &span_explicit_ids_ctxt,
            &span_filter,
            &span_fn_name,
            &span_guard,
            &span_guard_props,
            &span_impl_trait_return,
            &span_impl_trait_return_async,
            &span_mdl,
            &span_name_escape,
            &span_rt_ref,
            &span_setup,
            &span_setup_async,
            &span_well_known_props_precedence,
            &span_when,
            &span_guard_basic,
            &span_guard_basic_async,
            &tpl_basic,
            &tpl_cfg,
            &tpl_escape,
            &tpl_event_meta,
            &tpl_fmt,
        ],
    )
}
