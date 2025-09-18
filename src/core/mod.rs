//! Core game types and logic (data, input, AI, world).
//!
//! Re-exports:
//! - `player`: Player data and defaults
//! - `enemy`: Enemy AI and navigation
//! - `maze`: Maze loading and normalization
//! - `process_events`: Input handling and movement

pub mod player;
pub mod enemy;
pub mod maze;
pub mod process_events;
