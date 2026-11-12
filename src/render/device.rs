use anyhow::Ok;

pub struct RenderSurface {
    pub surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    is_configured: bool,
}

impl RenderSurface {
    pub async fn new(
        instance: &wgpu::Instance,
        window: std::sync::Arc<winit::window::Window>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let mut config = surface
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

        if config.height == 0 || config.width == 0 {
            config.height = 1;
            config.width = 1;
        }

        Ok(Self { surface, config, is_configured: false })
    }

    fn apply_config(&mut self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
        self.is_configured = true;
    }

    pub fn ensure_configured(&mut self, device: &wgpu::Device) {
        if self.is_configured == true {
            return;
        }

        self.apply_config(device);
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) {
        if size.height == 0 || size.height == 0 {
            return;
        }

        if self.config.height != size.height || self.config.width != size.width {
            self.config.height = size.height;
            self.config.width = size.width;
            self.apply_config(device);
        }
    }

    pub fn recover_lost(&mut self, device: &wgpu::Device) {
        self.apply_config(device);
    }
}

pub struct RenderConfig {
    pub instance_descriptor: wgpu::InstanceDescriptor,
    pub adapter_options: wgpu::RequestAdapterOptions<'static, 'static>,
    pub device_descriptor: wgpu::DeviceDescriptor<'static>,
}

impl Default for RenderConfig {
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
        device_descriptor: &wgpu::DeviceDescriptor<'_>,
        adapter_options: wgpu::RequestAdapterOptions<'_, '_>,
    ) -> anyhow::Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&render_surface.surface),
                ..adapter_options
            })
            .await?;

        let (device, queue) = adapter.request_device(device_descriptor).await?;

        Ok(Self { device, queue })
    }
}
