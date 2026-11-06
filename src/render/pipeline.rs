use std::num::NonZero;

use anyhow::Ok;
use wgpu::naga::self;

use crate::render::reflection::*;

bitflags::bitflags! {
    pub struct RenderPassFlags: u32 {
        const OPAQUE_MAIN = 1 << 0;

    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    Alpha,
    Additive,
}

pub enum DepthConfig {
    Disabled,
    Enabled {
        write: bool,
        compare: wgpu::CompareFunction,
    },
}

pub struct GraphicsPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub layout: wgpu::PipelineLayout,
    pub render_pass_flags: RenderPassFlags,
}

pub struct GraphicsPipelineBuilder<'a> {
    // Customization
    label: &'a str,

    // Shaders
    shader_path: &'a str,

    // Immediates
    immediate_size: u32,

    // Rasterization
    topology: wgpu::PrimitiveTopology,
    strip_index_format: Option<wgpu::IndexFormat>,
    cull_mode: Option<wgpu::Face>,
    front_face: wgpu::FrontFace,
    polygon_mode: wgpu::PolygonMode,
    unclipped_depth: bool,
    conservative: bool,

    // Depth
    depth_config: DepthConfig,

    // MSAA
    multisample: wgpu::MultisampleState,

    // Multiview
    multiview_mask: Option<NonZero<u32>>,

    // Depth stencil
    depth_stencil: Option<wgpu::DepthStencilState>,

    // Targets
    vertex_input_targets: Option<&'a [(VertexInputTarget<'a>, wgpu::VertexStepMode)]>,
    fragment_output_targets: Vec<Option<wgpu::ColorTargetState>>,

    // Pipeline Cache
    pipeline_cache: Option<&'a wgpu::PipelineCache>,

    // Metadata
    render_pass_flags: RenderPassFlags,
}

impl<'a> GraphicsPipelineBuilder<'a> {
    pub fn new(shader_path: &'a str) -> Self {
        Self {
            label: shader_path,
            shader_path,
            immediate_size: 0,
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Ccw,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
            depth_config: DepthConfig::Enabled {
                write: true,
                compare: wgpu::CompareFunction::Less,
            },
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            depth_stencil: None,
            fragment_output_targets: Vec::new(),
            vertex_input_targets: None,
            pipeline_cache: None,
            render_pass_flags: RenderPassFlags::empty(),
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    pub fn with_immediate_size(mut self, size: u32) -> Self {
        self.immediate_size = size;
        self
    }

    pub fn with_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    pub fn with_strip_index_format(mut self, strip_index_format: wgpu::IndexFormat) -> Self {
        self.strip_index_format = Some(strip_index_format);
        self
    }

    pub fn with_cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    pub fn with_front_face(mut self, front_face: wgpu::FrontFace) -> Self {
        self.front_face = front_face;
        self
    }

    pub fn with_polygon_mode(mut self, polygon_mode: wgpu::PolygonMode) -> Self {
        self.polygon_mode = polygon_mode;
        self
    }

    pub fn enable_unclipped_depth(mut self) -> Self {
        self.unclipped_depth = true;
        self
    }

    pub fn enable_conservative_rasterization(mut self) -> Self {
        self.conservative = true;
        self
    }

    pub fn with_depth_config(mut self, depth_config: DepthConfig) -> Self {
        self.depth_config = depth_config;
        self
    }

    pub fn with_sample_count(mut self, count: u32) -> Self {
        self.multisample.count = count;
        self
    }

    pub fn with_multisample(mut self, multisample: wgpu::MultisampleState) -> Self {
        self.multisample = multisample;
        self
    }

    pub fn with_multiview_mask(mut self, mask: Option<NonZero<u32>>) -> Self {
        self.multiview_mask = mask;
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    pub fn add_color_target(mut self, format: wgpu::TextureFormat) -> Self {
        self.fragment_output_targets
            .push(Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }));
        self
    }

    pub fn add_empty_color_target(mut self) -> Self {
        self.fragment_output_targets.push(None);
        self
    }

    pub fn add_custom_color_target(
        mut self,
        format: wgpu::TextureFormat,
        blend: Option<wgpu::BlendState>,
        write_mask: wgpu::ColorWrites,
    ) -> Self {
        self.fragment_output_targets
            .push(Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask,
            }));
        self
    }

    pub fn with_vertex_input_targets(
        mut self,
        vertex_input_targets: &'a [(VertexInputTarget<'a>, wgpu::VertexStepMode)],
    ) -> Self {
        self.vertex_input_targets = Some(vertex_input_targets);
        self
    }

    pub fn with_pipeline_cache(mut self, cache: &'a wgpu::PipelineCache) -> Self {
        self.pipeline_cache = Some(cache);
        self
    }

    pub fn with_render_pass_flags(mut self, render_pass_flags: RenderPassFlags) -> Self {
        self.render_pass_flags = render_pass_flags;
        self
    }

    pub fn build(self, device: &wgpu::Device) -> anyhow::Result<GraphicsPipeline> {
        // Parse the shader
        let source = std::fs::read_to_string(self.shader_path).expect("Failed to read shader file");
        let module = naga::front::wgsl::parse_str(&source).expect("Failed to parse shader file");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(self.shader_path),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let mut vertex_buffer_layout_builders: Vec<Option<VertexBufferLayoutBuilder>> = Vec::new();
        let reflection = ShaderReflection::reflect_module(
            device,
            &module,
            "",
            ShaderType::Render {
                vertex_input_targets: self.vertex_input_targets,
            },
            &mut vertex_buffer_layout_builders,
        )?;
        let vertex_buffer_layouts: Vec<Option<wgpu::VertexBufferLayout>> = vertex_buffer_layout_builders
            .iter_mut()
            .map(|slot| match slot{
                Some(layout) => Some(layout.create_buffer_layout()),
                None => None,
            })
            .collect();
        let layout_label = format!("{} layout", self.label);
        let bind_group_layouts = reflection.bind_group_layouts;
        let bind_group_refs: Vec<Option<&wgpu::BindGroupLayout>> = bind_group_layouts
            .iter()
            .map(|layout_opt| layout_opt.as_ref())
            .collect();
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&layout_label),
                bind_group_layouts: &bind_group_refs,
                immediate_size: self.immediate_size,
            });

        let (vert_ep, frag_ep) = match reflection.entry_points {
            EntryPoints::Compute => {
                panic!("Tried to construct a render pipeline with a Compute shader type")
            } // Shouldn't be possible if this funciton doesn't change
            EntryPoints::Render {
                vertex_ep,
                fragment_ep,
            } => (vertex_ep, fragment_ep),
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(self.label),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(&vert_ep),
                compilation_options: wgpu::PipelineCompilationOptions::default(), // Technically could be determined during shader reflection, TODO!!!
                buffers: &vertex_buffer_layouts[..],
            },
            fragment: frag_ep.as_deref().map(|entry | { 
                wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &self.fragment_output_targets,
                    compilation_options: wgpu::PipelineCompilationOptions::default(), // Same as with fragment shader
                }
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                strip_index_format: self.strip_index_format,
                front_face: self.front_face,
                cull_mode: self.cull_mode,
                unclipped_depth: self.unclipped_depth,
                polygon_mode: self.polygon_mode,
                conservative: self.conservative,
            },
            depth_stencil: self.depth_stencil,
            multisample: self.multisample,
            multiview_mask: self.multiview_mask,
            cache: self.pipeline_cache,
        });

        Ok(GraphicsPipeline {
            pipeline,
            layout: render_pipeline_layout,
            render_pass_flags: self.render_pass_flags,
        })
    }
}
