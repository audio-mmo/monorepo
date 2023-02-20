use ammo_physics::V2;

use crate::hashmap_tilemap::*;
use crate::TileTrait;

#[derive(Default)]
pub struct Tilemap<T: TileTrait> {
    hm: HashmapTilemap<T>,
}

impl<T: TileTrait> Tilemap<T> {
    pub fn new(default_tile: T) -> Tilemap<T> {
        Tilemap {
            hm: HashmapTilemap::new(default_tile),
        }
    }

    /// Get a tile from this tilemap, returning the default tile if no value was set.
    pub fn get(&self, coordinate: V2<u16>) -> &T {
        self.hm.get(coordinate)
    }

    /// Set a tile in this tilemap.
    pub fn set(&mut self, coordinate: V2<u16>, tile: T) {
        self.hm.set(coordinate, tile)
    }

    /// Iterate over all non-default tiles in this tilemap in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (V2<u16>, &T)> {
        self.hm.iter()
    }
}
