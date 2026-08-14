pub mod appinfo;
pub mod events;
pub mod launch;
mod state;

use std::sync::Arc;
#[cfg(target_family = "wasm")]
use wasm_bindgen::UnwrapThrowExt;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Fullscreen,
};

#[cfg(target_family = "wasm")]
use winit::event_loop::EventLoop;

use state::GraphicsState;
use winit::{application::ApplicationHandler, event_loop, window::Window};

pub struct App {
    window: Option<Arc<Window>>,
    state: Option<GraphicsState>,
    #[cfg(target_family = "wasm")]
    proxy: Option<winit::event_loop::EventLoopProxy<events::UserEvent>>,
}

impl App {
    pub fn new(#[cfg(target_family = "wasm")] event_loop: &EventLoop<events::UserEvent>) -> Self {
        #[cfg(target_family = "wasm")]
        let proxy = Some(event_loop.create_proxy());

        Self {
            state: None,
            window: None,
            #[cfg(target_family = "wasm")]
            proxy,
        }
    }
}

impl ApplicationHandler<events::UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        };

        let mut window_attributes = Window::default_attributes();

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            window_attributes = window_attributes.with_inner_size(LogicalSize::new(1280, 720));
        }

        #[cfg(target_family = "wasm")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().expect("Failed to get browser Window");
            let document = window.document().unwrap_throw();
            document.set_title(appinfo::NAME);
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

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
        self.window = Some(window);
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.state = None
    }

    fn user_event(&mut self, event_loop: &event_loop::ActiveEventLoop, event: events::UserEvent) {
        match event {
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let window = match &mut self.window {
            Some(window) => window,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

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
