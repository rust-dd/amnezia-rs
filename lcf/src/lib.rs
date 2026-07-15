//! Parser for RPG Maker 2000 LCF binary files.
//!
//! Reads `LcfDataBase` (`RPG_RT.ldb`), `LcfMapTree` (`RPG_RT.lmt`) and
//! `LcfMapUnit` (`MapXXXX.lmu`) into typed Rust structures. LCF is a
//! tag/length/value chunk format prefixed by a length-delimited signature
//! string. This crate is dev-time tooling for the asset converter and is
//! never linked into the shipped game binary.
