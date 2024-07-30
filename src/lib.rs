use config::Config;
use diffusion::Diffusion;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub mod config;
mod diffusion;

struct State<'a> {
    window: &'a Window,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    diffusion: Diffusion,
    steps_per_frame: u32,
    frame_number: u64,
}

impl<'a> State<'a> {
    async fn new(config: &Config, window: &'a Window) -> State<'a> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let mut surface_config = surface
            .get_default_config(&adapter, config.width, config.height)
            .unwrap();
        // enable vsync
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let vertices = &[
            [-1.0f32, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of_val(&vertices[0]) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                format: wgpu::VertexFormat::Float32x3,
                shader_location: 0,
            }],
        };

        let diffusion = Diffusion::new(config, &device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[diffusion.bind_group_layout()],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(surface.get_capabilities(&adapter).formats[0].into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
        });
        // TODO: this should go in resize
        surface.configure(&device, &surface_config);

        Self {
            window,
            surface,
            device,
            queue,
            render_pipeline,
            vertex_buffer,
            diffusion,
            steps_per_frame: config.steps_per_frame,
            frame_number: 0,
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            _render_pass.set_pipeline(&self.render_pipeline);
            _render_pass.set_bind_group(0, self.diffusion.current_bind_group(), &[]);
            _render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            _render_pass.draw(0..6, 0..1);
        }

        for _ in 0..self.steps_per_frame {
            self.diffusion.render(&mut encoder);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        self.frame_number += 1;
        println!(
            "Frame: {} Reaction-diffusion step: {}",
            self.frame_number,
            self.diffusion.step_number()
        );
        Ok(())
    }
}

impl<'a> ApplicationHandler<()> for State<'a> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if cause == winit::event::StartCause::Poll {
            self.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if window_id != self.window.id() {
            // event from another window, skip
            return;
        }
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                match self.render() {
                    Ok(_) => event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll),
                    // TODO: should resize (?) instead of exiting
                    Err(wgpu::SurfaceError::Lost) => event_loop.exit(),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            _ => {}
        }
    }
}

pub async fn run(config: &Config) {
    let event_loop = EventLoop::new().unwrap();
    let window_attributes = Window::default_attributes()
        .with_active(true)
        .with_inner_size(PhysicalSize::new(config.width, config.height));

    // TODO: fix deprecation, this should go inside `resumed`
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut state = State::new(config, &window).await;

    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                let canvas = web_sys::Element::from(window.canvas()?);
                body.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("error while mounting canvas");
    }

    event_loop.run_app(&mut state).unwrap();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn web_init() {
    console_error_panic_hook::set_once();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn web_run() {
    let config: Config = Default::default();
    run(&config).await
}
