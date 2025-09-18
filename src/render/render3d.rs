//! 3D renderer (columns + textured walls, sky/ground).
use raylib::prelude::*;
use crate::render::framebuffer::Framebuffer;
use crate::core::maze::Maze;
use crate::core::player::Player;
use crate::render::textures::TextureManager;
use crate::render::casters::cast_ray;

const CEIL_TOP:   Color = Color::new(10, 12, 18, 255);
const CEIL_MID:   Color = Color::new(20, 24, 32, 255);
const FLOOR_NEAR: Color = Color::new(56, 58, 62, 255);
const FLOOR_FAR:  Color = Color::new(26, 28, 30, 255);

#[inline]
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| -> u8 { ((x as f32) * (1.0 - t) + (y as f32) * t) as u8 };
    Color::new(f(a.r, b.r), f(a.g, b.g), f(a.b, b.b), 255)
}

fn paint_ceiling_and_floor_textured(fb: &mut Framebuffer, texman: &TextureManager) {
    let w = fb.width as u32;
    let h = fb.height as u32;
    let hh = h / 2;
    if let Some((tw, th)) = texman.image_size('K') {
        for y in 0..hh {
            let ty = (y as u32 * th) / hh;
            for x in 0..w {
                let tx = ((x as u32) * tw) / w;
                let c = texman.get_pixel_color('K', tx, ty.min(th-1));
                fb.set_pixel_color(x, y, c);
            }
        }
    } else {
        for y in 0..hh {
            let t = y as f32 / hh as f32;
            let col = lerp_color(CEIL_TOP, CEIL_MID, t);
            fb.set_current_color(col);
            for x in 0..w { fb.set_pixel(x, y); }
        }
    }
    if let Some((tw, th)) = texman.image_size('G') {
        for y in hh..h {
            let ty = (((y - hh) as u32) * th) / (h - hh);
            for x in 0..w {
                let tx = ((x as u32) * tw) / w;
                let c = texman.get_pixel_color('G', tx.min(tw-1), ty.min(th-1));
                fb.set_pixel_color(x, y, c);
            }
        }
    } else {
        for y in hh..h {
            let t = (y - hh) as f32 / (h - hh) as f32;
            let col = lerp_color(FLOOR_FAR, FLOOR_NEAR, t);
            fb.set_current_color(col);
            for x in 0..w { fb.set_pixel(x, y); }
        }
    }
}

pub fn render_3d(
    fb: &mut Framebuffer,
    maze: &Maze,
    block_size: usize,
    player: &Player,
    texman: &TextureManager,
    zbuffer: &mut [f32],
    time_sec: f32,
    panic_mode: bool,
    brightness: f32,
) {
    let w = fb.width as usize;
    let h = fb.height as f32;
    let hh = h * 0.5;
    let _ = (time_sec, panic_mode, brightness);
    paint_ceiling_and_floor_textured(fb, texman);
    for (i, z) in zbuffer.iter_mut().enumerate().take(w) {
        let t = i as f32 / fb.width as f32;
        let ray_a = player.a - (player.fov * 0.5) + (player.fov * t);
        let d = cast_ray(fb, maze, player, ray_a, block_size, false);
        *z = if d > 0.0 { d } else { f32::INFINITY };
        if d <= 0.0 { continue; }

        let diff = ray_a - player.a;
        let d_world = d / diff.cos().abs().max(1e-4);
        let hit_x = player.pos.x + ray_a.cos() * d_world;
        let hit_y = player.pos.y + ray_a.sin() * d_world;
        let ci = (hit_x / block_size as f32).floor() as isize;
        let cj = (hit_y / block_size as f32).floor() as isize;
        let mut is_exit_col = false;
        let mut wall_char = '#';
        if cj >= 0 && ci >= 0 {
            let (ci, cj) = (ci as usize, cj as usize);
            if cj < maze.len() && ci < maze[cj].len() {
                let ch = maze[cj][ci];
                is_exit_col = ch == 'g';
                wall_char = ch;
            }
        }

        const PROJ_K: f32 = 120.0;
        let mut col_h = (hh / d) * PROJ_K;
        let gap: f32 = 12.0;
        if col_h > gap * 2.0 { col_h -= gap * 2.0; }
        let y0 = (hh - col_h * 0.5).max(0.0) as u32;
        let y1 = (hh + col_h * 0.5).min(h - 1.0) as u32;
        let x = i as u32;

        let tex_key: char = if is_exit_col {
            'g'
        } else {
            match wall_char {
                '1' | '2' | '3' | '4' => wall_char,
                _ => {
                    let (ci, cj) = (ci.max(0) as usize, cj.max(0) as usize);
                    let h = (ci.wrapping_mul(31)) ^ (cj.wrapping_mul(17));
                    match h % 3 { 0 => '2', 1 => '3', _ => '4' }
                }
            }
        };

        let (tw, th) = texman.image_size(tex_key).unwrap_or((64, 64));
        let fx = (hit_x / block_size as f32).fract().abs();
        let fy = (hit_y / block_size as f32).fract().abs();
        let dist_fx = fx.min(1.0 - fx);
        let dist_fy = fy.min(1.0 - fy);
        let u = if dist_fx < dist_fy { fy } else { fx };
        let tx = (u * tw as f32).clamp(0.0, tw as f32 - 1.0) as u32;

        for y in y0..=y1 {
            let v = ((y - y0) as f32) / ((y1 - y0 + 1) as f32);
            let ty = (v * th as f32).clamp(0.0, th as f32 - 1.0) as u32;
            let col = texman.get_pixel_color(tex_key, tx, ty);
            fb.set_current_color(col);
            fb.set_pixel(x, y);
        }
    }
}
