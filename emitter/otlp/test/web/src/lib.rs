use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let _ = emit::setup()
        .emit_to(emit_web::console())
        .with_clock(emit_web::date_clock())
        .with_rng(emit_web::crypto_rng())
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
