use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let _ = emit::setup()
        .emit_to(emit::runtime::AssertInternal(emit::emitter::from_fn(
            |evt| {
                log(&format!("{evt:?}"));
            },
        )))
        .init_internal();
}

#[wasm_bindgen]
pub async fn http_proto() -> String {
    run_test(emit_otlp::logs_http_proto("http://localhost:34318/v1/logs")).await
}

#[wasm_bindgen]
pub async fn http_json() -> String {
    run_test(emit_otlp::logs_http_json("http://localhost:34318/v1/logs")).await
}

async fn run_test(builder: emit_otlp::OtlpLogsBuilder) -> String {
    let fragment = uuid::Uuid::new_v4().to_string();

    let service_name = emit::pkg!();

    let resource = emit::props! {
        #[emit::key("service.name")]
        service_name,
    };

    let emitter = emit_otlp::new().resource(resource).logs(builder).spawn();

    let rt = emit::runtime::Runtime::new().with_emitter(emitter);

    // Emit a log event
    emit::info!(rt, "A log message {fragment}");

    // Flush `emit_otlp`
    let flushed = rt.emitter().flush(std::time::Duration::from_secs(10)).await;
    assert!(flushed);

    fragment
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(msg: &str);
}
