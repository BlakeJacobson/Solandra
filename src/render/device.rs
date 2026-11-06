use anyhow::Ok;

pub struct RenderSurface {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl RenderSurface {
    pub async fn new(
        instance: &wgpu::Instance,
        window: std::sync::Arc<winit::window::Window>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let config = surface
            .get_default_config(
                &instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        compatible_surface: Some(&surface),
                        ..Default::default()
                    })
                    .await
                    .expect("Failed to get the adapter during render surface init"),
                size.width,
                size.height,
            )
            .expect("Failed to get a proper surface configuration in render surface init.");

        Ok(Self { surface, config })
    }
}

pub struct RenderDeviceConfig {
    pub instance_descriptor: wgpu::InstanceDescriptor,
    pub adapter_options: wgpu::RequestAdapterOptions<'static, 'static>,
    pub device_descriptor: wgpu::DeviceDescriptor<'static>,
}

impl Default for RenderDeviceConfig {
    fn default() -> Self {
        Self {
            instance_descriptor: wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                flags: Default::default(),
                memory_budget_thresholds: Default::default(),
                backend_options: Default::default(),
                display: None,
            },
            adapter_options: wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            },
            device_descriptor: wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            },
        }
    }
}

pub struct RenderDevice {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl RenderDevice {
    pub async fn new(
        instance: &wgpu::Instance,
        render_surface: &RenderSurface,
        config: impl Into<Option<RenderDeviceConfig>>,
    ) -> anyhow::Result<Self> {
        let config = config.into().unwrap_or_default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&render_surface.surface),
                ..config.adapter_options
            })
            .await?;

        let (device, queue) = adapter.request_device(&config.device_descriptor).await?;

        Ok(Self { device, queue })
    }
}
