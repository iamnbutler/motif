//! CPU benchmarks for scene building.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gesso_core::{DeviceRect, Quad, Scene, Srgba};
use glamour::{Point2, Size2};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

fn random_quad(rng: &mut SmallRng, width: f32, height: f32) -> Quad {
    let min_size = 4.0_f32;
    let max_size = 100.0_f32;

    let w = rng.gen_range(min_size..max_size);
    let h = rng.gen_range(min_size..max_size);
    let x = rng.gen_range(0.0..(width - w).max(1.0));
    let y = rng.gen_range(0.0..(height - h).max(1.0));
    let r = rng.gen_range(0.0..1.0);
    let g = rng.gen_range(0.0..1.0);
    let b = rng.gen_range(0.0..1.0);
    let a = rng.gen_range(0.5..1.0);

    Quad::new(
        DeviceRect::new(Point2::new(x, y), Size2::new(w, h)),
        Srgba::new(r, g, b, a),
    )
}

fn bench_scene_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("scene_push");

    for count in [1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut rng = SmallRng::seed_from_u64(42);
            let quads: Vec<Quad> = (0..count)
                .map(|_| random_quad(&mut rng, 1920.0, 1080.0))
                .collect();

            b.iter(|| {
                let mut scene = Scene::new();
                for quad in &quads {
                    scene.push_quad(black_box(quad.clone()));
                }
                black_box(scene)
            });
        });
    }

    group.finish();
}

fn bench_scene_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("scene_clear");

    for count in [1_000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut rng = SmallRng::seed_from_u64(42);
            let quads: Vec<Quad> = (0..count)
                .map(|_| random_quad(&mut rng, 1920.0, 1080.0))
                .collect();

            let mut scene = Scene::new();
            for quad in &quads {
                scene.push_quad(quad.clone());
            }

            b.iter(|| {
                scene.clear();
                for quad in &quads {
                    scene.push_quad(quad.clone());
                }
                scene.quad_count()
            });
        });
    }

    group.finish();
}

#[cfg(target_os = "macos")]
fn bench_quad_instance_conversion(c: &mut Criterion) {
    use gesso_core::metal::QuadInstance;

    let mut group = c.benchmark_group("quad_to_instance");

    for count in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut rng = SmallRng::seed_from_u64(42);
            let quads: Vec<Quad> = (0..count)
                .map(|_| random_quad(&mut rng, 1920.0, 1080.0))
                .collect();

            b.iter(|| {
                let instances: Vec<QuadInstance> = quads
                    .iter()
                    .map(|q| QuadInstance::from_quad(q))
                    .collect();
                black_box(instances)
            });
        });
    }

    group.finish();
}

#[cfg(target_os = "macos")]
criterion_group!(
    benches,
    bench_scene_push,
    bench_scene_clear,
    bench_quad_instance_conversion
);

#[cfg(not(target_os = "macos"))]
criterion_group!(benches, bench_scene_push, bench_scene_clear);

criterion_main!(benches);
