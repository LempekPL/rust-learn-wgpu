use crate::input::{Keyboard, Mouse};
use crate::vertex;
use crate::vertex::ShapeBatcher;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::{MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::KeyCode;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
use winit::window::{Window, WindowId};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsRaw {
    screen_size: [f32; 2],
    time: f32,
    _padding: u32,
}

impl From<(winit::dpi::PhysicalSize<u32>, f32)> for GlobalsRaw {
    fn from((size, time): (winit::dpi::PhysicalSize<u32>, f32)) -> Self {
        Self {
            screen_size: [size.width as f32, size.height as f32],
            time,
            _padding: 0,
        }
    }
}

struct Globals {
    raw: GlobalsRaw,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct State {
    instance: wgpu::Instance,
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Option<Arc<Window>>,

    should_close: bool,
    size: winit::dpi::PhysicalSize<u32>,
    time: Time,
    globals: Globals,
    keyboard: Keyboard,
    mouse: Mouse,

    shapes: ShapeBatcher,
}

struct Time {
    last_render_time: web_time::Instant,
    time_start: web_time::Instant,
    delta: web_time::Duration,
}

impl Time {
    fn new() -> Self {
        Self {
            time_start: web_time::Instant::now(),
            last_render_time: web_time::Instant::now(),
            delta: web_time::Duration::new(0, 0),
        }
    }

    fn update(&mut self) {
        let now = web_time::Instant::now();
        self.delta = now - self.last_render_time;
        self.last_render_time = now;
    }

    fn delta(&self) -> f64 {
        self.delta.as_secs_f64()
    }

    fn since_start(&self) -> web_time::Duration {
        self.time_start.elapsed()
    }
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: if cfg!(target_arch = "wasm32")
                || (cfg!(target_os = "android") && cfg!(target_arch = "x86_64"))
            // Vulkan doesn't work with android emulators
            {
                wgpu::Backends::GL
            } else {
                wgpu::Backends::PRIMARY
            },
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_present_mode = surface_caps
            .present_modes
            .iter()
            .find(|&&mode| mode == wgpu::PresentMode::Fifo)
            .copied()
            .unwrap_or(surface_caps.present_modes[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Vertex Buffer"),
            size: size_of::<GlobalsRaw>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globals_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals_bind_group"),
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        let globals = Globals {
            raw: GlobalsRaw::default(),
            buffer: globals_buffer,
            bind_group: globals_bind_group,
        };

        let mut quads =
            ShapeBatcher::new(&device, config.format, &[Some(&globals_bind_group_layout)]);

        quads.draw_rectangle(100., 100., 50., 50., wgpu::Color::WHITE.into());

        Ok(Self {
            instance,
            surface: Some(surface),
            device,
            queue,
            config,
            window: Some(window),

            should_close: false,
            size: winit::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1)),
            time: Time::new(),
            globals,
            keyboard: Keyboard::new(),
            mouse: Mouse::new(),

            shapes: quads,
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
        }
    }

    fn pre_update(&mut self) {
        self.time.update();
        self.globals.raw = GlobalsRaw::from((self.size, self.time.since_start().as_secs_f32()));
    }

    fn post_update(&mut self) {
        self.keyboard.update();
        self.mouse.update();
    }

    fn update(&mut self) {
        log::info!("FPS: {:?}", 1.0 / self.time.delta());
        if self.keyboard.just_pressed(KeyCode::Escape) {
            self.should_close = true;
        }
        if self.keyboard.just_pressed(KeyCode::Space) {
            self.shapes.clear();
        }
        if self.mouse.is_pressed(MouseButton::Left) {
            let (m_x, m_y) = self.mouse.position();
            let size = 10.;
            self.shapes.draw_rectangle(
                m_x as f32 - size / 2.,
                m_y as f32 - size / 2.,
                size,
                size,
                wgpu::Color::GREEN.into(),
            );
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        let Some(surface) = &self.surface else {
            return Ok(());
        };
        // reduces flickering on web
        let max_size = if cfg!(target_arch = "wasm32") {
            2048
        } else {
            u32::MAX
        };
        let target_width = self.size.width.min(max_size);
        let target_height = self.size.height.min(max_size);
        if self.config.width != target_width || self.config.height != target_height {
            self.config.width = target_width;
            self.config.height = target_height;
            surface.configure(&self.device, &self.config);
        }

        use wgpu::CurrentSurfaceTexture as CST;
        let output = match surface.get_current_texture() {
            CST::Success(surface_texture) | CST::Suboptimal(surface_texture) => surface_texture,
            CST::Timeout | CST::Occluded | CST::Validation => return Ok(()),
            CST::Outdated => {
                surface.configure(&self.device, &self.config);
                return Ok(());
            }
            CST::Lost => {
                anyhow::bail!("Lost device");
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.queue.write_buffer(
            &self.globals.buffer,
            0,
            bytemuck::bytes_of(&self.globals.raw),
        );
        use vertex::Rendering;
        self.shapes.update_buffers(&self.device, &self.queue);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

            self.shapes.render(&mut render_pass);
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(output);
        Ok(())
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }

    pub fn run(
        #[cfg(target_os = "android")] app: winit::platform::android::activity::AndroidApp,
    ) -> anyhow::Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            env_logger::Builder::from_default_env()
                .filter_level(log::LevelFilter::Info)
                .init();
        }
        #[cfg(target_arch = "wasm32")]
        {
            console_log::init_with_level(log::Level::Info).unwrap_throw();
        }
        #[cfg(target_os = "android")]
        {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Info)
                    .with_tag("RustLearnWgpu"),
            );
        }

        let mut event_loop = EventLoop::with_user_event();

        #[cfg(target_os = "android")]
        {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            event_loop.with_android_app(app);
        }
        let event_loop = event_loop.build()?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut app = App::new();
            event_loop.run_app(&mut app)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let app = App::new(&event_loop);
            event_loop.spawn_app(app);
        }

        Ok(())
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes().with_title("RustLearnWgpu");

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        if let Some(state) = &mut self.state {
            let surface = state.instance.create_surface(window.clone()).unwrap();
            surface.configure(&state.device, &state.config);

            state.surface = Some(surface);
            state.window = Some(window);
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.as_ref().unwrap().request_redraw();
            event.resize(event.window.as_ref().unwrap().inner_size());
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            None => return,
            Some(state) => state,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::RedrawRequested => {
                state.pre_update();
                state.update();
                state.post_update();

                if state.should_close {
                    event_loop.exit();
                    return;
                }

                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{}", e);
                        event_loop.exit();
                    }
                }
                if let Some(window) = state.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: mouse_state,
                button,
                ..
            } => {
                state.mouse.handle_mouse_input(mouse_state, button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                state.mouse.handle_cursor_moved(position);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                state.mouse.handle_mouse_wheel(delta);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                state.keyboard.handle_event(&event);
            }
            _ => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            state.surface = None;
            state.window = None;
        }
    }
}
