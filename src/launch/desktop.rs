#[cfg(not(any(target_family = "wasm", target_os = "android")))]
use crate::App;
use crate::appinfo;
#[cfg(not(any(target_family = "wasm", target_os = "android")))]
use winit::event_loop::EventLoop;

#[cfg(not(any(target_family = "wasm", target_os = "android")))]
/// Launches the app for a standard desktop target (Windows/Linux)
pub fn run() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        print!("\x1b[96m\x1b[1m[Note]\x1b[0m Log level was undefined, setting it to ");
        #[cfg(debug_assertions)]
        {
            print!("debug\n\r");
            unsafe { std::env::set_var("RUST_LOG", "debug") };
        }
        #[cfg(not(debug_assertions))]
        {
            print!("info\n\r");
            unsafe { std::env::set_var("RUST_LOG", "info") };
        }
    }

    env_logger::init();
    log::info!("Launching {} {}", appinfo::NAME, appinfo::VERSION);

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app)?;

    Ok(())
}
