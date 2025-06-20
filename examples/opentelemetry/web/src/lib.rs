use wasm_bindgen::prelude::*;

/*
Configure `emit`.

The shape of this function is likely going to be similar in all web applications.
We'll call this early on in our `index.html`.
*/
#[wasm_bindgen]
pub fn setup() -> Setup {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let rt = emit::setup()
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

    // Return a value we can use to flush later
    Setup {
        otlp: rt.map(|rt| rt.emitter()),
    }
}

/*
This type is a wrapper around a configured `Otlp` emitter that we can flush asynchronously.

`emit` itself only exposes synchronous flushing, but `emit_otlp` has an asynchronous flush method.
*/
#[wasm_bindgen]
pub struct Setup {
    otlp: Option<&'static emit_otlp::Otlp>,
}

#[wasm_bindgen]
impl Setup {
    #[wasm_bindgen]
    pub async fn flush(&self) {
        if let Some(ref otlp) = self.otlp {
            otlp.flush(std::time::Duration::from_secs(5)).await;
        }
    }
}

/*
Do some work, emitting events along the way.
*/
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
