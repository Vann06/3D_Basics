use raylib::prelude::*;
use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;
use crate::textures::TextureManager;
use crate::casters::cast_ray;

/// Colores
const WALL_FILL: Color = Color::new(0, 0, 0, 255); // centro de pared negro

// Normal (azules)
const EDGE_BLUE_BRIGHT: Color = Color::new(60, 230, 255, 255);
const EDGE_BLUE_MID:    Color = Color::new(20, 150, 235, 255);

// Pánico (rojo/naranja)
const EDGE_PANIC_BRIGHT: Color = Color::new(255, 70, 70, 255);
const EDGE_PANIC_MID:    Color = Color::new(255, 150, 60, 255);

// Grosor del brillo en los bordes y “halo” hacia cielo/suelo
const EDGE_LAYERS: u32 = 6;  // más grande = luz más gorda
const HALO_EXT:   u32 = 2;   // px adicionales “por fuera” del borde

// Cielo/suelo
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

#[inline]
fn scale_color(c: Color, k: f32) -> Color {
    let s = k.max(0.0);
    let mul = |v: u8| -> u8 { ((v as f32 * s).min(255.0)) as u8 };
    Color::new(mul(c.r), mul(c.g), mul(c.b), 255)
}

/// Cielo y suelo: usa texturas 'K' (sky) y 'G' (ground) si existen; si no, degrada a gradient
/// Optimized: static sampling (no scrolling)
fn paint_ceiling_and_floor_textured(fb: &mut Framebuffer, texman: &TextureManager, _time_sec: f32) {
    let w = fb.width as u32;
    let h = fb.height as u32;
    let hh = h / 2;
    // Sky
    if let Some((tw, th)) = texman.image_size('K') {
        // static sky: map x across texture width once
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
    // Ground
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

/// Render 3D con bordes superiores/inferiores de pared muy azules y brillantes.
/// `time_sec` anima el pulso; `panic_mode` cambia colores y frecuencia.
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

    paint_ceiling_and_floor_textured(fb, texman, time_sec);

    // Eliminamos pulso/luz en paredes para mejorar rendimiento
    let _ = (panic_mode, brightness); // keep signature-used vars

    for (i, z) in zbuffer.iter_mut().enumerate().take(w) {
        let t = i as f32 / fb.width as f32;
        let ray_a = player.a - (player.fov * 0.5) + (player.fov * t);
        let d = cast_ray(fb, maze, player, ray_a, block_size, false);
        *z = if d > 0.0 { d } else { f32::INFINITY };
        if d <= 0.0 { continue; }

    // Determinar si la celda golpeada es 'g'
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

        // Altura de pared (con un pequeño margen para ver más techo/suelo)
    const PROJ_K: f32 = 120.0;
    let mut col_h = (hh / d) * PROJ_K;
    // Más espacio arriba/abajo para ver más cielo/suelo
    let gap: f32 = 12.0;
        if col_h > gap * 2.0 { col_h -= gap * 2.0; }
        let y0 = (hh - col_h * 0.5).max(0.0) as u32;
        let y1 = (hh + col_h * 0.5).min(h - 1.0) as u32;
        let x = i as u32;

    // Sin modulación temporal de brillo

        // Texturizado de pared: usar 2/3/4 para paredes genéricas y ASCII; respetar dígitos explícitos
        let tex_key: char = if is_exit_col {
            'g'
        } else {
            match wall_char {
                '1' | '2' | '3' | '4' => wall_char,
                _ => {
                    // Distribuye entre '2','3','4' con hash estable de celda
                    let (ci, cj) = (ci.max(0) as usize, cj.max(0) as usize);
                    let h = (ci.wrapping_mul(31)) ^ (cj.wrapping_mul(17));
                    match h % 3 { 0 => '2', 1 => '3', _ => '4' }
                }
            }
        };

        let (tw, th) = texman.image_size(tex_key).unwrap_or((64, 64));
        // Calcular coordenada U en la textura dependiendo de qué lado golpeamos
        let fx = (hit_x / block_size as f32).fract().abs();
        let fy = (hit_y / block_size as f32).fract().abs();
        let dist_fx = fx.min(1.0 - fx);
        let dist_fy = fy.min(1.0 - fy);
        let u = if dist_fx < dist_fy { fy } else { fx };
        let tx = (u * tw as f32).clamp(0.0, tw as f32 - 1.0) as u32;

        // Dibujar columna texturizada con leve modulación de brillo
        for y in y0..=y1 {
            let v = ((y - y0) as f32) / ((y1 - y0 + 1) as f32);
            let ty = (v * th as f32).clamp(0.0, th as f32 - 1.0) as u32;
            let col = texman.get_pixel_color(tex_key, tx, ty);
            fb.set_current_color(col);
            fb.set_pixel(x, y);
        }
    }
}

/// ============= SPRITE DRAWING (compartido por orbs y enemy) =============

/// Dibuja un sprite “billboard” con clave `key`, en (sx,sy) mundo, usando alfa y zbuffer.
pub fn draw_sprite_world(
    framebuffer: &mut Framebuffer,
    player: &Player,
    texman: &TextureManager,
    zbuffer: &[f32],
    world_x: f32,
    world_y: f32,
    key: char,
    size_factor: f32,   // escala base (ej. 40..110)
    v_offset: f32,      // desplazar centro vertical (ej. +0.10 para bajarlo)
) {
    let sw = framebuffer.width as f32;
    let sh = framebuffer.height as f32;

    // vector jugador->sprite
    let dx = world_x - player.pos.x;
    let dy = world_y - player.pos.y;

    // ángulo relativo
    let sprite_a = dy.atan2(dx);
    let mut angle_diff = sprite_a - player.a;
    while angle_diff >  std::f32::consts::PI { angle_diff -= 2.0*std::f32::consts::PI; }
    while angle_diff < -std::f32::consts::PI { angle_diff += 2.0*std::f32::consts::PI; }

    // dentro del FOV (slightly relaxed to avoid half-cut visuals)
    if angle_diff.abs() > player.fov * 0.55 { return; }

    // distancia
    let dist = (dx*dx + dy*dy).sqrt();
    if dist < 8.0 || dist > 2500.0 { return; }

    // posición horizontal
    let screen_x = ((angle_diff / player.fov) + 0.5) * sw;

    // tamaño (inverso a distancia)
    let mut sprite_size = (sh / dist) * size_factor;
    // Evitar sprites gigantes al estar muy cerca (enemigo puede crecer un poco más)
    let is_enemy_face = key == 'N' || key == 'E' || key == 'S' || key == 'W';
    // Permite al enemigo ocupar más alto para ver la cara completa
    let max_px = if is_enemy_face { sh * 0.90 } else { sh * 0.42 };
    if sprite_size > max_px { sprite_size = max_px; }
    if sprite_size <= 1.0 { return; }

    // centro vertical; bottom-anchor enemy so its feet stick to floor, reducing top cut-offs
    let mut center_y = if is_enemy_face { sh * (0.63 + v_offset) } else { sh * (0.5 + v_offset) };
    if is_enemy_face && dist < 140.0 {
        center_y += (3.0 * ((dist * 0.05).sin())).round();
    }

    let start_x = (screen_x - sprite_size * 0.5).max(0.0) as i32;
    let end_x   = (screen_x + sprite_size * 0.5).min(sw - 1.0) as i32;
    let start_y = (center_y - sprite_size * 0.5).max(0.0) as i32;
    let end_y   = (start_y as f32 + sprite_size).min(sh - 1.0) as i32;

    let (tex_w, tex_h) = texman.image_size(key).unwrap_or((64, 64));

    for sx in start_x..=end_x {
        // test de profundidad con muros
        if (sx as usize) < zbuffer.len() && dist >= zbuffer[sx as usize] {
            continue;
        }
        let tx = (((sx - start_x) as f32) / (end_x - start_x + 1) as f32 * tex_w as f32) as u32;

        for sy in start_y..=end_y {
            let ty = (((sy - start_y) as f32) / (end_y - start_y + 1) as f32 * tex_h as f32) as u32;
            let color = texman.get_pixel_color(key, tx, ty);
            if color.a < 8 { continue; } // alfa: descartar transparente
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(sx as u32, sy as u32);
        }
    }
}

/// Dibuja varios sprites con ordenación por distancia (lejanos primero),
/// y añade una sombra simple “pegada al suelo” bajo cada sprite.
pub fn draw_sprites_sorted(
    framebuffer: &mut Framebuffer,
    player: &Player,
    texman: &TextureManager,
    zbuffer: &[f32],
    sprites: &mut [(&str, f32, f32, char, f32, f32)],
) {
    // Ordenar por distancia descendente (pintar primero el más lejano)
    sprites.sort_by(|a, b| {
        let da = (a.1 - player.pos.x).powi(2) + (a.2 - player.pos.y).powi(2);
        let db = (b.1 - player.pos.x).powi(2) + (b.2 - player.pos.y).powi(2);
        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
    });

    const DRAW_SPRITE_SHADOW: bool = false; // disable for performance
    for (_id, x, y, key, size, v_off) in sprites.iter().copied() {
        // Optional: Sombra elíptica bajo el sprite
        // Proyectar a pantalla como en draw_sprite_world
        let sw = framebuffer.width as f32;
        let sh = framebuffer.height as f32;
        let dx = x - player.pos.x;
        let dy = y - player.pos.y;
        let sprite_a = dy.atan2(dx);
        let mut angle_diff = sprite_a - player.a;
        while angle_diff >  std::f32::consts::PI { angle_diff -= 2.0*std::f32::consts::PI; }
        while angle_diff < -std::f32::consts::PI { angle_diff += 2.0*std::f32::consts::PI; }
    if DRAW_SPRITE_SHADOW && angle_diff.abs() <= player.fov * 0.5 {
            let dist = (dx*dx + dy*dy).sqrt();
            if dist >= 8.0 && dist <= 2500.0 {
                let screen_x = ((angle_diff / player.fov) + 0.5) * sw;
                let mut sprite_size = (sh / dist) * size;
                let max_px = if key == 'N' || key == 'E' || key == 'S' || key == 'W' { sh * 0.80 } else { sh * 0.42 };
                if sprite_size > max_px { sprite_size = max_px; }
                if sprite_size > 3.0 {
                    let center_y = sh * (0.5 + v_off);
                    let bottom_y = (center_y + sprite_size * 0.5).min(sh - 1.0);
                    let rx = (sprite_size * 0.35).max(6.0);
                    let ry = (sprite_size * 0.08).max(3.0);
                    let cx = screen_x.clamp(0.0, sw - 1.0) as i32;
                    let cy = bottom_y as i32;
                    let min_x = (cx as f32 - rx).max(0.0) as i32;
                    let max_x = (cx as f32 + rx).min(sw - 1.0) as i32;
                    let min_y = (cy as f32 - ry).max(0.0) as i32;
                    let max_y = (cy as f32 + ry).min(sh - 1.0) as i32;
                    for sy in min_y..=max_y {
                        for sx in min_x..=max_x {
                            let nx = (sx as f32 - cx as f32) / rx;
                            let ny = (sy as f32 - cy as f32) / ry;
                            let r2 = nx*nx + ny*ny;
                            if r2 <= 1.0 {
                                // Oscurecer pixel existente (mezcla simple)
                                let old = framebuffer.get_pixel(sx as u32, sy as u32);
                                let k = 0.55 * (1.0 - r2).clamp(0.0, 1.0); // más oscuro al centro
                                let mul = |v: u8| -> u8 { ((v as f32) * (1.0 - k)).clamp(0.0, 255.0) as u8 };
                                let newc = Color::new(mul(old.r), mul(old.g), mul(old.b), old.a);
                                framebuffer.set_pixel_color(sx as u32, sy as u32, newc);
                            }
                        }
                    }
                }
            }
        }

        // Ahora dibujar el sprite en sí
        draw_sprite_world(framebuffer, player, texman, zbuffer, x, y, key, size, v_off);
    }
}