#![allow(unused)]
// File automatically generated by build.rs.
// Changes made to this file will not be saved.
use graphics::wgpu;

pub mod bind_groups {
    use graphics::wgpu;

    #[derive(Debug)]
    pub struct BindGroup0(wgpu::BindGroup);
    #[derive(Debug)]
    pub struct BindGroupLayout0<'a> {
        pub color_texture: &'a wgpu::TextureView,
        pub color_sampler: &'a wgpu::Sampler,
    }
    const LAYOUT_DESCRIPTOR0: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {
                        filterable: true,
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    };
    impl BindGroup0 {
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&LAYOUT_DESCRIPTOR0)
        }
        pub fn from_bindings(device: &wgpu::Device, bindings: BindGroupLayout0) -> Self {
            let bind_group_layout = device.create_bind_group_layout(&LAYOUT_DESCRIPTOR0);
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(
                                    bindings.color_texture,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(
                                    bindings.color_sampler,
                                ),
                            },
                        ],
                        label: None,
                    },
                );
            Self(bind_group)
        }
        pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
            render_pass.set_bind_group(0, &self.0, &[]);
        }
    }
    #[derive(Debug)]
    pub struct BindGroups<'a> {
        pub bind_group0: &'a BindGroup0,
    }
    pub fn set_bind_groups<'a>(
        pass: &mut wgpu::RenderPass<'a>,
        bind_groups: BindGroups<'a>,
    ) {
        bind_groups.bind_group0.set(pass);
    }
}
pub const ENTRY_VERT: &str = "vert";
pub const ENTRY_FRAG: &str = "frag";
#[derive(Debug)]
pub struct VertexEntry<const N: usize> {
    entry_point: &'static str,
    buffers: [wgpu::VertexBufferLayout<'static>; N],
}
pub fn vertex_state<'a, const N: usize>(
    module: &'a wgpu::ShaderModule,
    entry: &'a VertexEntry<N>,
) -> wgpu::VertexState<'a> {
    wgpu::VertexState {
        module,
        entry_point: entry.entry_point,
        buffers: &entry.buffers,
    }
}
pub fn vert_entry() -> VertexEntry<0> {
    VertexEntry {
        entry_point: ENTRY_VERT,
        buffers: [],
    }
}
pub fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let source = std::borrow::Cow::Borrowed(include_str!("shader.wgsl"));
    device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source),
        })
}
pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device
        .create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &bind_groups::BindGroup0::get_bind_group_layout(device),
                ],
                push_constant_ranges: &[],
            },
        )
}