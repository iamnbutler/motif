//! GPU frame timing benchmark - measures actual rendering with window.
//!
//! Usage: cargo bench --bench quad_render -- [1k|10k|100k|1m]

use gesso_core::{
    metal::{MetalRenderer, MetalSurface},
    DeviceRect, Quad, Renderer, Scene, Srgba,
};
use glamour::{Point2, Size2};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

const WARMUP_FRAMES: usize = 10;
const SAMPLE_FRAMES: usize = 100;

struct BenchStats {
    frame_times: Vec<Duration>,
    scene_build_times: Vec<Duration>,
    render_times: Vec<Duration>,
}

impl BenchStats {
    fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(SAMPLE_FRAMES),
            scene_build_times: Vec::with_capacity(SAMPLE_FRAMES),
            render_times: Vec::with_capacity(SAMPLE_FRAMES),
        }
    }

    fn record(&mut self, frame: Duration, scene_build: Duration, render: Duration) {
        self.frame_times.push(frame);
        self.scene_build_times.push(scene_build);
        self.render_times.push(render);
    }

    fn report(&self, quad_count: usize) {
        let avg = |times: &[Duration]| {
            times.iter().sum::<Duration>() / times.len() as u32
        };
        let min = |times: &[Duration]| *times.iter().min().unwrap();
        let max = |times: &[Duration]| *times.iter().max().unwrap();
        let p50 = |times: &[Duration]| {
            let mut sorted = times.to_vec();
            sorted.sort();
            sorted[sorted.len() / 2]
        };
        let p99 = |times: &[Duration]| {
            let mut sorted = times.to_vec();
            sorted.sort();
            sorted[(sorted.len() as f64 * 0.99) as usize]
        };

        let frame_avg = avg(&self.frame_times);
        let fps = 1.0 / frame_avg.as_secs_f64();

        println!("\n=== Benchmark Results: {} quads ===", quad_count);
        println!("Samples: {} frames (after {} warmup)", SAMPLE_FRAMES, WARMUP_FRAMES);
        println!();
        println!("Frame time:");
        println!("  avg: {:>8.2?}  ({:.1} FPS)", frame_avg, fps);
        println!("  min: {:>8.2?}  max: {:>8.2?}", min(&self.frame_times), max(&self.frame_times));
        println!("  p50: {:>8.2?}  p99: {:>8.2?}", p50(&self.frame_times), p99(&self.frame_times));
        println!();
        println!("Scene build (CPU):");
        println!("  avg: {:>8.2?}", avg(&self.scene_build_times));
        println!("  min: {:>8.2?}  max: {:>8.2?}", min(&self.scene_build_times), max(&self.scene_build_times));
        println!();
        println!("Render submit:");
        println!("  avg: {:>8.2?}", avg(&self.render_times));
        println!("  min: {:>8.2?}  max: {:>8.2?}", min(&self.render_times), max(&self.render_times));
        println!();
        println!("Throughput: {:.2}M quads/sec", (quad_count as f64 * fps) / 1_000_000.0);
    }
}

struct App {
    window: Option<Window>,
    renderer: Option<MetalRenderer>,
    surface: Option<MetalSurface>,
    scene: Scene,
    quad_count: usize,
    frame_count: usize,
    stats: BenchStats,
    done: bool,
}

impl App {
    fn new(quad_count: usize) -> Self {
        Self {
            window: None,
            renderer: None,
            surface: None,
            scene: Scene::new(),
            quad_count,
            frame_count: 0,
            stats: BenchStats::new(),
            done: false,
        }
    }

    fn build_scene(&mut self, width: f32, height: f32) {
        self.scene.clear();

        // Seeded RNG for reproducibility, but re-seed each frame for variation
        let mut rng = SmallRng::seed_from_u64(self.frame_count as u64);

        let min_size = 4.0_f32;
        let max_size = 100.0_f32.min(width / 10.0).min(height / 10.0);

        for _ in 0..self.quad_count {
            // Random size
            let w = rng.gen_range(min_size..max_size);
            let h = rng.gen_range(min_size..max_size);

            // Random position (ensuring quad stays within viewport)
            let x = rng.gen_range(0.0..(width - w).max(1.0));
            let y = rng.gen_range(0.0..(height - h).max(1.0));

            // Random color
            let r = rng.gen_range(0.0..1.0);
            let g = rng.gen_range(0.0..1.0);
            let b = rng.gen_range(0.0..1.0);
            let a = rng.gen_range(0.5..1.0); // Semi-transparent to fully opaque

            let quad = Quad::new(
                DeviceRect::new(
                    Point2::new(x, y),
                    Size2::new(w, h),
                ),
                Srgba::new(r, g, b, a),
            );
            self.scene.push_quad(quad);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title(format!("Gesso Bench - {} quads", self.quad_count))
                .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0));
            let window = event_loop.create_window(attrs).unwrap();

            let renderer = MetalRenderer::new();
            let surface = unsafe { MetalSurface::new(&window, renderer.device()) };
            surface.set_vsync(false); // Disable vsync for accurate benchmarking

            window.request_redraw();
            self.window = Some(window);
            self.renderer = Some(renderer);
            self.surface = Some(surface);

            println!("Starting benchmark: {} quads", self.quad_count);
            println!("Press Q or Escape to exit early");
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    let should_exit = match &event.logical_key {
                        Key::Named(NamedKey::Escape) => true,
                        Key::Character(c) if c == "q" => true,
                        _ => false,
                    };
                    if should_exit {
                        if !self.done && self.stats.frame_times.len() > 10 {
                            self.stats.report(self.quad_count);
                        }
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    if let Some(window) = &self.window {
                        let scale = window.scale_factor() as f32;
                        surface.resize(size.width as f32 * scale, size.height as f32 * scale);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if self.done {
                    return;
                }

                let frame_start = Instant::now();

                // Get drawable size first
                let drawable_size = self.surface.as_ref().map(|s| s.drawable_size());

                if let Some((width, height)) = drawable_size {
                    // Measure scene build time
                    let scene_start = Instant::now();
                    self.build_scene(width, height);
                    let scene_build_time = scene_start.elapsed();

                    // Measure render time
                    if let (Some(renderer), Some(surface)) = (&mut self.renderer, &mut self.surface) {
                        let render_start = Instant::now();
                        renderer.render(&self.scene, surface);
                        let render_time = render_start.elapsed();

                        let frame_time = frame_start.elapsed();

                        self.frame_count += 1;

                        // Skip warmup frames
                        if self.frame_count > WARMUP_FRAMES {
                            self.stats.record(frame_time, scene_build_time, render_time);

                            // Progress indicator
                            let samples = self.stats.frame_times.len();
                            if samples % 20 == 0 {
                                print!("\rSampling: {}/{}", samples, SAMPLE_FRAMES);
                                use std::io::Write;
                                std::io::stdout().flush().ok();
                            }

                            if samples >= SAMPLE_FRAMES {
                                println!();
                                self.stats.report(self.quad_count);
                                self.done = true;
                                event_loop.exit();
                                return;
                            }
                        }
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn parse_quad_count(arg: &str) -> usize {
    match arg.to_lowercase().as_str() {
        "1k" => 1_000,
        "10k" => 10_000,
        "100k" => 100_000,
        "1m" => 1_000_000,
        _ => arg.parse().unwrap_or_else(|_| {
            eprintln!("Invalid quad count: {}. Use 1k, 10k, 100k, 1m, or a number.", arg);
            std::process::exit(1);
        }),
    }
}

fn main() {
    let quad_count = std::env::args()
        .nth(1)
        .map(|s| parse_quad_count(&s))
        .unwrap_or(10_000);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll); // Run as fast as possible
    let mut app = App::new(quad_count);
    event_loop.run_app(&mut app).unwrap();
}
