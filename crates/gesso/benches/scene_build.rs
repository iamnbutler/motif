//! CPU benchmarks for scene building via DrawContext.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use gesso_core::{DrawContext, Point, Rect, ScaleFactor, Scene, Size, Srgba};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

const QUAD_COUNT: usize = 10_000;

fn bench_draw_context_paint(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_context");
    group.throughput(Throughput::Elements(QUAD_COUNT as u64));

    // Pre-generate random data
    let mut rng = SmallRng::seed_from_u64(42);
    let quads: Vec<(Rect, Srgba)> = (0..QUAD_COUNT)
        .map(|_| {
            let x = rng.gen_range(0.0..1920.0);
            let y = rng.gen_range(0.0..1080.0);
            let w = rng.gen_range(4.0..100.0);
            let h = rng.gen_range(4.0..100.0);
            let r = rng.gen_range(0.0..1.0);
            let g = rng.gen_range(0.0..1.0);
            let b = rng.gen_range(0.0..1.0);
            let a = rng.gen_range(0.5..1.0);
            (
                Rect::new(Point::new(x, y), Size::new(w, h)),
                Srgba::new(r, g, b, a),
            )
        })
        .collect();

    group.bench_function("paint_quad", |b| {
        b.iter(|| {
            let mut scene = Scene::new();
            let scale = ScaleFactor(2.0); // Typical HiDPI
            let mut cx = DrawContext::new(&mut scene, scale);

            for (rect, color) in &quads {
                cx.paint_quad(*rect, *color);
            }

            black_box(scene.quad_count())
        });
    });

    group.bench_function("with_offset", |b| {
        b.iter(|| {
            let mut scene = Scene::new();
            let scale = ScaleFactor(2.0);
            let mut cx = DrawContext::new(&mut scene, scale);

            // Simulate nested UI hierarchy
            cx.with_offset(Point::new(100.0, 100.0), |cx| {
                for (rect, color) in &quads {
                    cx.paint_quad(*rect, *color);
                }
            });

            black_box(scene.quad_count())
        });
    });

    group.finish();
}

fn bench_scene_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("scene");

    // Pre-fill a scene
    let mut rng = SmallRng::seed_from_u64(42);
    let mut scene = Scene::new();
    let scale = ScaleFactor(2.0);
    {
        let mut cx = DrawContext::new(&mut scene, scale);
        for _ in 0..QUAD_COUNT {
            let rect = Rect::new(
                Point::new(rng.gen_range(0.0..1920.0), rng.gen_range(0.0..1080.0)),
                Size::new(rng.gen_range(4.0..100.0), rng.gen_range(4.0..100.0)),
            );
            cx.paint_quad(rect, Srgba::new(1.0, 0.0, 0.0, 1.0));
        }
    }

    group.bench_function("clear", |b| {
        b.iter(|| {
            scene.clear();
            black_box(scene.quad_count())
        });
    });

    group.finish();
}

#[cfg(target_os = "macos")]
fn bench_quad_instance_conversion(c: &mut Criterion) {
    use gesso_core::metal::QuadInstance;
    use gesso_core::{DeviceRect, Quad};
    use glamour::{Point2, Size2};

    let mut group = c.benchmark_group("metal");
    group.throughput(Throughput::Elements(QUAD_COUNT as u64));

    let mut rng = SmallRng::seed_from_u64(42);
    let quads: Vec<Quad> = (0..QUAD_COUNT)
        .map(|_| {
            Quad::new(
                DeviceRect::new(
                    Point2::new(rng.gen_range(0.0..1920.0), rng.gen_range(0.0..1080.0)),
                    Size2::new(rng.gen_range(4.0..100.0), rng.gen_range(4.0..100.0)),
                ),
                Srgba::new(
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.5..1.0),
                ),
            )
        })
        .collect();

    group.bench_function("quad_to_instance", |b| {
        b.iter(|| {
            let instances: Vec<QuadInstance> = quads
                .iter()
                .map(QuadInstance::from_quad)
                .collect();
            black_box(instances.len())
        });
    });

    group.finish();
}

#[cfg(target_os = "macos")]
criterion_group!(
    benches,
    bench_draw_context_paint,
    bench_scene_clear,
    bench_quad_instance_conversion
);

#[cfg(not(target_os = "macos"))]
criterion_group!(benches, bench_draw_context_paint, bench_scene_clear);

criterion_main!(benches);
