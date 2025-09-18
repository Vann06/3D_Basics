//! Rendering utilities and 3D renderer.
//!
//! Re-exports:
//! - `framebuffer`: CPU framebuffer and effects
//! - `textures`: Texture/pixmap manager with fallbacks
//! - `casters`: Ray casting helper
//! - `line`: Bresenham integer line drawing
//! - `render3d`: Column renderer for walls and scene
//! - `sprites`: Sprite drawing (billboards and sorting)

pub mod framebuffer;
pub mod textures;
pub mod casters;
pub mod line;
pub mod render3d;
pub mod sprites;
