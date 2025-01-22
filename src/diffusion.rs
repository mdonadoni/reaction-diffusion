use wgpu::{util::DeviceExt, BufferUsages};

use crate::config::Config;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct ConfigUniform {
    width: u32,
    height: u32,
    size: u32,
    timestep: f32,
    diffusion_a: f32,
    diffusion_b: f32,
    feed: f32,
    kill: f32,
}

pub(crate) struct Diffusion {
    size: u32,
    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group_a: wgpu::BindGroup,
    bind_group_b: wgpu::BindGroup,

    buffer_a0: wgpu::Buffer,
    buffer_a1: wgpu::Buffer,
    buffer_b0: wgpu::Buffer,
    buffer_b1: wgpu::Buffer,

    step_number: u64,
    uniform: ConfigUniform,
    uniform_buffer: wgpu::Buffer,
    uniform_has_changed: bool,
    to_be_reset: bool,
}

impl Diffusion {
    const SHADER: &'static str = include_str!("diffusion.wgsl");

    fn init_values(width: u32, height: u32) -> (Vec<f32>, Vec<f32>) {
        let width = width as usize;
        let height = height as usize;
        let size = width * height;

        let mut a_init_values = Vec::<f32>::with_capacity(size);
        let mut b_init_values = Vec::<f32>::with_capacity(size);
        for i in 0..size {
            if i > size / 5 * 2
                && i < size / 5 * 3
                && i % width > width / 5 * 2
                && i % width < width / 5 * 3
            {
                a_init_values.push(0.0);
                b_init_values.push(1.0);
            } else {
                a_init_values.push(1.0);
                b_init_values.push(0.0);
            }
        }
        (a_init_values, b_init_values)
    }

    pub(crate) fn new(config: &Config, device: &wgpu::Device) -> Self {
        let width = config.width;
        let height = config.height;
        let size = width * height;

        // TODO: support more shapes
        let (a_init_values, b_init_values) = Self::init_values(width, height);

        let config_uniform = ConfigUniform {
            width,
            height,
            size,
            timestep: config.timestep,
            diffusion_a: config.diffusion_a,
            diffusion_b: config.diffusion_b,
            feed: config.feed,
            kill: config.kill,
        };
        let buffer_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Config"),
            contents: bytemuck::cast_slice(&[config_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let buffer_a0 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer A0"),
            contents: bytemuck::cast_slice(&a_init_values),
            usage: BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let buffer_a1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer A1"),
            contents: bytemuck::cast_slice(&a_init_values),
            usage: BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let buffer_b0 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer B0"),
            contents: bytemuck::cast_slice(&b_init_values),
            usage: BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let buffer_b1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer B1"),
            contents: bytemuck::cast_slice(&b_init_values),
            usage: BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Diffusion BindGroupLayout"),
            entries: &[
                // config
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // A input
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // B input
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // A output
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // B output
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Diffusion PipelineLayout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Diffusion Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::SHADER.into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Diffusion ComputePipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

        // Buffer A0 and B0 are inputs, A1 and B1 are outputs
        let bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group A"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_a0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_b0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_a1.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffer_b1.as_entire_binding(),
                },
            ],
        });

        // Buffer A0 and B0 are outputs, A1 and B1 are inputs
        let bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group B"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_a1.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer_b1.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer_a0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffer_b0.as_entire_binding(),
                },
            ],
        });

        Self {
            size,
            compute_pipeline,
            bind_group_layout,
            bind_group_a,
            bind_group_b,
            step_number: 0,
            uniform: config_uniform,
            uniform_buffer: buffer_uniforms,
            uniform_has_changed: false,
            buffer_a0,
            buffer_a1,
            buffer_b0,
            buffer_b1,
            to_be_reset: false,
        }
    }

    pub(crate) fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub(crate) fn current_bind_group(&self) -> &wgpu::BindGroup {
        if self.step_number % 2 == 0 {
            &self.bind_group_a
        } else {
            &self.bind_group_b
        }
    }

    pub(crate) fn step_number(&self) -> u64 {
        self.step_number
    }

    pub(crate) fn render(&mut self, queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {
        if self.uniform_has_changed {
            self.uniform_has_changed = false;
            queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.uniform]),
            )
        }

        if self.to_be_reset {
            self.to_be_reset = false;
            let (a_init_values, b_init_values) =
                Self::init_values(self.uniform.width, self.uniform.height);
            queue.write_buffer(&self.buffer_a0, 0, bytemuck::cast_slice(&a_init_values));
            queue.write_buffer(&self.buffer_a1, 0, bytemuck::cast_slice(&a_init_values));
            queue.write_buffer(&self.buffer_b0, 0, bytemuck::cast_slice(&b_init_values));
            queue.write_buffer(&self.buffer_b1, 0, bytemuck::cast_slice(&b_init_values));
        }

        // prepare render pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&Default::default());
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, self.current_bind_group(), &[]);
            compute_pass.dispatch_workgroups(self.size.div_ceil(64), 1, 1);
        }
        self.step_number += 1;
    }

    pub(crate) fn set_kill(&mut self, kill: f32) {
        self.uniform_has_changed = true;
        self.uniform.kill = kill;
    }

    pub(crate) fn set_feed(&mut self, feed: f32) {
        self.uniform_has_changed = true;
        self.uniform.feed = feed;
    }

    pub(crate) fn set_diffusion_a(&mut self, diffusion_a: f32) {
        self.uniform_has_changed = true;
        self.uniform.diffusion_a = diffusion_a;
    }

    pub(crate) fn set_diffusion_b(&mut self, diffusion_b: f32) {
        self.uniform_has_changed = true;
        self.uniform.diffusion_b = diffusion_b;
    }

    pub(crate) fn set_timestep(&mut self, timestep: f32) {
        self.uniform_has_changed = true;
        self.uniform.timestep = timestep;
    }

    pub(crate) fn reset(&mut self) {
        self.to_be_reset = true;
    }
}
