use naga::{Scalar, ScalarKind, VectorSize};
use wgpu::naga;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexInputTarget<'a> {
    Struct(&'a str),
    Parameter(&'a str),
}

pub enum ShaderType<'a> {
    #[allow(unused)]
    Compute,
    Render {
        vertex_input_targets: Option<&'a [(VertexInputTarget<'a>, wgpu::VertexStepMode)]>,
    },
}

pub enum EntryPoints {
    Compute,
    Render {
        vertex_ep: String,
        fragment_ep: Option<String>,
    },
}

pub struct VertexBufferLayoutBuilder {
    pub stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayoutBuilder {
    pub fn create_buffer_layout(&mut self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.stride,
            step_mode: self.step_mode,
            attributes: &self.attributes,
        }
    }
}

pub struct ShaderReflection {
    pub entry_points: EntryPoints,
    pub bind_group_layouts: Vec<Option<wgpu::BindGroupLayout>>,
}

impl ShaderReflection {
    pub fn reflect_module(
        device: &wgpu::Device,
        module: &naga::Module,
        shader_name: &str,
        shader_type: ShaderType,
        vertex_buffer_builders: &mut Vec<Option<VertexBufferLayoutBuilder>>,
    ) -> anyhow::Result<Self> {
        let mut entry_points = match shader_type {
            ShaderType::Compute => EntryPoints::Compute,
            ShaderType::Render { .. } => {
                let vertex_ep: String = String::default();
                let fragment_ep: Option<String> = None;
                EntryPoints::Render {
                    vertex_ep,
                    fragment_ep,
                }
            }
        };
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        let _ = validator
            .validate(module)
            .expect("Failed to validate shader");

        // Generate bind groups based on the parse of the shader
        let mut groups = Vec::new();
        for (var_handle, var) in module.global_variables.iter() {
            if let Some(ref binding) = var.binding {
                let group_idx = binding.group as usize;
                while groups.len() <= group_idx {
                    groups.push(Vec::new());
                }
                groups[group_idx].push((var_handle, var));
            }
        }

        // Determine the visibility for each variable in the shader
        let mut variable_visibility = std::collections::HashMap::new();
        for ep in &module.entry_points {
            // Determine visibility
            let stage_flag = match ep.stage {
                naga::ShaderStage::Vertex => wgpu::ShaderStages::VERTEX,
                naga::ShaderStage::Fragment => wgpu::ShaderStages::FRAGMENT,
                naga::ShaderStage::Compute => wgpu::ShaderStages::COMPUTE,
                naga::ShaderStage::Mesh => wgpu::ShaderStages::MESH,
                naga::ShaderStage::Task => wgpu::ShaderStages::TASK,
                naga::ShaderStage::RayGeneration => wgpu::ShaderStages::RAY_GENERATION,
                naga::ShaderStage::Miss => wgpu::ShaderStages::MISS,
                naga::ShaderStage::AnyHit => wgpu::ShaderStages::ANY_HIT,
                naga::ShaderStage::ClosestHit => wgpu::ShaderStages::CLOSEST_HIT,
            };
            for (_, expr) in ep.function.expressions.iter() {
                if let naga::Expression::GlobalVariable(var_handle) = *expr {
                    variable_visibility
                        .entry(var_handle)
                        .or_insert(wgpu::ShaderStages::NONE)
                        .insert(stage_flag);
                }
            }

            // Determine vertex attributes
            match shader_type {
                ShaderType::Render {
                    vertex_input_targets,
                } => {
                    match stage_flag {
                        wgpu::ShaderStages::VERTEX => {
                            if let EntryPoints::Render {
                                ref mut vertex_ep, ..
                            } = entry_points
                            {
                                *vertex_ep = ep.name.to_owned();
                            }
                            for arg in &ep.function.arguments {
                                let arg_name = arg.name.clone().expect(
                                    "Couldn't find funciton name, this is likely due to using a 
                                    non WGSL shader language, which is not yet supported",
                                );
                                let arg_ty = &module.types[arg.ty].inner;
                                let binding = arg.binding.clone();
                                let mut location = match binding {
                                    Some(naga::Binding::Location {
                                        location: binding_loc,
                                        ..
                                    }) => binding_loc,
                                    _ => {
                                        continue;
                                    }
                                };

                                let targets = match vertex_input_targets {
                                    Some(input_targets) => input_targets,
                                    None => anyhow::bail!("Vertex shader input that requires a buffer layout detected,
                                                            but no vertex shader input targets were provided.")
                                };

                                // ensure user provided mappings for argument
                                let step_mode = match targets.iter().find_map(|(target, mode)| {
                                    let is_struct =
                                        matches!(arg_ty, naga::TypeInner::Struct { .. });

                                    let matches = match *target {
                                        VertexInputTarget::Struct(name) => {
                                            is_struct && name == arg_name
                                        }
                                        VertexInputTarget::Parameter(name) => {
                                            !is_struct && name == arg_name
                                        }
                                    };
                                    if matches { Some(mode) } else { None }
                                }) {
                                    Some(mode) => *mode,
                                    None => anyhow::bail!(
                                        "Shader refleciton failed: ensure that all vertex input variables are mapped to step modes"
                                    ),
                                };

                                // Create vertex attributes based on parameter type
                                let mut offset: wgpu::BufferAddress = 0;
                                let mut vertex_attributes = Vec::new();
                                Self::populate_vertex_attributes(
                                    module,
                                    arg_ty,
                                    binding,
                                    &mut offset,
                                    &mut location,
                                    &mut vertex_attributes,
                                )?;

                                vertex_buffer_builders.push( match vertex_attributes.is_empty() {
                                    true => None,
                                    false => Some(VertexBufferLayoutBuilder {
                                        stride: offset,
                                        step_mode,
                                        attributes: vertex_attributes,
                                    }),
                                });
                            }
                        }
                        wgpu::ShaderStages::FRAGMENT => {
                            if let EntryPoints::Render {
                                ref mut fragment_ep,
                                ..
                            } = entry_points
                            {
                                *fragment_ep = Some(ep.name.to_owned());
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Generate all bind group layouts
        let mut bind_group_layouts = Vec::new();
        for (group_idx, group) in groups.into_iter().enumerate() {
            let mut bind_group_entries = Vec::new();
            for (var_handle, var) in &group {
                // Binding index
                let binding = var
                    .binding
                    .as_ref()
                    .expect("Could not find binding index for a global variable")
                    .binding;

                // Binding visibility
                let visibility = variable_visibility
                    .get(&var_handle)
                    .expect("variable not found in variable visibility lookup table");

                // Binding type and binding size
                let (ty, count) = Self::get_buffer_binding_type(var, module, device)?;

                bind_group_entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: *visibility,
                    ty,
                    count,
                });
            }

            let label = format!("{} Bind group {}", shader_name, group_idx);
            bind_group_layouts.push(match group.is_empty() {
                false => Some(
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &bind_group_entries[..],
                        label: Some(&label),
                    }),
                ),
                true => None,
            });
        }

        Ok(Self {
            entry_points,
            bind_group_layouts,
        })
    }

    fn populate_vertex_attributes(
        module: &naga::Module,
        ty: &naga::TypeInner,
        binding: Option<wgpu::naga::Binding>,
        offset: &mut wgpu::BufferAddress,
        location: &mut u32,
        vertex_attributes: &mut Vec<wgpu::VertexAttribute>,
    ) -> anyhow::Result<()> {
        match binding {
            Some(naga::Binding::Location {
                location: binding_loc,
                ..
            }) => {
                *location = binding_loc;
            }
            Some(naga::Binding::BuiltIn(_)) => {
                return Ok(());
            }
            None => {}
        }

        match ty {
            naga::TypeInner::Struct { members, span } => {
                for member in members {
                    let mut member_offset = member.offset as wgpu::BufferAddress + *offset;

                    let member_ty = &module.types[member.ty].inner;
                    let member_binding = member.binding.clone();

                    Self::populate_vertex_attributes(
                        module,
                        member_ty,
                        member_binding,
                        &mut member_offset,
                        location,
                        vertex_attributes,
                    )?;
                }
                *offset += *span as wgpu::BufferAddress;
            }
            naga::TypeInner::Matrix {
                columns,
                rows,
                scalar,
            } => {
                let format = Self::inner_type_to_vertex_fmt(&naga::TypeInner::Vector {
                    size: *rows,
                    scalar: *scalar,
                })?;
                let col_stride = match Self::vertex_format_size(format)? {
                    12 => 16,
                    24 => 32,
                    other => other,
                };

                for _ in 0..(*columns as u32) {
                    vertex_attributes.push(wgpu::VertexAttribute {
                        format: format,
                        offset: *offset,
                        shader_location: *location,
                    });
                    *offset += col_stride;
                    *location += 1;
                }
            }
            naga::TypeInner::Scalar(_) | naga::TypeInner::Vector { .. } => {
                let format = Self::inner_type_to_vertex_fmt(&ty)?;
                let width = match Self::vertex_format_size(format)? {
                    12 => 16,
                    24 => 32,
                    other => other,
                };
                vertex_attributes.push(wgpu::VertexAttribute {
                    offset: *offset,
                    shader_location: *location,
                    format: format,
                });
                *offset += width;
                *location += 1;
            }
            naga::TypeInner::Array { base, size, stride } => {
                let len = match size {
                    naga::ArraySize::Constant(non_zero) => non_zero.get(),
                    _ => {
                        anyhow::bail!("Dynamic arrays cannot be used as vertex shader parameters");
                    }
                };
                let element_type = &module.types[*base].inner;

                for _ in 0..len {
                    let mut element_offset = *offset; // This is only for child functions to track their relative offsets, main offset is just affected by stride
                    Self::populate_vertex_attributes(
                        module,
                        element_type,
                        None,
                        &mut element_offset,
                        location,
                        vertex_attributes,
                    )?;
                    *offset += *stride as wgpu::BufferAddress;
                }
            }
            _ => {
                panic!("WGSL vertex shader input of type '{:?}' is unsupported", ty);
            }
        }
        Ok(())
    }

    // THE FOLLOWING FUNCTION WAS GENERATED WITH THE USE OF AI
    // I hope the reason is pretty clear
    fn inner_type_to_vertex_fmt(
        type_inner: &naga::TypeInner,
    ) -> anyhow::Result<wgpu::VertexFormat> {
        let format = match *type_inner {
            naga::TypeInner::Scalar(Scalar { kind, width }) => match (kind, width) {
                (ScalarKind::Float, 8) => wgpu::VertexFormat::Float64,
                (ScalarKind::Float, 4) => wgpu::VertexFormat::Float32,
                (ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32,
                (ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32,
                (ScalarKind::Float, 2) => wgpu::VertexFormat::Float16,
                (ScalarKind::Sint, 2) => wgpu::VertexFormat::Sint16,
                (ScalarKind::Uint, 2) => wgpu::VertexFormat::Uint16,
                (ScalarKind::Sint, 1) => wgpu::VertexFormat::Sint8,
                (ScalarKind::Uint, 1) => wgpu::VertexFormat::Uint8,
                _ => anyhow::bail!(
                    "Unsupported scalar type: kind={:?}, width={:?}",
                    kind,
                    width
                ),
            },
            naga::TypeInner::Vector {
                size,
                scalar: Scalar { kind, width },
            } => match (size, kind, width) {
                // 32-bit Vectors
                (VectorSize::Bi, ScalarKind::Float, 4) => wgpu::VertexFormat::Float32x2,
                (VectorSize::Tri, ScalarKind::Float, 4) => wgpu::VertexFormat::Float32x3,
                (VectorSize::Quad, ScalarKind::Float, 4) => wgpu::VertexFormat::Float32x4,

                (VectorSize::Bi, ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32x2,
                (VectorSize::Tri, ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32x3,
                (VectorSize::Quad, ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32x4,

                (VectorSize::Bi, ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32x2,
                (VectorSize::Tri, ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32x3,
                (VectorSize::Quad, ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32x4,

                // 16-bit Vectors (No Tri/x3 format in WebGPU)
                (VectorSize::Bi, ScalarKind::Float, 2) => wgpu::VertexFormat::Float16x2,
                (VectorSize::Quad, ScalarKind::Float, 2) => wgpu::VertexFormat::Float16x4,
                (VectorSize::Bi, ScalarKind::Sint, 2) => wgpu::VertexFormat::Sint16x2,
                (VectorSize::Quad, ScalarKind::Sint, 2) => wgpu::VertexFormat::Sint16x4,
                (VectorSize::Bi, ScalarKind::Uint, 2) => wgpu::VertexFormat::Uint16x2,
                (VectorSize::Quad, ScalarKind::Uint, 2) => wgpu::VertexFormat::Uint16x4,

                // 8-bit Vectors (No Tri/x3 format in WebGPU)
                (VectorSize::Bi, ScalarKind::Sint, 1) => wgpu::VertexFormat::Sint8x2,
                (VectorSize::Quad, ScalarKind::Sint, 1) => wgpu::VertexFormat::Sint8x4,
                (VectorSize::Bi, ScalarKind::Uint, 1) => wgpu::VertexFormat::Uint8x2,
                (VectorSize::Quad, ScalarKind::Uint, 1) => wgpu::VertexFormat::Uint8x4,

                _ => anyhow::bail!(
                    "Unsupported vector type: size={:?}, kind={:?}, width={:?}",
                    size,
                    kind,
                    width
                ),
            },
            ref unsupported => {
                anyhow::bail!("Unsupported vertex input type variant: {:?}", unsupported);
            }
        };

        Ok(format)
    }

    // THE FOLLOWING FUNCTION WAS GENERATED WITH THE USE OF AI
    // I hope the reason is pretty clear
    pub fn vertex_format_size(format: wgpu::VertexFormat) -> anyhow::Result<u64> {
        match format {
            // 8-bit & Packed / Normalized formats
            wgpu::VertexFormat::Uint8
            | wgpu::VertexFormat::Sint8
            | wgpu::VertexFormat::Unorm8
            | wgpu::VertexFormat::Snorm8 => Ok(1),

            wgpu::VertexFormat::Uint8x2
            | wgpu::VertexFormat::Sint8x2
            | wgpu::VertexFormat::Unorm8x2
            | wgpu::VertexFormat::Snorm8x2 => Ok(2),

            wgpu::VertexFormat::Uint8x4
            | wgpu::VertexFormat::Sint8x4
            | wgpu::VertexFormat::Unorm8x4
            | wgpu::VertexFormat::Snorm8x4
            | wgpu::VertexFormat::Unorm8x4Bgra => Ok(4),

            // 16-bit formats
            wgpu::VertexFormat::Uint16
            | wgpu::VertexFormat::Sint16
            | wgpu::VertexFormat::Float16
            | wgpu::VertexFormat::Unorm16
            | wgpu::VertexFormat::Snorm16 => Ok(2),

            wgpu::VertexFormat::Uint16x2
            | wgpu::VertexFormat::Sint16x2
            | wgpu::VertexFormat::Unorm16x2
            | wgpu::VertexFormat::Snorm16x2
            | wgpu::VertexFormat::Float16x2 => Ok(4),

            wgpu::VertexFormat::Uint16x4
            | wgpu::VertexFormat::Sint16x4
            | wgpu::VertexFormat::Unorm16x4
            | wgpu::VertexFormat::Snorm16x4
            | wgpu::VertexFormat::Float16x4 => Ok(8),

            // 32-bit formats
            wgpu::VertexFormat::Uint32
            | wgpu::VertexFormat::Sint32
            | wgpu::VertexFormat::Float32
            | wgpu::VertexFormat::Unorm10_10_10_2 => Ok(4), // 32-bit packed structure

            wgpu::VertexFormat::Uint32x2
            | wgpu::VertexFormat::Sint32x2
            | wgpu::VertexFormat::Float32x2 => Ok(8),

            wgpu::VertexFormat::Uint32x3
            | wgpu::VertexFormat::Sint32x3
            | wgpu::VertexFormat::Float32x3 => Ok(12),

            wgpu::VertexFormat::Uint32x4
            | wgpu::VertexFormat::Sint32x4
            | wgpu::VertexFormat::Float32x4 => Ok(16),

            // 64-bit formats
            wgpu::VertexFormat::Float64 => Ok(8),
            wgpu::VertexFormat::Float64x2 => Ok(16),
            wgpu::VertexFormat::Float64x3 => Ok(24),
            wgpu::VertexFormat::Float64x4 => Ok(32),

            // Catch all for updates to wgpu
            #[allow(unreachable_patterns)]
            _ => {
                anyhow::bail!(
                    "Vertex format size not known, this can happen if wgpu implements a new vertex format"
                );
            }
        }
    }

    fn get_buffer_binding_type(
        var: &naga::GlobalVariable,
        module: &naga::Module,
        device: &wgpu::Device,
    ) -> anyhow::Result<(wgpu::BindingType, Option<std::num::NonZero<u32>>)> {
        let ty = &module.types[var.ty];
        match &ty.inner {
            naga::TypeInner::BindingArray { base, size } => {
                let limits = device.limits();
                let (base_type, upper_size_bound) = match &module.types[*base].inner {
                    naga::TypeInner::Image { .. } => (
                        wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        limits.max_binding_array_elements_per_shader_stage,
                    ),
                    naga::TypeInner::Sampler { .. } => (
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        limits.max_binding_array_sampler_elements_per_shader_stage,
                    ),
                    naga::TypeInner::AccelerationStructure { vertex_return } => (
                        wgpu::BindingType::AccelerationStructure {
                            vertex_return: *vertex_return,
                        },
                        limits.max_binding_array_acceleration_structure_elements_per_shader_stage,
                    ),
                    other => {
                        anyhow::bail!("Binding array inner type was invalid type: {:?}", other);
                    }
                };
                let inner_size = match size {
                    naga::ArraySize::Constant(non_zero) => {
                        if non_zero.get() < upper_size_bound {
                            non_zero
                        } else {
                            anyhow::bail!("Invalid dynamic size for binding array");
                        }
                    }
                    _ => anyhow::bail!("Dynamic binding arrays are not yet supported"),
                };
                Ok((base_type, Some(*inner_size)))
            }
            naga::TypeInner::Struct { .. } | naga::TypeInner::Array { .. } => {
                let buffer_ty = match var.space {
                    naga::AddressSpace::Uniform => wgpu::BufferBindingType::Uniform,
                    naga::AddressSpace::Storage { access } => {
                        let read_only = !access.contains(naga::StorageAccess::STORE);
                        wgpu::BufferBindingType::Storage { read_only }
                    }
                    _ => panic!("Invalid buffer address space"),
                };
                Ok((
                    wgpu::BindingType::Buffer {
                        ty: buffer_ty,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    None,
                ))
            }
            naga::TypeInner::Image { .. } => Ok((
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                None,
            )),
            naga::TypeInner::Sampler { .. } => Ok((
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                None,
            )),
            naga::TypeInner::AccelerationStructure { vertex_return } => Ok((
                wgpu::BindingType::AccelerationStructure {
                    vertex_return: *vertex_return,
                },
                None,
            )),
            other => {
                anyhow::bail!(
                    "Failed to find appropriate wgpu binding type for naga type: {:?} during shader refleciton for variable: {:?}",
                    other,
                    var
                );
            }
        }
    }
}
