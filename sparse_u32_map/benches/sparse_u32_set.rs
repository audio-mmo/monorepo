use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use ammo_sparse_u32_map::*;

const MAP_KEY_COUNT: u64 = 1000;
const MAP_KEY_MUL: u64 = 50;

fn build_map() -> SparseU32Map<u32> {
    let mut map: SparseU32Map<u32> = Default::default();
    for i in 1..MAP_KEY_COUNT {
        map.insert(i as u32, i as u32);
    }
    map
}

fn read_grouped(map: &SparseU32Map<u32>, size: usize) {
    for i in 1..(MAP_KEY_COUNT * MAP_KEY_MUL) as u32 {
        black_box(map.get((i / size as u32) % (MAP_KEY_COUNT * MAP_KEY_MUL) as u32));
    }
}

pub fn benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");
    for size in [1, 5, 10, 20, 50, 100, 500] {
        group.throughput(Throughput::Elements(MAP_KEY_COUNT * MAP_KEY_MUL));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, size| {
            let map = build_map();
            b.iter(|| read_grouped(&map, *size))
        });
    }
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
