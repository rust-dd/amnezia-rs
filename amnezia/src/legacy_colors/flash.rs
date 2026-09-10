use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
#[require(super::raster::Rasterized)]
pub(crate) struct SpriteFlash(pub [u8; 4]);

#[cfg(test)]
mod tests;
