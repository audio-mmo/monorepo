use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ammo_cached_hash_map::*;

const MAP_KEY_COUNT: usize = 1000;

fn build_map() -> CachedHashMap<usize, usize> {
    let mut map = CachedHashMap::new();
    for i in 1..MAP_KEY_COUNT {
        map.get_inner_mut().insert(i, i);
    }
    map
}

fn read_grouped(map: &CachedHashMap<usize, usize>) {
    for i in 1..MAP_KEY_COUNT * 10 {
        black_box(map.get_cached(&black_box((i / 5) % MAP_KEY_COUNT)));
    }
}

pub fn benchmarks(c: &mut Criterion) {
    c.bench_function("int_read_write", |b| {
        let map = build_map();
        b.iter(|| read_grouped(&map))
    });
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
