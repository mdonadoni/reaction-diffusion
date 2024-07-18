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
    step_number: u64,
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
            usage: BufferUsages::STORAGE,
        });
        let buffer_a1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer A1"),
            contents: bytemuck::cast_slice(&a_init_values),
            usage: BufferUsages::STORAGE,
        });
        let buffer_b0 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer B0"),
            contents: bytemuck::cast_slice(&b_init_values),
            usage: BufferUsages::STORAGE,
        });
        let buffer_b1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Buffer B1"),
            contents: bytemuck::cast_slice(&b_init_values),
            usage: BufferUsages::STORAGE,
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
            entry_point: "diffusion_step",
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

    pub(crate) fn render(&mut self, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut compute_pass = encoder.begin_compute_pass(&Default::default());
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, self.current_bind_group(), &[]);
            // TODO: height and width might not be evenly divisible by workgroup size
            compute_pass.dispatch_workgroups(self.size / 64, 1, 1);
        }
        self.step_number += 1;
    }
}
