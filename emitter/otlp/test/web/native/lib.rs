use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub fn http_proto() -> String {
    run_test(emit_otlp::logs_http_proto("http://localhost:34318/v1/logs"))
}

#[wasm_bindgen]
pub fn http_json() -> String {
    run_test(emit_otlp::logs_http_json("http://localhost:34318/v1/logs"))
}

fn run_test(builder: emit_otlp::OtlpLogsBuilder) -> String {
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

    // Flush `emit_otlp` and read the output from the collector
    rt.blocking_flush(std::time::Duration::from_secs(10));

    fragment
}
