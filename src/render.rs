mod device;
mod pipeline;
mod resource;
mod presenter;

use std::sync::Arc;

use anyhow::Ok;
use device::*;
use pipeline::*;
use resource::*;
use winit::window::{self, Window};

pub trait Pass {
    fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        resource_reg: &ResourceRegistry,
        pipeline_reg: &PipelineRegistry,
    );
}

pub struct Renderer {
    window: Arc<Window>,
    render_device: RenderDevice,
    render_surface: RenderSurface,
    resource_reg: ResourceRegistry,
    pipeline_reg: PipelineRegistry,
    render_graph: RenderGraph,
    virtual_target: TextureHandle,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        render_config: Option<RenderConfig>,
    ) -> anyhow::Result<Self> {
        let render_config = render_config.unwrap_or_default();
        let RenderConfig {
            instance_descriptor,
            adapter_options,
            device_descriptor,
        } = render_config;

        // Create engine containers for wgpu device, queue, ...
        let instance = wgpu::Instance::new(instance_descriptor);
        let mut render_surface = RenderSurface::new(&instance, window.clone()).await?;
        let render_device = RenderDevice::new(
            &instance,
            &render_surface,
            &device_descriptor,
            adapter_options,
        )
        .await?;
        render_surface.recover_lost(&render_device.device);


        let resource_reg = ResourceRegistry::new(&render_device.device);
        let pipeline_reg = PipelineRegistry::new(&render_device.device);
        let render_graph = RenderGraph::new();

        Ok(Self {
            window: window.clone(),
            render_device,
            render_surface,
            resource_reg,
            pipeline_reg,
            render_graph,
        })
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let RenderDevice { ref device, ref queue} = self.render_device;
        self.window.request_redraw();

        self.render_surface.ensure_configured(&self.render_device.device);

        let output = match self.render_surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => return Ok(()),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.render_surface.resize(device, self.window.inner_size());
                return Ok(());
            },
            wgpu::CurrentSurfaceTexture::Lost => {
                self.render_surface.recover_lost(device);
                return Ok(());
            },
            wgpu::CurrentSurfaceTexture::Validation => panic!("Invalid surface texture."),
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Render encoder"),
        });

        self.render_graph.execute(&mut encoder, &self.resource_reg, &self.pipeline_reg, );


        Ok(())
    }
}

struct RenderGraph {
    //nodes_pool: HashMap<>
    // Precomputed vector of passes that describes a whole
    command_stream: Vec<Box<dyn Pass>>,
    cs_dirty: bool,
}

impl RenderGraph {
    pub fn new() -> Self {
        let command_stream = Vec::<Box<dyn Pass>>::new();

        Self { command_stream, cs_dirty:true }
    }

    pub fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        resource_reg: &resource::ResourceRegistry,
        pipeline_reg: &PipelineRegistry,
        final_output: &TextureHandle,
    ) {
        if self.cs_dirty {
            self.compute_command_stream();
            self.cs_dirty = false;
        }

        for pass in &mut self.command_stream {
            pass.as_mut().encode(encoder, resource_reg, pipeline_reg);
        }
    }

    fn compute_command_stream(&mut self) {
        todo!();
    }
}

// struct PassNode {
//     name: String,
//     in_buffers:
//     out_buffers:
//     in_textures:
//     out_textures:
// }
