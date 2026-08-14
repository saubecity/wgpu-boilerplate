use msdftext::launch;

fn main() {
    #[cfg(not(any(target_family = "wasm", target_os = "android")))]
    launch::desktop::run().unwrap();
    #[cfg(target_family = "wasm")]
    launch::wasm::run();
}
