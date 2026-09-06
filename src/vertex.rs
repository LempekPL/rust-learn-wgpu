pub trait Vertex: Sized {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute];

    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: Self::ATTRIBUTES,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<[f32; 4]> for Color {
    fn from(v: [f32; 4]) -> Self {
        Self {
            r: v[0],
            g: v[1],
            b: v[2],
            a: v[3],
        }
    }
}

impl From<glam::Vec4> for Color {
    fn from(v: glam::Vec4) -> Self {
        Self {
            r: v.x,
            g: v.y,
            b: v.z,
            a: v.w,
        }
    }
}

impl From<wgpu::Color> for Color {
    fn from(v: wgpu::Color) -> Self {
        Self {
            r: v.r as f32,
            g: v.g as f32,
            b: v.b as f32,
            a: v.a as f32,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PointVertex {
    position: glam::Vec2,
    uv: glam::Vec2,
    color: Color,
}

impl PointVertex {
    pub fn new(position: glam::Vec2, uv: glam::Vec2, color: Color) -> Self {
        Self {
            position,
            uv,
            color,
        }
    }
}

impl Vertex for PointVertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
}

impl From<([f32; 2], [f32; 2], [f32; 4])> for PointVertex {
    fn from((pos, uv, col): ([f32; 2], [f32; 2], [f32; 4])) -> Self {
        Self {
            position: pos.into(),
            uv: uv.into(),
            color: col.into(),
        }
    }
}

impl From<([f32; 2], [f32; 2], Color)> for PointVertex {
    fn from((pos, uv, color): ([f32; 2], [f32; 2], Color)) -> Self {
        Self {
            position: pos.into(),
            uv: uv.into(),
            color,
        }
    }
}

pub trait Rendering {
    fn update_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>);
}

pub struct ShapeBatcher {
    vertices: Vec<PointVertex>,
    indices: Vec<u32>,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_buffer: wgpu::Buffer,
    index_capacity: usize,
    render_pipeline: wgpu::RenderPipeline,
}

impl ShapeBatcher {
    const INITIAL_VERTEX_CAPACITY: usize = 1024;
    const INITIAL_INDEX_CAPACITY: usize = (Self::INITIAL_VERTEX_CAPACITY as f32 * 1.5) as usize;

    pub fn new<'a>(device: &wgpu::Device, format: wgpu::TextureFormat, bind_group_layouts: &'a [Option<&'a wgpu::BindGroupLayout>]) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Vertex Buffer"),
            size: (Self::INITIAL_VERTEX_CAPACITY * size_of::<PointVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Index Buffer"),
            size: (Self::INITIAL_INDEX_CAPACITY * size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/quad.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shape Pipeline Layout"),
                bind_group_layouts,
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shape Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: None,
                buffers: &[Some(PointVertex::LAYOUT)],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: None,
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
            multiview_mask: None,
        });

        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer,
            vertex_capacity: Self::INITIAL_VERTEX_CAPACITY,
            index_buffer,
            index_capacity: Self::INITIAL_INDEX_CAPACITY,
            render_pipeline,
        }
    }

    pub fn draw_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        let left = x;
        let right = x + width;
        let top = y;
        let bottom = y + height;

        let start_index = self.vertices.len() as u32;

        self.vertices.extend_from_slice(&[
            PointVertex::from(([left, top], [0.,0.], color)),
            PointVertex::from(([left, bottom], [0.,1.], color)),
            PointVertex::from(([right, bottom], [1.,1.], color)),
            PointVertex::from(([right, top], [1.,0.], color)),
        ]);

        self.indices.extend_from_slice(&[
            start_index,
            start_index + 1,
            start_index + 2,
            start_index,
            start_index + 2,
            start_index + 3,
        ]);
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

impl Rendering for ShapeBatcher {
    fn update_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.indices.is_empty() {
            return;
        }

        if self.vertices.len() > self.vertex_capacity {
            self.vertex_capacity = (self.vertices.len() * 2).max(Self::INITIAL_VERTEX_CAPACITY);

            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Shape Vertex Buffer"),
                size: (self.vertex_capacity * size_of::<PointVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if self.indices.len() > self.index_capacity {
            self.index_capacity = (self.indices.len() * 2).max(Self::INITIAL_INDEX_CAPACITY);

            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Shape Index Buffer"),
                size: (self.index_capacity * size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.indices.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }
}
