//! A [HashmapTilemap] is a tilemap backed by a hashmap, for debugging purposes.
//!
//! This tilemap has no redeeming features, save for being obviously correct.  The public tilemap type proxies to it
//! depending on debug builds and features, with asserts that the return values are equal.  This map is `O(n)` memory on number of
//! tiles stored.
use std::collections::HashMap;

use ammo_physics::V2;

use crate::TileTrait;

#[derive(Default)]
pub(crate) struct HashmapTilemap<T: TileTrait> {
    tiles: HashMap<V2<u16>, T>,
    default_tile: T,
}

impl<T: TileTrait> HashmapTilemap<T> {
    pub(crate) fn new(default_tile: T) -> HashmapTilemap<T> {
        HashmapTilemap {
            tiles: Default::default(),
            default_tile,
        }
    }

    /// Get a given tile, if one was set.  Otherwise, return the default tile.
    pub(crate) fn get(&self, coordinate: V2<u16>) -> &T {
        self.tiles.get(&coordinate).unwrap_or(&self.default_tile)
    }

    /// Set a tile.
    pub(crate) fn set(&mut self, coordinate: V2<u16>, tile: T) {
        if tile == self.default_tile {
            self.tiles.remove(&coordinate);
        } else {
            self.tiles.insert(coordinate, tile);
        }
    }

    /// Iterate over all non-default tiles in this tilemap
    pub(crate) fn iter(&self) -> impl Iterator<Item = (V2<u16>, &T)> {
        self.tiles.iter().map(|(x, y)| (*x, y))
    }
}
