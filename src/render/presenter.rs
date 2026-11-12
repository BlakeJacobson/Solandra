use crate::render::pipeline::RenderPipelineBuilder;

use super::pipeline;

pub struct Presenter {
    pipeline: pipeline::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl Presenter {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let pipeline_builder = RenderPipelineBuilder::new(shader_path);
        
    }
}