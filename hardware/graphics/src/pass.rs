use std::ops::{
    Deref,
    DerefMut,
};

pub enum RenderPass<'a> {
    Wgpu(wgpu::RenderPass<'a>),
    Profiled(wgpu_profiler::OwningScope<'a, wgpu::RenderPass<'a>>),
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

pub enum ComputePass<'a> {
    Wgpu(wgpu::ComputePass<'a>),
    Profiled(wgpu_profiler::OwningScope<'a, wgpu::ComputePass<'a>>),
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
