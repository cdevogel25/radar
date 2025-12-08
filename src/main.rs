use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ ActiveEventLoop, ControlFlow, EventLoop },
    window::{ Window, WindowId }
};

struct WgpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    // keep the window here so it doesn't get dropped
    window: Arc<Window>,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    start_time: Instant,
}

struct App {
    state: Option<WgpuState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Radar")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_resizable(false);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_state = pollster::block_on(async {
            let instance = wgpu::Instance::default();

            let surface = instance.create_surface(window.clone()).unwrap();

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .unwrap();

            let caps = surface.get_capabilities(&adapter);
            let format = caps.formats[0];
            let size = window.inner_size();

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);

            let shader_str = include_str!("circle.wgsl");

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_str)),
            });

            let uniform_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("uniform_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }]
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Uniform Buffer"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
                label: Some("uniform_bind_group"),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

            WgpuState {
                surface,
                device,
                queue,
                config,
                pipeline,
                window,
                uniform_buffer,
                uniform_bind_group,
                start_time: Instant::now(),
            }
        });

        self.state = Some(wgpu_state);

        self.state.as_ref().unwrap().window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if physical_size.width > 0 && physical_size.height > 0 {
                    state.config.width = physical_size.width;
                    state.config.height = physical_size.height;
                    state.surface.configure(&state.device, &state.config);
                    state.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                let elapsed = state.start_time.elapsed().as_secs_f32();

                let rotations_per_second = 0.1;
                let angle = (elapsed * rotations_per_second * std::f32::consts::TAU) % std::f32::consts::TAU;
                let aspect = state.config.width as f32 / state.config.height as f32;
                let radius = 0.9f32;

                let mut data = [0u8; 16];
                data[0..4].copy_from_slice(&angle.to_le_bytes());
                data[4..8].copy_from_slice(&aspect.to_le_bytes());
                data[8..12].copy_from_slice(&radius.to_le_bytes());
                data[12..16].copy_from_slice(&0f32.to_le_bytes());

                state.queue.write_buffer(&state.uniform_buffer, 0, &data);

                let frame = match state.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Outdated) => return,
                    Err(e) => {
                        eprintln!("Render error: {:?}", e);
                        return;
                    }
                };
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rpass.set_pipeline(&state.pipeline);
                    rpass.set_bind_group(0, &state.uniform_bind_group, &[]);
                    rpass.draw(0..3, 0..1);
                }

                state.queue.submit(std::iter::once(encoder.finish()));
                frame.present();

                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App { state: None };

    event_loop.run_app(&mut app).unwrap();
}