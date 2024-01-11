mod error;

pub use error::Error as ContextError;
pub use wgpu;
use wgpu::{Adapter, Device, Queue, Surface, SurfaceCapabilities, TextureFormat};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct Context {
    window: Window,
    surface: Surface,

    adapter: Adapter,
    device: Device,
    queue: Queue,

    capabilities: SurfaceCapabilities,
}

impl Context {
    pub fn new<T>(
        event_loop: &EventLoop<T>,
        window: WindowBuilder,
        features: impl FnOnce(&wgpu::Adapter) -> wgpu::Features,
        limits: wgpu::Limits,
    ) -> Result<Self, ContextError> {
        let window = window.with_visible(false).build(event_loop)?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }?;

        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    // Request an adapter which can render to our surface
                    compatible_surface: Some(&surface),
                })
                .await
                .ok_or_else(|| ContextError::AdapterCreationError)?;

            let adapter_limits = adapter.limits();

            if !limits.check_limits(&adapter_limits) {
                return Err(ContextError::LimitsSurpassed);
            }

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: features(&adapter),
                        limits: adapter_limits,
                    },
                    None,
                )
                .await?;

            Ok::<_, ContextError>((adapter, device, queue))
        })?;

        let capabilities = surface.get_capabilities(&adapter);

        Ok(Context {
            window,
            surface,

            adapter,
            device,
            queue,

            capabilities,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn capabilities(&self) -> &SurfaceCapabilities {
        &self.capabilities
    }

    pub fn formats(&self) -> &[TextureFormat] {
        &self.capabilities.formats
    }

    pub fn view_format(&self) -> TextureFormat {
        self.capabilities.formats[0]
    }
}
