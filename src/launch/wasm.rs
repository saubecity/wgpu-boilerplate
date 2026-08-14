#[cfg(target_family = "wasm")]
use {
    crate::App, crate::appinfo, wasm_bindgen::prelude::*, winit::event_loop::EventLoop,
    winit::platform::web::EventLoopExtWebSys,
};

#[cfg(target_family = "wasm")]
#[wasm_bindgen(start)]
// Launches the app for a WASM target
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Trace).expect("Could not init logger");

    log::info!("Launching {} {} on WASM", appinfo::NAME, appinfo::VERSION);

    let event_loop = winit::event_loop::EventLoop::with_user_event()
        .build()
        .expect("Failed to build WASM event loop");

    let app = App::new(&event_loop);
    event_loop.spawn_app(app);
}
