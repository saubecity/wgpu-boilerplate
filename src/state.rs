use anyhow::Context;
use wgpu::wgt::TextureViewDescriptor;

use std::any;
use std::sync::Arc;
use wgpu::{
    Backends, CommandEncoderDescriptor, CurrentSurfaceTexture, RenderPassColorAttachment,
    SurfaceTexture,
};
use wgpu::{
    CompositeAlphaMode, Device, InstanceDescriptor, Limits, Surface, SurfaceCapabilities,
    SurfaceColorSpace, SurfaceConfiguration, TextureFormat, TextureUsages,
};
use winit::{dpi::PhysicalSize, window::Window};

use wgpu::DeviceType;

use crate::appinfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderAction {
    Idle,
    Continue,
}

#[derive(Debug, Clone)]
pub struct GraphicsPreferences {
    hdr: bool,
    power_preference: wgpu::PowerPreference,
}

impl Default for GraphicsPreferences {
    fn default() -> Self {
        Self {
            hdr: false,
            power_preference: wgpu::PowerPreference::LowPower,
        }
    }
}

pub struct GraphicsState {
    surface: wgpu::Surface<'static>,
    instance: wgpu::Instance,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    downlevel: wgpu::DownlevelCapabilities,
    pref: GraphicsPreferences,
    is_surface_configured: bool,
}

impl GraphicsState {
    pub async fn new(window: &Arc<Window>) -> anyhow::Result<Self> {
        let pref = GraphicsPreferences::default();

        let instance_desc = InstanceDescriptor {
            backends: Backends::all(),
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        };

        #[cfg(target_family = "wasm")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(instance_desc).await;
        #[cfg(not(target_family = "wasm"))]
        let instance = wgpu::Instance::new(instance_desc);

        let surface = instance
            .create_surface(window.clone())
            .context("Could not create surface")?;

        let adapter = Self::pick_adapter(&instance, &surface, pref.power_preference)
            .await
            .context("Could not pick adapter")?;

        let downlevel = adapter.get_downlevel_capabilities();
        let adapter_info = adapter.get_info();

        log::info!(
            "Using {} on backend {}",
            adapter_info.name,
            adapter_info.backend.to_str()
        );

        let device_desc = wgpu::DeviceDescriptor {
            label: Some("msdftext - GPU"),
            required_limits: Limits::downlevel_webgl2_defaults(),
            ..Default::default()
        };

        let (device, queue) = adapter.request_device(&device_desc).await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let size = window.inner_size();
        let config = Self::create_surface_config(pref.hdr, &surface_caps, &size);
        let mut is_surface_configured = Self::configure_surface_impl(&surface, &config, &device);

        Ok(Self {
            surface,
            instance,
            queue,
            config,
            device,
            downlevel,
            pref,
            is_surface_configured,
        })
    }

    pub fn render(&mut self, window: &Window) -> anyhow::Result<RenderAction> {
        if !self.is_surface_configured {
            return Err(anyhow::anyhow!(
                "Surface is not configured, this is mostly normal"
            ));
        }

        let output = Self::get_surface_texture(&self.surface, &self.device, &self.config)
            .ok_or_else(|| anyhow::anyhow!("Failed to get surface texture"))?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("msdftext - Command Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("msdftext - render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                })],
                ..Default::default()
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);

        Ok(RenderAction::Idle)
    }

    fn get_surface_texture(
        surface: &Surface,
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> Option<SurfaceTexture> {
        match surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => Some(surface_texture),
            CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                log::debug!("Suboptimal texture, reconfiguring surface");

                Self::configure_surface_impl(surface, config, device);
                return Some(surface_texture);
            }
            CurrentSurfaceTexture::Outdated => {
                Self::configure_surface_impl(surface, config, device);
                return None;
            }
            // seperated later for android stuff
            CurrentSurfaceTexture::Lost => {
                Self::configure_surface_impl(surface, config, device);
                return None;
            }
            _ => None,
        }
    }

    //to be removed
    fn surface_texture_result_to_message(status: CurrentSurfaceTexture) -> String {
        match status {
            CurrentSurfaceTexture::Success(surface_texture) => {
                String::from("Successfully acquired a surface texture with no issues")
            }
            CurrentSurfaceTexture::Suboptimal(surface_texture) => String::from(
                "Successfully acquired a surface texture, but texture no longer matches the properties of the underlying surface",
            ),
            CurrentSurfaceTexture::Timeout => {
                String::from("A timeout was encountered while trying to acquire the next frame")
            }
            CurrentSurfaceTexture::Occluded => {
                String::from("The window is occluded (e.g. minimized or behind another window)")
            }
            CurrentSurfaceTexture::Outdated => String::from(
                "The underlying surface has changed, and therefore the surface configuration is outdated",
            ),
            CurrentSurfaceTexture::Lost => {
                String::from("The surface has been lost and needs to be recreated")
            }
            CurrentSurfaceTexture::Validation => String::from(
                "A validation error inside Surface::get_current_texture was raised and caught by an error scope or on_uncaptured_error",
            ),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.configure_surface();
    }

    fn configure_surface_impl(
        surface: &Surface,
        config: &SurfaceConfiguration,
        device: &Device,
    ) -> bool {
        if config.width > 0 && config.height > 0 {
            surface.configure(device, config);
            return true;
        }
        false
    }

    pub fn configure_surface(&mut self) {
        self.is_surface_configured =
            Self::configure_surface_impl(&self.surface, &self.config, &self.device);
    }

    /// Creates a surface configuration from a surface and window
    pub fn create_surface_config(
        prefer_hdr: bool,
        surface_caps: &SurfaceCapabilities,
        window_size: &PhysicalSize<u32>,
    ) -> SurfaceConfiguration {
        let format = Self::pick_surface_format(surface_caps, prefer_hdr);

        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            width: window_size.width,
            height: window_size.height,
            alpha_mode: Self::pick_alpha_mode(surface_caps, None),
            present_mode: Self::pick_present_mode(surface_caps, None),
            color_space: Self::pick_color_space(prefer_hdr, format, surface_caps),
            desired_maximum_frame_latency: 2,
            format,
            view_formats: vec![],
        }
    }

    pub fn pick_color_space(
        prefer_hdr: bool,
        format: TextureFormat,
        surface_caps: &SurfaceCapabilities,
    ) -> SurfaceColorSpace {
        if prefer_hdr {
            #[cfg(target_family = "wasm")]
            let pref = SurfaceColorSpace::ExtendedSrgb;
            #[cfg(not(target_family = "wasm"))]
            let pref = SurfaceColorSpace::ExtendedSrgbLinear;

            if let Some(color_spaces) = pref.to_color_spaces() {
                if color_spaces
                    .iter()
                    .any(|cs| surface_caps.color_spaces(format).contains(cs))
                {
                    return pref;
                }
            }
        }

        SurfaceColorSpace::Auto
    }

    /// An internal function to pick a surface format
    pub fn pick_surface_format(
        surface_caps: &SurfaceCapabilities,
        prefer_hdr: bool,
    ) -> TextureFormat {
        if prefer_hdr {
            if let Some(&fp16) = surface_caps
                .formats
                .iter()
                .find(|&&f| f == TextureFormat::Rgba16Float)
            {
                return fp16;
            }
        }

        surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .or_else(|| surface_caps.formats.first().copied())
            .unwrap_or(TextureFormat::Bgra8UnormSrgb)
    }

    /// Picks an alpha mode that is supported by the GPU
    pub fn pick_alpha_mode(
        surface_caps: &SurfaceCapabilities,
        preference: Option<CompositeAlphaMode>,
    ) -> CompositeAlphaMode {
        preference
            .into_iter()
            .chain([CompositeAlphaMode::Opaque, CompositeAlphaMode::Auto])
            .find(|alpha_mode| surface_caps.alpha_modes.contains(alpha_mode))
            .unwrap_or(surface_caps.alpha_modes[0])
    }

    /// Picks a present that is compatible with the current GPU
    pub fn pick_present_mode(
        surface_caps: &SurfaceCapabilities,
        preference: Option<wgpu::PresentMode>,
    ) -> wgpu::PresentMode {
        preference
            .into_iter()
            .chain([wgpu::PresentMode::Mailbox])
            .find(|mode| surface_caps.present_modes.contains(mode))
            .unwrap_or(wgpu::PresentMode::AutoVsync)
    }

    /// Picks the most suitable adapter and backend depending on the platform
    ///
    /// **Windows**: Tries DX12 first, fallbacks to Vulkan or GL
    /// **Others**: Lets wgpu choose it thru request_adapter
    ///
    /// If the platform is not here, than it lets wgpu choose instead
    pub async fn pick_adapter(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        power_preference: wgpu::PowerPreference,
    ) -> Option<wgpu::Adapter> {
        #[cfg(target_family = "windows")]
        if let Some(adapter) =
            Self::pick_adapter_from_backend(instance, surface, Backends::DX12).await
        {
            return Some(adapter);
        };

        #[cfg(target_family = "windows")]
        log::warn!("Could not use DX12, Mailbox present mode may not be supported");

        let adapter_params = wgpu::RequestAdapterOptionsBase {
            power_preference: power_preference,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
            ..Default::default()
        };

        instance.request_adapter(&adapter_params).await.ok()
    }

    #[cfg(not(target_family = "wasm"))]
    /// An internal function to pick an adapter that supports the specified backend
    async fn pick_adapter_from_backend(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        backend: Backends,
    ) -> Option<wgpu::Adapter> {
        instance
            .enumerate_adapters(backend)
            .await
            .into_iter()
            .filter(|adapter| adapter.is_surface_supported(surface))
            .max_by_key(|adapter| match adapter.get_info().device_type {
                DeviceType::IntegratedGpu => 2,
                DeviceType::DiscreteGpu => 1,
                _ => 0,
            })
    }
}
