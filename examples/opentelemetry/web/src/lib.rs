use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let _ = emit::setup()
        .emit_to(
            emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: emit::pkg!(),
                })
                .logs(emit_otlp::logs_http_proto("http://localhost:4418/v1/logs"))
                .traces(emit_otlp::traces_http_proto(
                    "http://localhost:4418/v1/traces",
                ))
                .metrics(emit_otlp::metrics_http_proto(
                    "http://localhost:4418/v1/metrics",
                ))
                .spawn(),
        )
        .try_init();
}

#[wasm_bindgen]
pub fn run() {
    emit::debug!("Hello {user}", user: "Web");
    emit::info!("Hello {user}", user: "Web");
    emit::warn!("Hello {user}", user: "Web");
    emit::error!("Hello {user}", user: "Web");

    exec_expensive_operation();
}

#[emit::span("Expensive operation")]
fn exec_expensive_operation() {}
