mod camera;
mod terrain;

use camera::Camera;

use std::time::Instant;
use std::{sync::Arc, u32};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::WinitPlatform;

struct ImguiState {
    context: imgui::Context,
    platform: WinitPlatform,
    renderer: Renderer,
    clear_color: wgpu::Color,
    demo_open: bool,
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let new_window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        self.state = Some(pollster::block_on(State::new(Arc::clone(&new_window))));

        self.window = Some(new_window);

        if let (Some(window), Some(state)) = (&self.window, &mut self.state) {
            state.setup_imgui(window);
            window.request_redraw();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: ()) {
        if let (Some(window), Some(state)) = (&self.window, &mut self.state) {
            let imgui = state.imgui.as_mut().unwrap();
            imgui.platform.handle_event::<()>(
                imgui.context.io_mut(),
                window,
                &winit::event::Event::UserEvent(event),
            );
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let (Some(window), Some(state)) = (&self.window, &mut self.state) {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    if state.mouse_pressed {
                        state.camera.process_mouse(delta.0, delta.1);
                    }
                }
                _ => (),
            }

            if let Some(imgui) = &mut state.imgui {
                imgui.platform.handle_event::<()>(
                    imgui.context.io_mut(),
                    window,
                    &winit::event::Event::DeviceEvent { device_id, event },
                );
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(window), Some(game_state)) = (&self.window, &mut self.state) {
            match event {
                WindowEvent::CloseRequested => {
                    println!("The close button was pressed; stopping");
                    event_loop.exit();
                }
                WindowEvent::Resized(new_size) => {
                    game_state.resize(new_size);
                }
                WindowEvent::RedrawRequested => {
                    window.request_redraw();

                    let now = Instant::now();
                    let dt = now - game_state.last_render_time;

                    if let Some(imgui) = &mut game_state.imgui {
                        imgui
                            .context
                            .io_mut()
                            .update_delta_time(now - imgui.last_frame);
                        imgui.last_frame = now;

                        let frame = match game_state.surface.get_current_texture() {
                            Ok(frame) => frame,
                            Err(e) => {
                                eprintln!("dropped frame: {e:?}");
                                return;
                            }
                        };
                        imgui
                            .platform
                            .prepare_frame(imgui.context.io_mut(), window)
                            .expect("Failed to prepare frame");
                        let ui = imgui.context.frame();

                        {
                            let window = ui.window("Hello world");
                            window
                                .size([300.0, 100.0], Condition::FirstUseEver)
                                .build(|| {
                                    ui.text("Hello world!");
                                    ui.text("This...is...imgui-rs on WGPU!");
                                    ui.separator();
                                    let mouse_pos = ui.io().mouse_pos;
                                    ui.text(format!(
                                        "Mouse Position: ({:.1},{:.1})",
                                        mouse_pos[0], mouse_pos[1]
                                    ));
                                });

                            let window = ui.window("Hello too");
                            window
                                .size([400.0, 200.0], Condition::FirstUseEver)
                                .position([400.0, 200.0], Condition::FirstUseEver)
                                .build(|| {
                                    ui.text(format!("Frametime: {dt:?}"));
                                });

                            ui.show_demo_window(&mut imgui.demo_open);
                        }

                        if imgui.last_cursor != ui.mouse_cursor() {
                            imgui.last_cursor = ui.mouse_cursor();
                            imgui.platform.prepare_render(ui, window);
                        }
                    }

                    game_state.last_render_time = now;
                    game_state.update(dt);
                    match game_state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            game_state.resize(game_state.size)
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            println!("Out of memory; stopping");
                            event_loop.exit();
                        }
                        Err(wgpu::SurfaceError::Timeout) => println!("Surface timeout"),
                    }
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key),
                            state,
                            ..
                        },
                    ..
                } => {
                    game_state.camera.process_keyboard(key, state);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    game_state.camera.process_scroll(&delta);
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    game_state.mouse_pressed = state == ElementState::Pressed;
                }
                _ => (),
            }

            if let Some(imgui) = &mut game_state.imgui {
                imgui.platform.handle_event::<()>(
                    imgui.context.io_mut(),
                    window,
                    &winit::event::Event::WindowEvent { window_id, event },
                );
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RayOrigin {
    pos: [f32; 3],
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenRes {
    res: [f32; 2],
    _padding: [f32; 1],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SceneProps {
    sun_intensity: f32,
    lights_intensity: f32,
    voxel_count: u32,
    ray_offset: f32,
    ray_bounces: f32,
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{:?}", shader)),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
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
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    imgui: Option<ImguiState>,

    last_render_time: Instant,

    camera: Camera,
    camera_buffer: wgpu::Buffer,
    ray_origin_buffer: wgpu::Buffer,

    screen_res_buffer: wgpu::Buffer,
    pixels_texture: wgpu::Texture,

    render_bind_group: wgpu::BindGroup,
    compute_bind_group: wgpu::BindGroup,

    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,

    mouse_pressed: bool,
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY,
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let pixels_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pixels Buffer"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let pixels_texture_view =
            pixels_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let screen_res = ScreenRes {
            res: [size.width as f32, size.height as f32],
            _padding: [0.0],
        };
        let screen_res_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Res Buffer"),
            contents: bytemuck::cast_slice(&[screen_res]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera = camera::Camera::new(
            (0.0, 3.0, -7.0),
            cgmath::Deg(90.0),
            cgmath::Deg(-20.0),
            size,
            cgmath::Deg(45.0),
            0.1,
            100.0,
        );

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera.get_uniform()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute bind group layout descriptor"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let ray_origin_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray origin"),
            contents: bytemuck::cast_slice(&[RayOrigin {
                pos: camera.position.into(),
                _padding: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let voxels = terrain::voxelizer::generate_volume();

        let scene_props_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene props"),
            contents: bytemuck::cast_slice(&[SceneProps {
                sun_intensity: 50.0,
                lights_intensity: 1.0,
                voxel_count: voxels.len() as u32,
                ray_offset: 0.0,
                ray_bounces: 8.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let random_seed_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Random seed"),
            contents: &[0],
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let voxels_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxels"),
            contents: bytemuck::cast_slice(&voxels),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute bind group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&pixels_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: screen_res_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ray_origin_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: scene_props_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: random_seed_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: voxels_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raygen"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raygen.wgsl").into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_module,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render bind group layout desc"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(std::num::NonZeroU64::new(12).unwrap()),
                        },
                        count: None,
                    },
                ],
            });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render bind group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&pixels_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: screen_res_buffer.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("render.wgsl").into()),
            };

            create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                None,
                &[],
                shader,
            )
        };

        Self {
            surface,
            device,
            queue,
            config,
            size,

            imgui: None,

            last_render_time: Instant::now(),

            screen_res_buffer,
            pixels_texture,

            camera,
            camera_buffer,
            ray_origin_buffer,

            compute_bind_group,
            render_bind_group,

            compute_pipeline,
            render_pipeline,

            mouse_pressed: false,
        }
    }

    fn setup_imgui(&mut self, window: &Arc<Window>) {
        let mut context = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::new(&mut context);
        platform.attach_window(
            context.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        context.set_ini_filename(None);

        let font_size = 13.0; // (13.0 * self.hidpi_factor) as f32;
        context.io_mut().font_global_scale = 1.0; // (1.0 / self.hidpi_factor) as f32;

        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let renderer_config = RendererConfig {
            texture_format: self.config.format,
            ..Default::default()
        };

        let renderer = Renderer::new(&mut context, &self.device, &self.queue, renderer_config);
        let last_frame = Instant::now();
        let last_cursor = None;
        let demo_open = true;

        self.imgui = Some(ImguiState {
            context,
            platform,
            renderer,
            clear_color,
            demo_open,
            last_frame,
            last_cursor,
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.camera.resize(new_size);
        }
    }

    fn update(&mut self, dt: instant::Duration) {
        self.camera.update_camera(dt);

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera.get_uniform()]),
        );

        self.queue.write_buffer(
            &self.ray_origin_buffer,
            0,
            bytemuck::cast_slice(&[RayOrigin {
                pos: self.camera.position.into(),
                _padding: 0.0,
            }]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

        // Add dispatch call - calculate workgroups based on screen size
        let workgroup_count_x = (self.config.width * self.config.height + 63) / 64;
        compute_pass.dispatch_workgroups(workgroup_count_x, 1, 1);
        drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_group, &[]);
        render_pass.draw(0..6, 0..1);

        if let Some(imgui) = &mut self.imgui {
            imgui
                .renderer
                .render(
                    imgui.context.render(),
                    &self.queue,
                    &self.device,
                    &mut render_pass,
                )
                .expect("Rendering failed");
        }

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
