use config::Config;
use diffusion::Diffusion;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub mod config;
mod diffusion;
mod event;

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    diffusion: Diffusion,
    steps_per_frame: u32,
    frame_number: u64,
}

impl State {
    async fn new(config: &Config, window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
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
            self.diffusion.render(&mut self.queue, &mut encoder);
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

impl ApplicationHandler<event::Event> for State {
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

    fn user_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        event: event::Event,
    ) {
        match event {
            event::Event::SetKill(kill) => self.diffusion.set_kill(kill),
            event::Event::SetFeed(feed) => self.diffusion.set_feed(feed),
            event::Event::SetDiffusionA(diffusion_a) => self.diffusion.set_diffusion_a(diffusion_a),
            event::Event::SetDiffusionB(diffusion_b) => self.diffusion.set_diffusion_b(diffusion_b),
            event::Event::SetStepsPerFrame(steps_per_frame) => {
                self.steps_per_frame = steps_per_frame
            }
            event::Event::SetTimestep(timestep) => self.diffusion.set_timestep(timestep),
        }
    }
}

#[wasm_bindgen]
pub struct App {
    event_loop: EventLoop<event::Event>,
    window_handle: Arc<Window>,
    config: Config,
}

#[wasm_bindgen]
impl App {
    pub fn new(config: Config) -> Self {
        let event_loop = EventLoop::<event::Event>::with_user_event()
            .build()
            .unwrap();
        let window_attributes = Window::default_attributes()
            .with_active(true)
            .with_inner_size(PhysicalSize::new(config.width, config.height));

        // TODO: fix deprecation, this should go inside `resumed`
        let window = event_loop.create_window(window_attributes).unwrap();

        Self {
            event_loop,
            window_handle: Arc::new(window),
            config,
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = mountCanvas)]
    pub fn mount_canvas(&self) {
        // TODO: this can be improved
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                let canvas = web_sys::Element::from(self.window_handle.canvas()?);
                body.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("error while mounting canvas");
    }

    pub fn updater(&self) -> AppUpdater {
        AppUpdater {
            event_loop_proxy: self.event_loop.create_proxy(),
        }
    }

    pub async fn run(self) {
        let mut state = State::new(&self.config, self.window_handle.clone()).await;
        #[cfg(target_arch = "wasm32")]
        self.event_loop.spawn_app(state);
        #[cfg(not(target_arch = "wasm32"))]
        self.event_loop.run_app(&mut state).unwrap();
    }
}

#[wasm_bindgen]
pub struct AppUpdater {
    event_loop_proxy: EventLoopProxy<event::Event>,
}

#[wasm_bindgen]
impl AppUpdater {
    fn send_event(&self, event: event::Event) {
        self.event_loop_proxy.send_event(event).unwrap();
    }

    #[wasm_bindgen(js_name = setKill)]
    pub fn set_kill(&self, kill: f32) {
        self.send_event(event::Event::SetKill(kill));
    }

    #[wasm_bindgen(js_name = setFeed)]
    pub fn set_feed(&self, feed: f32) {
        self.send_event(event::Event::SetFeed(feed));
    }

    #[wasm_bindgen(js_name = setDiffusionA)]
    pub fn set_diffusion_a(&self, diffusion_a: f32) {
        self.send_event(event::Event::SetDiffusionA(diffusion_a));
    }

    #[wasm_bindgen(js_name = setDiffusionB)]
    pub fn set_diffusion_b(&self, diffusion_b: f32) {
        self.send_event(event::Event::SetDiffusionB(diffusion_b));
    }

    #[wasm_bindgen(js_name = setTimestep)]
    pub fn set_timestep(&self, timestep: f32) {
        self.send_event(event::Event::SetTimestep(timestep));
    }

    #[wasm_bindgen(js_name = setStepsPerFrame)]
    pub fn set_steps_per_frame(&self, steps_per_frame: u32) {
        self.send_event(event::Event::SetStepsPerFrame(steps_per_frame));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn web_init() {
    console_error_panic_hook::set_once();
}
