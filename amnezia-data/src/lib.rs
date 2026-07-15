//! Clean intermediate data format shared between the asset converter and the
//! game.
//!
//! The converter (writer) and the Bevy game (reader) agree on these
//! `serde`-serialisable types. This crate deliberately carries no RPG Maker
//! 2000 or Bevy dependency, so the on-disk format stays engine-agnostic and
//! the shipped game binary inherits nothing from the legacy runtime.
