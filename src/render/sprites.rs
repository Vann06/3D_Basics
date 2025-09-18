//! Sprite drawing helpers (billboards + sorting).
//!
//! Exposes:
//! - `draw_sprite_world`: draw a single billboard sprite with z-buffer
//! - `draw_sprites_sorted`: sort by distance and draw many sprites
//!
use crate::render::framebuffer::Framebuffer;
use crate::core::player::Player;
use crate::render::textures::TextureManager;

pub fn draw_sprite_world(
    framebuffer: &mut Framebuffer,
    player: &Player,
    texman: &TextureManager,
    zbuffer: &[f32],
    world_x: f32,
    world_y: f32,
    key: char,
    size_factor: f32,
    v_offset: f32,
) {
    let sw = framebuffer.width as f32;
    let sh = framebuffer.height as f32;
    let dx = world_x - player.pos.x;
    let dy = world_y - player.pos.y;
    let sprite_a = dy.atan2(dx);
    let mut angle_diff = sprite_a - player.a;
    while angle_diff >  std::f32::consts::PI { angle_diff -= 2.0*std::f32::consts::PI; }
    while angle_diff < -std::f32::consts::PI { angle_diff += 2.0*std::f32::consts::PI; }
    if angle_diff.abs() > player.fov * 0.55 { return; }
    let dist = (dx*dx + dy*dy).sqrt();
    if dist < 8.0 || dist > 2500.0 { return; }
    let screen_x = ((angle_diff / player.fov) + 0.5) * sw;
    let mut sprite_size = (sh / dist) * size_factor;
    let is_enemy_face = matches!(key, 'N'|'E'|'S'|'W');
    let max_px = if is_enemy_face { sh * 0.90 } else { sh * 0.42 };
    if sprite_size > max_px { sprite_size = max_px; }
    if sprite_size <= 1.0 { return; }
    let mut center_y = sh * (0.5 + v_offset);
    if is_enemy_face && dist < 140.0 { center_y += (3.0 * ((dist * 0.05).sin())).round(); }
    let start_x = (screen_x - sprite_size * 0.5).max(0.0) as i32;
    let end_x   = (screen_x + sprite_size * 0.5).min(sw - 1.0) as i32;
    let start_y = (center_y - sprite_size * 0.5).max(0.0) as i32;
    let end_y   = (start_y as f32 + sprite_size).min(sh - 1.0) as i32;
    let (tex_w, tex_h) = texman.image_size(key).unwrap_or((64, 64));
    for sx in start_x..=end_x {
        if (sx as usize) < zbuffer.len() && dist >= zbuffer[sx as usize] { continue; }
        let tx = (((sx - start_x) as f32) / (end_x - start_x + 1) as f32 * tex_w as f32) as u32;
        for sy in start_y..=end_y {
            let ty = (((sy - start_y) as f32) / (end_y - start_y + 1) as f32 * tex_h as f32) as u32;
            let color = texman.get_pixel_color(key, tx, ty);
            if color.a < 8 { continue; }
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(sx as u32, sy as u32);
        }
    }
}

pub fn draw_sprites_sorted(
    framebuffer: &mut Framebuffer,
    player: &Player,
    texman: &TextureManager,
    zbuffer: &[f32],
    sprites: &mut [(&str, f32, f32, char, f32, f32)],
) {
    sprites.sort_by(|a, b| {
        let da = (a.1 - player.pos.x).powi(2) + (a.2 - player.pos.y).powi(2);
        let db = (b.1 - player.pos.x).powi(2) + (b.2 - player.pos.y).powi(2);
        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (_id, x, y, key, size, v_off) in sprites.iter().copied() {
        draw_sprite_world(framebuffer, player, texman, zbuffer, x, y, key, size, v_off);
    }
}
