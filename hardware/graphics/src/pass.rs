use std::ops::{
    Deref,
    DerefMut,
};

/// A Renderpass.
pub enum RenderPass<'a> {
    /// A plain RenderPass.
    Wgpu(wgpu::RenderPass<'a>),
    /// A profiled RenderPass.
    Profiled(profiler::gpu::OwningScope<'a, wgpu::RenderPass<'a>>),
}

impl<'a> Deref for RenderPass<'a> {
    type Target = wgpu::RenderPass<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            RenderPass::Wgpu(pass) => pass,
            RenderPass::Profiled(pass) => pass,
        }
    }
}

impl<'a> DerefMut for RenderPass<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RenderPass::Wgpu(pass) => pass,
            RenderPass::Profiled(pass) => pass,
        }
    }
}

/// A ComputePass.
pub enum ComputePass<'a> {
    /// A plain ComputePass.
    Wgpu(wgpu::ComputePass<'a>),
    /// A profiled ComputePass.
    Profiled(profiler::gpu::OwningScope<'a, wgpu::ComputePass<'a>>),
}

impl<'a> Deref for ComputePass<'a> {
    type Target = wgpu::ComputePass<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            ComputePass::Wgpu(pass) => pass,
            ComputePass::Profiled(pass) => pass,
        }
    }
}

impl<'a> DerefMut for ComputePass<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            ComputePass::Wgpu(pass) => pass,
            ComputePass::Profiled(pass) => pass,
        }
    }
}
