macro_rules! define_handle {
    ($name:ident) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        pub struct $name(pub u32);

        impl $name {
            const DOMAIN_SHIFT: u32 = 31;
            const INDEX_MASK: u32 = (1 << 31) - 1;

            pub fn new(domain: Domain, index: usize) -> Self {
                let raw = ((domain as u32) << Self::DOMAIN_SHIFT) | (index as u32 & Self::INDEX_MASK);
                Self(raw)
            }

            pub fn domain(&self) -> Domain {
                match self.0 >> Self::DOMAIN_SHIFT{
                    0 => Domain::FrameLocal,
                    1 => Domain::Accumulator,
                    _ => panic!("Invalid handle tried to access resource registry: \n\tReason: invalid domain"),
                }
            }

            pub fn index(&self) -> usize {
                (self.0 & Self::INDEX_MASK) as usize
            }
        }
    };
}

define_handle!(TextureHandle);
define_handle!(BufferHandle);

#[repr(u8)]
pub enum Domain {
    FrameLocal = 0,
    Accumulator = 1,
}

pub struct TextureDescription {}

pub struct PhysicalTexture {
    pub inner: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub size: wgpu::Extent3d,
}

pub struct BufferDescription {}

pub struct PhysicalBuffer {
    pub inner: wgpu::Buffer,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

pub struct ResourceRegistry {
    device: wgpu::Device,
    // TEXTURES:
    frame_local_textures: Vec<PhysicalTexture>,
    accumulator_textures: Vec<PhysicalTexture>,
    // BUFFERS:
    frame_local_buffers: Vec<PhysicalBuffer>,
    accumulator_buffers: Vec<PhysicalBuffer>,
}

impl ResourceRegistry {
    pub fn new(device: &wgpu::Device) -> Self {
        let device = device.clone();
        // TEXTURES:
        let frame_local_textures = Vec::<PhysicalTexture>::new();
        let accumulator_textures = Vec::<PhysicalTexture>::new();
        // BUFFERS:
        let frame_local_buffers = Vec::<PhysicalBuffer>::new();
        let accumulator_buffers = Vec::<PhysicalBuffer>::new();

        Self {
            device,
            frame_local_textures,
            accumulator_textures,
            frame_local_buffers,
            accumulator_buffers,
        }
    }
}

// pub struct ResourceRegistry {
//     device: wgpu::Device,
//     // TEXTURES:
//     persistent_textures: Vec<PhysicalTexture>,
//     transient_textures: Vec<PhysicalTexture>,
//     history_textures: Vec<PhysicalTexture>,
//     // BUFFERS:
//     persistent_buffers: Vec<PhysicalBuffer>,
//     transient_buffers: Vec<PhysicalBuffer>,
//     history_buffers: Vec<PhysicalBuffer>,
// }

// impl ResourceRegistry {
//     pub fn new(device: wgpu::Device) -> Self {
//         let device = device.clone();
//         let persistent_textures =  Vec::<PhysicalTexture>::new();
//         let transient_textures = Vec::<PhysicalTexture>::new();
//         let history_textures = Vec::<PhysicalTexture>::new();
//         let persistent_buffers = Vec::<PhysicalBuffer>::new();
//         let transient_buffers = Vec::<PhysicalBuffer>::new();
//         let history_buffers = Vec::<PhysicalBuffer>::new();

//         Self {
//             device,
//             persistent_textures,
//             transient_textures,
//             history_textures,
//             persistent_buffers,
//             transient_buffers,
//             history_buffers
//         }
//     }

//     pub fn register_texture(&mut self, domain: Domain, description: &TextureDescription) -> TextureHandle {
//         let inner = self.device.create_texture(&wgpu::TextureDescriptor {
//             label: todo!(),
//             size: todo!(),
//             mip_level_count: todo!(),
//             sample_count: todo!(),
//             dimension: todo!(),
//             format: todo!(),
//             usage: todo!(),
//             view_formats: todo!(),
//         });
//         let view = inner.create_view(&wgpu::TextureViewDescriptor {
//             label: todo!(),
//             format: todo!(),
//             dimension: todo!(),
//             usage: todo!(),
//             aspect: todo!(),
//             base_mip_level: todo!(),
//             mip_level_count: todo!(),
//             base_array_layer: todo!(),
//             array_layer_count: todo!(),
//         });
//         let format = todo!();
//         let size = todo!();

//         let physical_texture = PhysicalTexture { inner, view, format, size };
//         let index;

//         match domain {
//             Domain::Persistent => {
//                 let index = self.persistent_textures.len();
//                 self.persistent_textures.push(physical_texture);
//             }
//             Domain::Transient => {
//                 let index = self.transient_textures.len();
//                 self.transient_textures.push(physical_texture);
//             },
//             Domain::History => {
//                 let index = self.history_textures.len();
//                 self.history_textures.push(physical_texture);
//             },
//         }

//         TextureHandle::new(domain, index)
//     }
// }
