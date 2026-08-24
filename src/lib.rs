pub mod appinfo;
pub mod events;
pub mod launch;
mod state;

use state::GraphicsState;
use std::sync::Arc;
#[cfg(target_family = "wasm")]
use wasm_bindgen::JsCast;
#[cfg(target_family = "wasm")]
use wasm_bindgen::UnwrapThrowExt;
use winit::event_loop::EventLoop;
#[cfg(target_family = "wasm")]
use winit::platform::web::WindowAttributesExtWebSys;
use winit::{application::ApplicationHandler, event_loop, window::Window};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, WindowAttributes},
};

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<GraphicsState>,
    proxy: Option<winit::event_loop::EventLoopProxy<events::UserEvent>>,
}

impl App {
    pub fn new(event_loop: &EventLoop<events::UserEvent>) -> Self {
        let proxy = Some(event_loop.create_proxy());

        Self {
            state: None,
            window: None,
            proxy,
        }
    }

    #[cfg(not(target_family = "wasm"))]
    /*fn get_window_attributes(default_attributes: WindowAttributes) -> WindowAttributes {
        default_attributes
            .clone()
            .with_inner_size(LogicalSize::new(1280, 720))
    }*/

    fn get_window_attributes(default_attributes: WindowAttributes) -> WindowAttributes {
        default_attributes.with_inner_size(LogicalSize::new(1280, 720))
    }

    #[cfg(target_family = "wasm")]
    fn get_window_attributes(default_attributes: WindowAttributes) -> WindowAttributes {
        let window = wgpu::web_sys::window().expect("Failed to get browser Window");
        let document = window.document().unwrap_throw();
        document.set_title(appinfo::NAME);
        let canvas = document
            .get_element_by_id(appinfo::CANVAS_ID)
            .unwrap_throw();
        let html_canvas_element = canvas.unchecked_into();
        default_attributes.with_canvas(Some(html_canvas_element))
    }
}

impl ApplicationHandler<events::UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        };

        let window_attributes = Self::get_window_attributes(Window::default_attributes());

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_title("msdftext");

        #[cfg(not(target_family = "wasm"))]
        {
            match pollster::block_on(GraphicsState::new(&window)) {
                Ok(state) => self.state = Some(state),
                Err(err) => {
                    log::error!("Failed to initialize graphics: {err:#?}");
                    event_loop.exit();
                }
            }
        }

        #[cfg(target_family = "wasm")]
        {
            let window_clone = window.clone();
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    match proxy.send_event(events::UserEvent::WasmGraphicsInitialized(
                        GraphicsState::new(&window_clone).await.unwrap(),
                    )) {
                        Ok(_) => return,
                        Err(_) => {
                            log::error!("Wopsie Oupsie, something bad happend");
                            return;
                        }
                    };
                });
            }
        }

        self.window = Some(window);
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.state = None
    }

    fn user_event(&mut self, event_loop: &event_loop::ActiveEventLoop, event: events::UserEvent) {
        match event {
            #[cfg(target_family = "wasm")]
            events::UserEvent::WasmGraphicsInitialized(state) => {
                self.state = Some(state);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(window) = &mut self.window else {
            return;
        };

        let Some(state) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
                //window.request_redraw();
            }

            WindowEvent::RedrawRequested => match state.render(window) {
                Err(err) => log::error!("Render Failed: {}", err),
                _ => {}
            },

            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => match (event.physical_key, event.repeat, event.state) {
                (PhysicalKey::Code(KeyCode::Escape), false, ElementState::Pressed) => {
                    event_loop.exit();
                }
                (PhysicalKey::Code(KeyCode::F11), false, ElementState::Pressed) => {
                    if window.fullscreen().is_none() {
                        window.set_fullscreen(Some(Fullscreen::Borderless(Default::default())));
                    } else {
                        window.set_fullscreen(None);
                    }
                }

                _ => {}
            },

            _ => {}
        }
    }
}
