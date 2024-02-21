mod error;

use std::sync::Arc;

pub use error::Error as ContextBuildError;
use error::Error;
pub use wgpu;
use wgpu::{
    Adapter,
    Device,
    Queue,
    Surface,
    SurfaceCapabilities,
    TextureFormat,
};
use winit::{
    event_loop::EventLoop,
    window::{
        Window,
        WindowBuilder,
    },
};

struct WindowData {
    window: Arc<Window>,
    surface: Surface<'static>,
    capabilities: SurfaceCapabilities,
}

pub struct ContextBuilder {
    features: Box<dyn FnOnce(&wgpu::Adapter) -> wgpu::Features>,
    limits: wgpu::Limits,

    window: Option<WindowBuilder>,
}

impl ContextBuilder {
    pub fn new(
        features: impl FnOnce(&wgpu::Adapter) -> wgpu::Features + 'static,
        limits: wgpu::Limits,
    ) -> Self {
        Self {
            features: Box::new(features),
            limits,
            window: None,
        }
    }

    pub fn with_window(self, window: WindowBuilder) -> Self {
        Self {
            window: Some(window),
            ..self
        }
    }

    pub fn has_window(&self) -> bool {
        self.window.is_some()
    }

    pub fn build<T: 'static>(
        self,
        event_loop: Option<&EventLoop<T>>,
    ) -> Result<Context, ContextBuildError> {
        let Self {
            features,
            limits,
            window,
        } = self;

        let window_info = event_loop.zip(window);

        Context::create(window_info, features, limits)
    }
}

pub struct Context {
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,

    window_data: Option<WindowData>,
}

impl Context {
    fn create<T>(
        window_info: Option<(&EventLoop<T>, WindowBuilder)>,
        features: impl FnOnce(&wgpu::Adapter) -> wgpu::Features,
        limits: wgpu::Limits,
    ) -> Result<Self, ContextBuildError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let (mut window, mut surface) = if let Some((event_loop, window)) = window_info {
            // create an invisible window
            let window = Arc::new(window.with_visible(false).build(event_loop)?);
            // and a surface to put a gfx context on
            let surface = instance.create_surface(Arc::clone(&window))?;

            (Some(window), Some(surface))
        } else {
            (None, None)
        };

        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    // Request an adapter which can render to our surface
                    compatible_surface: surface.as_ref(),
                })
                .await
                .ok_or_else(|| Error::AdapterCreationError)?;

            let adapter_limits = adapter.limits();

            if !limits.check_limits(&adapter_limits) {
                return Err(Error::LimitsSurpassed);
            }

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: features(&adapter),
                        required_limits: adapter_limits,
                    },
                    None,
                )
                .await?;

            Ok::<_, Error>((adapter, device, queue))
        })?;

        let window_data = if let (Some(surface), Some(window)) = (surface.take(), window.take()) {
            let capabilities = surface.get_capabilities(&adapter);

            Some(WindowData {
                window,
                surface,
                capabilities,
            })
        } else {
            None
        };

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        Ok(Context {
            adapter,
            device,
            queue,
            window_data,
        })
    }

    pub fn is_headless(&self) -> bool {
        self.window_data.is_none()
    }

    pub fn window(&self) -> Option<Arc<Window>> {
        self.window_data.as_ref().map(|d| d.window.clone())
    }

    pub fn surface(&self) -> Option<&Surface> {
        self.window_data.as_ref().map(|d| &d.surface)
    }

    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    pub fn device(&self) -> Arc<Device> {
        Arc::clone(&self.device)
    }

    pub fn queue(&self) -> Arc<Queue> {
        Arc::clone(&self.queue)
    }

    pub fn capabilities(&self) -> Option<&SurfaceCapabilities> {
        self.window_data.as_ref().map(|d| &d.capabilities)
    }

    pub fn formats(&self) -> Option<&[TextureFormat]> {
        self.capabilities().map(|cap| cap.formats.as_slice())
    }

    pub fn view_format(&self) -> Option<TextureFormat> {
        const PREFERRED: [TextureFormat; 2] = [
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8Unorm,
        ];
        if let Some(formats) = self.formats() {
            for tex in PREFERRED {
                if formats.contains(&tex) {
                    // always prefer non srgb swapchain
                    return Some(tex);
                }
            }
            // even though Bgra8Unorm is expected to exist
            // just choose the first item as a back-up
            formats.first().copied()
        } else {
            None
        }
    }
}
