use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use ammo_cached_hash_map::*;

const MAP_KEY_COUNT: u64 = 1000;
const MAP_KEY_MUL: u64 = 50;

fn build_map() -> CachedHashMap<usize, usize> {
    let mut map = CachedHashMap::new();
    for i in 1..MAP_KEY_COUNT {
        map.get_inner_mut().insert(i as usize, i as usize);
    }
    map
}

fn read_grouped(map: &CachedHashMap<usize, usize>, size: usize) {
    for i in 1..(MAP_KEY_COUNT * MAP_KEY_MUL) as usize {
        black_box(map.get_cached(&black_box((i as usize / size) % MAP_KEY_COUNT as usize)));
    }
}

pub fn benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("int_read_write");
    for size in [5, 10, 20, 50, 100, 500] {
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
