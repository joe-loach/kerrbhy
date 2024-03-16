use wgpu::{
    Buffer,
    BufferAddress,
    Device,
    Extent3d,
    ImageCopyBuffer,
    ImageCopyTexture,
    ImageSubresourceRange,
    RenderPassDescriptor,
    Texture,
};

use crate::{
    ComputePass,
    RenderPass,
};

pub enum Encoder<'a> {
    Wgpu(&'a mut wgpu::CommandEncoder),
    Profiled(wgpu_profiler::Scope<'a, wgpu::CommandEncoder>),
}

impl<'a> From<wgpu_profiler::Scope<'a, wgpu::CommandEncoder>> for Encoder<'a> {
    fn from(value: wgpu_profiler::Scope<'a, wgpu::CommandEncoder>) -> Self {
        Encoder::Profiled(value)
    }
}

impl<'a> From<&'a mut wgpu::CommandEncoder> for Encoder<'a> {
    fn from(value: &'a mut wgpu::CommandEncoder) -> Self {
        Encoder::Wgpu(value)
    }
}

impl<'a> Encoder<'a> {
    pub fn profiled(
        profiler: &'a wgpu_profiler::GpuProfiler,
        enc: &'a mut wgpu::CommandEncoder,
        label: &str,
        device: &Device,
    ) -> Self {
        Encoder::Profiled(profiler.scope(label, enc, device))
    }

    pub fn inner(&mut self) -> &mut wgpu::CommandEncoder {
        match self {
            Encoder::Wgpu(enc) => enc,
            Encoder::Profiled(enc) => enc,
        }
    }

    /// Begins recording of a render pass.
    ///
    /// This function returns a [`RenderPass`] object which records a single
    /// render pass.
    #[inline]
    pub fn begin_render_pass<'pass>(
        &'pass mut self,
        label: &str,
        device: &Device,
        desc: RenderPassDescriptor<'pass, '_>,
    ) -> RenderPass<'_> {
        match self {
            Encoder::Wgpu(enc) => RenderPass::Wgpu(enc.begin_render_pass(&desc)),
            Encoder::Profiled(enc) => {
                RenderPass::Profiled(enc.scoped_render_pass(label, device, desc))
            }
        }
    }

    /// Begins recording of a compute pass.
    ///
    /// This function returns a [`ComputePass`] object which records a single
    /// compute pass.
    #[inline]
    pub fn begin_compute_pass(&mut self, label: &str, device: &Device) -> ComputePass<'_> {
        match self {
            Encoder::Wgpu(enc) => ComputePass::Wgpu(enc.begin_compute_pass(&Default::default())),
            Encoder::Profiled(enc) => ComputePass::Profiled(enc.scoped_compute_pass(label, device)),
        }
    }

    /// Copy data from one buffer to another.
    ///
    /// # Panics
    ///
    /// - Buffer offsets or copy size not a multiple of
    ///   [`COPY_BUFFER_ALIGNMENT`].
    /// - Copy would overrun buffer.
    /// - Copy within the same buffer.
    #[inline]
    pub fn copy_buffer_to_buffer(
        &mut self,
        source: &Buffer,
        source_offset: BufferAddress,
        destination: &Buffer,
        destination_offset: BufferAddress,
        copy_size: BufferAddress,
    ) {
        match self {
            Encoder::Wgpu(enc) => enc.copy_buffer_to_buffer(
                source,
                source_offset,
                destination,
                destination_offset,
                copy_size,
            ),
            Encoder::Profiled(enc) => enc.copy_buffer_to_buffer(
                source,
                source_offset,
                destination,
                destination_offset,
                copy_size,
            ),
        }
    }

    /// Copy data from a buffer to a texture.
    #[inline]
    pub fn copy_buffer_to_texture(
        &mut self,
        source: ImageCopyBuffer<'_>,
        destination: ImageCopyTexture<'_>,
        copy_size: Extent3d,
    ) {
        match self {
            Encoder::Wgpu(enc) => enc.copy_buffer_to_texture(source, destination, copy_size),
            Encoder::Profiled(enc) => enc.copy_buffer_to_texture(source, destination, copy_size),
        }
    }

    /// Copy data from a texture to a buffer.
    #[inline]
    pub fn copy_texture_to_buffer(
        &mut self,
        source: ImageCopyTexture<'_>,
        destination: ImageCopyBuffer<'_>,
        copy_size: Extent3d,
    ) {
        match self {
            Encoder::Wgpu(enc) => enc.copy_texture_to_buffer(source, destination, copy_size),
            Encoder::Profiled(enc) => enc.copy_texture_to_buffer(source, destination, copy_size),
        }
    }

    /// Copy data from one texture to another.
    ///
    /// # Panics
    ///
    /// - Textures are not the same type
    /// - If a depth texture, or a multisampled texture, the entire texture must
    ///   be copied
    /// - Copy would overrun either texture
    #[inline]
    pub fn copy_texture_to_texture(
        &mut self,
        source: ImageCopyTexture<'_>,
        destination: ImageCopyTexture<'_>,
        copy_size: Extent3d,
    ) {
        match self {
            Encoder::Wgpu(enc) => enc.copy_texture_to_texture(source, destination, copy_size),
            Encoder::Profiled(enc) => enc.copy_texture_to_texture(source, destination, copy_size),
        }
    }

    /// Clears texture to zero.
    ///
    /// Note that unlike with clear_buffer, `COPY_DST` usage is not required.
    ///
    /// # Implementation notes
    ///
    /// - implemented either via buffer copies and render/depth target clear,
    ///   path depends on texture usages
    /// - behaves like texture zero init, but is performed immediately (clearing
    ///   is *not* delayed via marking it as uninitialized)
    ///
    /// # Panics
    ///
    /// - `CLEAR_TEXTURE` extension not enabled
    /// - Range is out of bounds
    #[inline]
    pub fn clear_texture(&mut self, texture: &Texture, subresource_range: &ImageSubresourceRange) {
        match self {
            Encoder::Wgpu(enc) => enc.clear_texture(texture, subresource_range),
            Encoder::Profiled(enc) => enc.clear_texture(texture, subresource_range),
        }
    }

    /// Clears buffer to zero.
    ///
    /// # Panics
    ///
    /// - Buffer does not have `COPY_DST` usage.
    /// - Range it out of bounds
    #[inline]
    pub fn clear_buffer(
        &mut self,
        buffer: &Buffer,
        offset: BufferAddress,
        size: Option<BufferAddress>,
    ) {
        match self {
            Encoder::Wgpu(enc) => enc.clear_buffer(buffer, offset, size),
            Encoder::Profiled(enc) => enc.clear_buffer(buffer, offset, size),
        }
    }
}
