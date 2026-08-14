#[cfg(target_family = "wasm")]
use crate::state::GraphicsState;

pub enum UserEvent {
    #[cfg(target_family = "wasm")]
    WasmGraphicsInitialized(GraphicsState),
    RedrawRequest,
}
