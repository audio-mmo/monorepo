use proptest::prelude::*;
use proptest::proptest;

use ammo_physics::V2;

use crate::hashmap_tilemap::*;
use crate::*;

#[derive(Eq, PartialEq, Hash, Debug)]
struct TestTile(u16);

impl TileTrait for TestTile {}

fn fuzz_tilemap_impl(tiles: Vec<(u16, u16, u16)>) -> proptest::test_runner::TestCaseResult {
    let mut authoritative = HashmapTilemap::new(TestTile(0));
    let mut testing = Tilemap::new(TestTile(0));

    for (x, y, t) in tiles.iter().cloned() {
        authoritative.set(V2::new(x, y), TestTile(t));
        testing.set(V2::new(x, y), TestTile(t));
        prop_assert_eq!(authoritative.get(V2::new(x, y)).0, t);
        prop_assert_eq!(testing.get(V2::new(x, y)).0, t);
    }

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000,
        ..Default::default()
    })]
    #[test]
    #[allow(unreachable_code)]
    fn fuzz_tilemap_small(
        tiles in    proptest::collection::vec((0..100u16, 0..100u16, 0..100u16), 0..10000usize),
    ) {
        return fuzz_tilemap_impl(tiles);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000,
        ..Default::default()
    })]
    #[test]
    #[allow(unreachable_code)]
    fn fuzz_tilemap_big(
        tiles in    proptest::collection::vec((0..1000u16, 0..1000u16, 0..100u16), 0..1000usize),
    ) {
        return fuzz_tilemap_impl(tiles);
    }
}

// Tests tiles taking on two non-default values.  This is intended to uncover issues around optimizations for large
// regions of contiguous space.
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000,
        ..Default::default()
    })]
    #[test]
    #[allow(unreachable_code)]
    fn fuzz_tilemap_big_binary(
        tiles in    proptest::collection::vec((0..1000u16, 0..1000u16, 1..=2u16), 0..100000usize),
    ) {
        return fuzz_tilemap_impl(tiles);
    }
}
