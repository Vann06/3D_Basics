//! Game entry point and loop.
//!
//! Responsibilities:
//! - Initialize window, audio, textures, framebuffer, and levels
//! - Run the main update/draw loop and manage `GameState`
//! - Handle input (delegated to `process_events`), enemy updates, orb collection
//! - Orchestrate 3D render (`render3d`), sprites, flashlight overlay, HUD, and minimap
//! - Maintain internal render scaling for performance and upload framebuffer to a texture
//!
#![allow(unused_imports)]
#![allow(dead_code)]

mod render;
mod core;
mod audio;

use crate::render::textures::TextureManager;
use raylib::prelude::*;
use crate::audio::manager::AudioManager;
use std::thread;
use std::time::Duration;
use crate::render::framebuffer::Framebuffer;
use crate::core::maze::{Maze, load_maze};
use crate::core::player::Player;
use crate::core::process_events::process_events;
use crate::render::casters::cast_ray;
use crate::render::render3d::render_3d;
use crate::render::sprites::{draw_sprite_world, draw_sprites_sorted};
use rand::seq::SliceRandom;
use crate::core::enemy::Enemy;
use std::path::Path;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GameState { Menu, Playing, Escaping, Won, Caught }

// Menu state: simple "Play" entry that cycles through preset levels.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum MenuItem { Play }

#[derive(Clone)]
struct LevelCfg {
    file: &'static str,
    enemy_enabled: bool,
    show_minimap: bool,
    brightness: f32, // multiplicador para paredes (líneas azules más intensas)
}

fn level_cfg(idx: i32) -> LevelCfg {
    match idx {
    // L1: enemigo activo y minimapa ON; brillo base 1.0
    0 => LevelCfg { file: "maze1.txt", enemy_enabled: true,  show_minimap: true,  brightness: 1.0 },
    // L2: enemigo ON; brillo un poco más fuerte
    1 => LevelCfg { file: "maze2.txt", enemy_enabled: true,  show_minimap: true,  brightness: 1.15 },
    // L3: enemigo ON; con minimapa; un poco más intenso
    2 => LevelCfg { file: "maze3.txt", enemy_enabled: true,  show_minimap: true,  brightness: 1.25 },
    _ => LevelCfg { file: "maze1.txt", enemy_enabled: true,  show_minimap: true,  brightness: 1.0 },
    }
}

// Tamaño de celda en unidades de mundo
pub const BLOCK: f32 = 64.0;

// ---------- ORBS ----------
struct Orb { x: f32, y: f32, active: bool }

fn is_free_cell(maze: &Maze, i: usize, j: usize) -> bool {
    if j >= maze.len() || i >= maze[j].len() { return false; }
    let c = maze[j][i];
    c == ' ' || c == 'g'
}
fn is_safe_cell(maze: &Maze, i: usize, j: usize) -> bool {
    if !is_free_cell(maze, i, j) { return false; }
    let dirs = [(-1,0),(1,0),(0,-1),(0,1)];
    for (dx,dy) in dirs {
        let ni = i as isize + dx;
        let nj = j as isize + dy;
        if ni < 0 || nj < 0 { continue; }
        let (ni, nj) = (ni as usize, nj as usize);
        if nj < maze.len() && ni < maze[nj].len() {
            let c = maze[nj][ni];
            if c != ' ' && c != 'g' { return false; }
        }
    }
    true
}
fn spawn_orbs_in_empty_cells(maze: &Maze, block: f32, count: usize) -> Vec<Orb> {
    let mut free_cells: Vec<(usize,usize)> = Vec::new();
    for (j, row) in maze.iter().enumerate() {
        for (i, _c) in row.iter().enumerate() {
            if is_safe_cell(maze, i, j) {
                free_cells.push((i, j));
            }
        }
    }
    let mut rng = rand::thread_rng();
    free_cells.shuffle(&mut rng);
    free_cells.into_iter()
        .take(count)
        .map(|(i,j)| Orb {
            x: (i as f32 + 0.5) * block,
            y: (j as f32 + 0.5) * block,
            active: true,
        })
        .collect()
}

// ---------- 2D DEBUG ----------
fn draw_cell(
    framebuffer: &mut Framebuffer,
    xo: usize,
    yo: usize,
    block_size: usize,
    cell: char,
) {
    if cell == ' ' { return; }
    framebuffer.set_current_color(Color::RED);
    for x in xo..xo + block_size {
        for y in yo..yo + block_size {
            framebuffer.set_pixel(x as u32, y as u32);
        }
    }
}
pub fn render_maze(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    block_size: usize,
) {
    for (row_index, row) in maze.iter().enumerate() {
        for (col_index, &cell) in row.iter().enumerate() {
            let xo = col_index * block_size;
            let yo = row_index * block_size;
            draw_cell(framebuffer, xo, yo, block_size, cell);
        }
    }
}

// ---------- MINIMAPA ----------
fn draw_minimap(
    d: &mut RaylibDrawHandle,
    maze: &Maze,
    player: &Player,
    orbs: &[Orb],
    enemy: &Enemy,
    window_width: i32,
) {
    let cell_px: i32 = 9;
    let margin: i32 = 10;
    let map_w: i32 = (maze[0].len() as i32) * cell_px;
    let map_h: i32 = (maze.len() as i32) * cell_px;

    let origin_x = window_width - map_w - margin;
    let origin_y = margin;

    d.draw_rectangle(origin_x - 4, origin_y - 4, map_w + 8, map_h + 8, Color::new(0, 0, 0, 180));

    for (j, row) in maze.iter().enumerate() {
        for (i, &c) in row.iter().enumerate() {
            let x = origin_x + (i as i32) * cell_px;
            let y = origin_y + (j as i32) * cell_px;
            if c != ' ' && c != 'g' {
                d.draw_rectangle(x, y, cell_px, cell_px, Color::new(120, 120, 140, 230));
            } else if c == 'g' {
                // salida: destacar en blanco brillante
                d.draw_rectangle(x, y, cell_px, cell_px, Color::new(255, 255, 255, 240));
            }
        }
    }

    for o in orbs.iter().filter(|o| o.active) {
        let i = (o.x / BLOCK).floor() as i32;
        let j = (o.y / BLOCK).floor() as i32;
        let cx = origin_x + i * cell_px + cell_px / 2;
        let cy = origin_y + j * cell_px + cell_px / 2;
        d.draw_circle(cx, cy, (cell_px as f32) * 0.25, Color::YELLOW);
    }

    // Jugador
    let pi = (player.pos.x / BLOCK).floor() as i32;
    let pj = (player.pos.y / BLOCK).floor() as i32;
    let px = origin_x + pi * cell_px + cell_px / 2;
    let py = origin_y + pj * cell_px + cell_px / 2;

    d.draw_circle(px, py, (cell_px as f32) * 0.35, Color::GREEN);
    let dir_len = (cell_px as f32) * 0.8;
    let dx = player.a.cos() * dir_len;
    let dy = player.a.sin() * dir_len;
    d.draw_line(px, py, (px as f32 + dx) as i32, (py as f32 + dy) as i32, Color::LIME);

    // Enemy marker (no radius visualization)
    if enemy.active {
        let ei = (enemy.x / BLOCK).floor() as i32;
        let ej = (enemy.y / BLOCK).floor() as i32;
        let ex = origin_x + ei * cell_px + cell_px / 2;
        let ey = origin_y + ej * cell_px + cell_px / 2;
        d.draw_circle(ex, ey, (cell_px as f32) * 0.35, Color::RED);
    }

    d.draw_rectangle_lines(origin_x - 4, origin_y - 4, map_w + 8, map_h + 8, Color::WHITE);
}

fn reset_game(maze: &Maze, _block_size: usize) -> (Vec<Orb>, usize, Player, Enemy) {
    // Much more orbs: roughly 20% of free cells, capped to avoid extremes
    let free_cells = maze.iter().flatten().filter(|&&c| c == ' ' || c == 'g').count();
    let desired = ((free_cells as f32) * 0.20).clamp(20.0, 180.0) as usize;
    let orbs = spawn_orbs_in_empty_cells(maze, BLOCK, desired);
    let score: usize = 0;
    let player = Player::new(1.5 * BLOCK, 1.5 * BLOCK, 0.0);
    let enemy = Enemy::new(2.5 * BLOCK, 2.5 * BLOCK, 0.0);
    (orbs, score, player, enemy)
}

fn main() {
    let window_width = 1300;
    let window_height = 900;
    // Internal render scale (lower than 1.0 to boost FPS). 0.66 ~ 66% resolution.
    let render_scale: f32 = 0.66;
    let fb_w = ((window_width as f32) * render_scale).round() as i32;
    let fb_h = ((window_height as f32) * render_scale).round() as i32;
    let block_size = BLOCK as usize;

    let (mut window, raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raycaster Example")
        .build();

    window.disable_cursor();
    window.set_target_fps(60);

    // Audio manager (rodio)
    let mut audio = AudioManager::new();
    if let Some(a) = audio.as_mut() {
        a.load_sfx_auto();
        a.play_music_loop_auto();
    }
    let mut caught_sfx_played = false;

    let mut texman = TextureManager::new(&mut window, &raylib_thread);
    let mut framebuffer = Framebuffer::new(fb_w as u32, fb_h as u32);
    framebuffer.set_background_color(Color::new(20, 20, 30, 255));

    // Textura persistente para blitear el framebuffer cada frame
    let img = Image::gen_image_color(fb_w, fb_h, Color::BLACK);
    let mut fb_tex = window
        .load_texture_from_image(&raylib_thread, &img)
        .expect("crear texture framebuffer");

    // Cargar nivel por defecto (Level 1)
    let mut selected_level: i32 = 0;
    let mut cfg = level_cfg(selected_level);
    let mut maze = load_maze(cfg.file);

    let (mut orbs, mut score, mut player, mut enemy) = reset_game(&maze, block_size);
    enemy.active = false; // spawn retardado
    let mut enemy_spawn_timer: f32 = 1.8; // aparece tras ~1.8s
    let mut level_start_time = window.get_time() as f32;
    // Preload `teto.gif` for the menu (single frame; GIF animation not handled)
    let tex_teto = Image::load_image("assets/teto.gif")
        .ok()
        .and_then(|img| window.load_texture_from_image(&raylib_thread, &img).ok());

    let mut zbuffer = vec![f32::INFINITY; framebuffer.width as usize];
    let mode_3d = true;
    let mut game_state = GameState::Menu;
    // Simplified menu: Enter starts next level; no menu index needed

    // Delta time tracking
    let mut last_time = window.get_time();

    while !window.window_should_close() {
        // dt
    let now = window.get_time();
    let dt = (now - last_time) as f32;
    last_time = now;

    // Menu input & drawing
    let mut touched_exit = false;
    if matches!(game_state, GameState::Menu) {
        // Level selection shortcuts on menu
        if window.is_key_pressed(KeyboardKey::KEY_ONE) { selected_level = 0; }
        if window.is_key_pressed(KeyboardKey::KEY_TWO) { selected_level = 1; }
        if window.is_key_pressed(KeyboardKey::KEY_THREE) { selected_level = 2; }
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) || window.is_key_pressed(KeyboardKey::KEY_KP_ENTER) {
            let start_idx = selected_level.clamp(0, 2);
            cfg = level_cfg(start_idx);
            maze = load_maze(cfg.file);
            let (o, s, p, e) = reset_game(&maze, block_size);
            orbs = o; score = s; player = p; enemy = e;
            enemy.active = false;
            // Spawn earlier on L1 and L2; keep later on L3
            enemy_spawn_timer = if start_idx == 0 || start_idx == 1 { 0.5 } else { 12.0 };
            level_start_time = window.get_time() as f32;
            game_state = GameState::Playing;
            // Next time in menu, advance to next level
            selected_level = (start_idx + 1) % 3;
        }
    } else {
    // Entrada jugador solo cuando estamos jugando/escapando; bloqueado si "Caught"
        if matches!(game_state, GameState::Playing | GameState::Escaping) {
            touched_exit = process_events(&mut window, &mut player, &maze, block_size);
        }
        // ENTER para volver al menú desde el juego o desde Caught
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) || window.is_key_pressed(KeyboardKey::KEY_KP_ENTER) {
            game_state = GameState::Menu;
            continue;
        }
    }

    // Lógica de enemigo
        if matches!(game_state, GameState::Playing | GameState::Escaping) {
            // activar enemigo tras un pequeño retraso, y colocarlo lejos del jugador
            if cfg.enemy_enabled {
                if !enemy.active {
                    // para L2/L3: aparece hacia media partida: por tiempo o por progreso de orbs
                    let elapsed = window.get_time() as f32 - level_start_time;
                    let total = (orbs.len() + score) as i32; // total inicial de orbs
                    let collected = score as i32;
                    let mid_orbs = total.max(1) / 2;
                    enemy_spawn_timer -= dt;
                    let time_gate = if selected_level == 1 { elapsed >= 12.0 } else { elapsed >= 10.0 };
                    let progress_gate = collected >= mid_orbs;
                    if enemy_spawn_timer <= 0.0 || time_gate || progress_gate {
                        enemy.active = true;
                        // Prefer spawn near the exit on Level 2, otherwise far from player
                        let mut placed = false;
                        if selected_level == 1 {
                            // buscar 'g' y elegir una celda libre en un anillo alrededor
                            let mut exit_pos: Option<(usize,usize)> = None;
                            'outer: for (j,row) in maze.iter().enumerate() {
                                for (i,&c) in row.iter().enumerate() {
                                    if c == 'g' { exit_pos = Some((i,j)); break 'outer; }
                                }
                            }
                            if let Some((gi, gj)) = exit_pos {
                                let h = maze.len();
                                let w = maze[0].len();
                                // probar anillos de radio 1..=6, eligiendo el más lejos del jugador dentro del primer anillo con candidatos
                                for r in 1..=6 {
                                    let mut ring_best: Option<(usize,usize,f32)> = None;
                                    let r_i = r as isize;
                                    for dy in -r_i..=r_i {
                                        for dx in -r_i..=r_i {
                                            if dx.abs().max(dy.abs()) != r_i { continue; }
                                            let ii = gi as isize + dx;
                                            let jj = gj as isize + dy;
                                            if ii < 0 || jj < 0 { continue; }
                                            let (ii, jj) = (ii as usize, jj as usize);
                                            if jj >= h || ii >= maze[jj].len() { continue; }
                                            if maze[jj][ii] != ' ' { continue; }
                                            let wx = (ii as f32 + 0.5) * BLOCK;
                                            let wy = (jj as f32 + 0.5) * BLOCK;
                                            let dxp = wx - player.pos.x; let dyp = wy - player.pos.y;
                                            let d2p = dxp*dxp + dyp*dyp;
                                            // evitar spawns demasiado cerca del jugador (< 6 celdas)
                                            if d2p < (6.0*BLOCK)*(6.0*BLOCK) { continue; }
                                            if ring_best.map(|b| d2p > b.2).unwrap_or(true) {
                                                ring_best = Some((ii,jj,d2p));
                                            }
                                        }
                                    }
                                    if let Some((ii,jj,_)) = ring_best {
                                        enemy.x = (ii as f32 + 0.5) * BLOCK;
                                        enemy.y = (jj as f32 + 0.5) * BLOCK;
                                        placed = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !placed {
                            // fallback: buscar celda libre lejana al jugador
                            let mut best: Option<(usize,usize,f32)> = None;
                            for (j,row) in maze.iter().enumerate() {
                                for (i,&c) in row.iter().enumerate() {
                                    if c == ' ' {
                                        let wx = (i as f32 + 0.5) * BLOCK;
                                        let wy = (j as f32 + 0.5) * BLOCK;
                                        let dx = wx - player.pos.x; let dy = wy - player.pos.y;
                                        let d2 = dx*dx + dy*dy;
                                        if d2 > 10.0*BLOCK*10.0*BLOCK {
                                            if best.map(|b| d2 > b.2).unwrap_or(true) { best = Some((i,j,d2)); }
                                        }
                                    }
                                }
                            }
                            if let Some((i,j,_)) = best {
                                enemy.x = (i as f32 + 0.5) * BLOCK;
                                enemy.y = (j as f32 + 0.5) * BLOCK;
                            }
                        }
                    }
                }
                if enemy.active {
                    enemy.update(&maze, player.pos.x, player.pos.y, block_size, dt);
                }
            }
        }

    // Recoger orbs
        {
            let pr = 18.0;
            for (_idx, o) in orbs.iter_mut().enumerate() {
                if o.active {
                    let dx = o.x - player.pos.x;
                    let dy = o.y - player.pos.y;
                    if (dx*dx + dy*dy).sqrt() <= pr {
                        o.active = false;
                        score += 1;
                        if let Some(a) = audio.as_mut() { a.play_orb(); }
                    }
                }
            }
        }

    // Estado de juego
    if game_state == GameState::Playing && !orbs.iter().any(|o| o.active) {
            game_state = GameState::Escaping;
        }
    if game_state == GameState::Escaping && touched_exit {
            game_state = GameState::Won;
        }

        framebuffer.clear();

        if matches!(game_state, GameState::Menu) {
            // Menu screen: enhanced red-themed look with level list
            let mut d = window.begin_drawing(&raylib_thread);
            // Background gradient (dark to deep red)
            for i in 0..window_height {
                let t = i as f32 / window_height as f32;
                let r = (24.0 + 120.0 * t) as u8;
                d.draw_line(0, i, window_width, i, Color::new(r, 8, 16, 255));
            }
            // Red vignette using rings
            let cx = (window_width as f32) * 0.5;
            let cy = (window_height as f32) * 0.5;
            for k in 0..8 {
                let alpha = (18 + k * 10) as u8;
                let inner = (window_height as f32 * (0.55 + k as f32 * 0.03)).min(window_height as f32);
                let outer = inner + 18.0;
                d.draw_ring(
                    Vector2 { x: cx, y: cy },
                    inner as f32,
                    outer as f32,
                    0.0,
                    360.0,
                    72,
                    Color::new(220, 20, 40, alpha),
                );
            }
            // Title with red glow (fake blur by layered text)
            let title = "Teto Escape";
            let ts = 64;
            let tw = d.measure_text(title, ts);
            let tx = (window_width - tw)/2 - 150;
            let ty = 60;
            for (ox, oy, col) in [(-2,0, Color::new(255,50,80,120)), (2,0, Color::new(255,50,80,120)), (0,2, Color::new(255,50,80,120)), (0,-2, Color::new(255,50,80,120))] {
                d.draw_text(title, tx+ox, ty+oy, ts, col);
            }
            d.draw_text(title, tx, ty, ts, Color::new(255, 230, 210, 255));

            // Left panel: level list
            let base_x = 100; let base_y = 220;
            d.draw_text("Select Level:", base_x, base_y - 40, 28, Color::new(255, 200, 200, 255));
            for i in 0..3 {
                let y = base_y + i * 48;
                let selected = i == selected_level.clamp(0,2);
                let label = format!("Level {}", i+1);
                if selected {
                    d.draw_rectangle(base_x - 16, y - 6, 200, 40, Color::new(160, 20, 30, 160));
                    d.draw_text(&label, base_x, y, 36, Color::new(255, 100, 120, 255));
                } else {
                    d.draw_text(&label, base_x, y, 34, Color::new(230, 220, 220, 220));
                }
            }
            d.draw_text("1/2/3: Choose | ENTER: Play | ESC: Exit", base_x, base_y + 3*48 + 20, 22, Color::new(230,230,230,220));

            // Right panel for teto.gif with slight bobbing animation & red tint
            let panel_x = (window_width as f32 * 0.55) as i32;
            d.draw_rectangle(panel_x, 0, window_width - panel_x, window_height, Color::new(24, 10, 12, 200));
            if let Some(tex) = &tex_teto {
                let tex_w = tex.width(); let tex_h = tex.height();
                let target_w = window_width - panel_x - 20; let target_h = window_height - 20;
                let time_sec = d.get_time() as f32;
                let wob = 0.04 * (time_sec * 2.6).sin(); // pequeña oscilación de escala
                let base_scale = (target_w as f32 / tex_w as f32).min(target_h as f32 / tex_h as f32);
                let scale = (base_scale * (1.0 + wob)).max(0.1);
                let draw_w = (tex_w as f32 * scale) as i32; let draw_h = (tex_h as f32 * scale) as i32;
                let dx = panel_x + (target_w - draw_w)/2 + 10; let mut dy = (target_h - draw_h)/2 + 10;
                dy += (6.0 * (time_sec * 1.8).sin()) as i32; // bob vertical sutil
                d.draw_texture_pro(&tex,
                    Rectangle { x: 0.0, y: 0.0, width: tex_w as f32, height: tex_h as f32 },
                    Rectangle { x: dx as f32, y: dy as f32, width: draw_w as f32, height: draw_h as f32 },
                    Vector2 { x: 0.0, y: 0.0 },
                    0.0,
                    Color::new(255, 200, 200, 255));
                // Soft red overlay for a subtle blur feel
                d.draw_rectangle(dx-12, dy-12, draw_w+24, draw_h+24, Color::new(200, 30, 50, 40));
            } else {
                let msg = "Missing assets/teto.gif";
                let tw = d.measure_text(msg, 24);
                d.draw_text(msg, panel_x + (window_width - panel_x - tw)/2, window_height/2, 24, Color::RED);
            }
            continue; // skip rest of render loop while in menu
        } else if !mode_3d {
            // Vista 2D debug
            render_maze(&mut framebuffer, &maze, block_size);
            framebuffer.set_current_color(Color::YELLOW);
            framebuffer.set_pixel(player.pos.x as u32, player.pos.y as u32);
            framebuffer.set_current_color(Color::WHITE);
            let num_rays = 25;
            for i in 0..num_rays {
                let t = i as f32 / num_rays as f32;
                let ray_angle = player.a - (player.fov / 2.0) + (player.fov * t);
                cast_ray(&mut framebuffer, &maze, &player, ray_angle, block_size, true);
            }
        } else {
            // 3D + sprites

            // Parámetros de render
            let time_sec = window.get_time() as f32;
            // Pánico si el enemigo te ve o si está muy cerca
            let enemy_sees = enemy.sees_player(&maze, player.pos.x, player.pos.y, block_size);
            let dxp = enemy.x - player.pos.x;
            let dyp = enemy.y - player.pos.y;
            let dist_now = (dxp*dxp + dyp*dyp).sqrt();
            let near = dist_now < 200.0;
            let panic_mode = enemy_sees || near;
            texman.set_alert_mode(panic_mode);
            // Sin tinte verde en el enemigo cuando persigue

            // Render principal
            render_3d(
                &mut framebuffer,
                &maze,
                block_size,
                &player,
                &texman,
                &mut zbuffer,
                time_sec,
                panic_mode,
                cfg.brightness,
            );

            // While seen: play continuous loop (enemy_seen). Stop when not seen. (No player alert sound.)
            if let Some(a) = audio.as_mut() {
                if enemy_sees {
                    a.start_enemy_seen_loop();
                } else {
                    a.stop_enemy_seen_loop();
                }
            }

            // Scale blur with proximity but gate by performance: only apply when running ~55+ FPS
            let strong_range = 200.0; // strongest effect here
            let far_range = 600.0;    // very light effect up to here
            let t_close = (1.0 - (dist_now / strong_range)).clamp(0.0, 1.0);
            let t_far = (1.0 - (dist_now / far_range)).clamp(0.0, 1.0);
            let t = (0.5 * t_far + 0.5 * t_close).clamp(0.0, 1.0);
            let perf_ok = dt <= (1.0 / 55.0) as f32;
            if perf_ok && t > 0.05 {
                // Single-pass lighter blur to reduce CPU cost
                let strength = (0.35 + 0.45 * t).min(0.8);
                let passes = 1;
                let radius = (0.60 + 0.25 * t).min(0.85);
                framebuffer.apply_circular_blur(strength, passes, radius);
            }
            // Flashlight overlay is drawn later to sit above the world

            // sprites depth-sorted
            let mut sprites: Vec<(&str, f32, f32, char, f32, f32)> = Vec::new();
            for (_idx, o) in orbs.iter().enumerate().filter(|(_,o)| o.active).map(|(i,o)|(i,o)) {
                // Orbs baseline at v_offset ~0.10
                sprites.push(("orb", o.x, o.y, 'o', 28.0, 0.10));
            }
            if cfg.enemy_enabled && enemy.active {
                // Enemy aligned at the same baseline as orbs for cohesion
                sprites.push(("enemy", enemy.x, enemy.y, 'N', 90.0, 0.10));
            }
            draw_sprites_sorted(&mut framebuffer, &player, &texman, &zbuffer, &mut sprites);
        }

    // HUD + MINIMAPA
    let fps_now = window.get_fps();
    // Transición a estado Caught cuando el enemigo te alcanza
    if matches!(game_state, GameState::Playing | GameState::Escaping) && cfg.enemy_enabled {
            let dx = enemy.x - player.pos.x;
            let dy = enemy.y - player.pos.y;
            if (dx*dx + dy*dy).sqrt() < 26.0 {
                game_state = GameState::Caught;
                if !caught_sfx_played {
                    if let Some(a) = audio.as_mut() { a.play_player_caught(); }
                    caught_sfx_played = true;
                }
            } else {
                caught_sfx_played = false;
            }
        }
        {
            // Capturar estado de WASD antes de pedir préstamo mutable de window para dibujar (evita conflicto)
            let wasd_state = (
                window.is_key_down(KeyboardKey::KEY_W),
                window.is_key_down(KeyboardKey::KEY_A),
                window.is_key_down(KeyboardKey::KEY_S),
                window.is_key_down(KeyboardKey::KEY_D),
            );
            let mut d = window.begin_drawing(&raylib_thread);
            d.clear_background(Color::BLACK);

            // Actualizar audio (no-op para rodio, placeholder)
            if let Some(a) = audio.as_ref() { a.update(); }
            // Subir framebuffer a textura y dibujar de un golpe (rápido)
            framebuffer.upload_to_texture(&mut fb_tex);
            // Escalar la textura low-res del framebuffer a la ventana completa
            let src = Rectangle { x: 0.0, y: 0.0, width: fb_tex.width() as f32, height: fb_tex.height() as f32 };
            let dst = Rectangle { x: 0.0, y: 0.0, width: window_width as f32, height: window_height as f32 };
            let origin = Vector2 { x: 0.0, y: 0.0 };
            d.draw_texture_pro(&fb_tex, src, dst, origin, 0.0, Color::WHITE);

            // Footsteps SFX solo cuando hay movimiento con WASD
            if let Some(a) = audio.as_mut() {
                let moving_keys = { let (w,a_key,s,d_key) = wasd_state; w || a_key || s || d_key };
                static mut WAS_MOVING: bool = false;
                static mut LAST_PX: f32 = 0.0;
                static mut LAST_PY: f32 = 0.0;
                static mut ACCUM: f32 = 0.0;
                unsafe {
                    if moving_keys {
                        let dx = player.pos.x - LAST_PX;
                        let dy = player.pos.y - LAST_PY;
                        let d = (dx*dx + dy*dy).sqrt();
                        ACCUM += d;
                        LAST_PX = player.pos.x; LAST_PY = player.pos.y;
                        if !WAS_MOVING {
                            // immediate first step on movement start
                            a.force_player_step();
                            ACCUM = 0.0;
                            WAS_MOVING = true;
                        } else {
                            let stride = if player.sprinting { 22.0 } else { 34.0 };
                            if ACCUM >= stride {
                                a.play_player_step(player.sprinting);
                                ACCUM -= stride;
                            }
                        }
                    } else {
                        WAS_MOVING = false;
                        ACCUM = 0.0;
                        LAST_PX = player.pos.x; LAST_PY = player.pos.y;
                        a.stop_player_steps(); // hard stop foot audio when idle
                    }
                }
                if enemy.active {
                    // Scale enemy step volume by distance (closer = louder)
                    let dx = enemy.x - player.pos.x;
                    let dy = enemy.y - player.pos.y;
                    let dist = (dx*dx + dy*dy).sqrt();
                    // Map distance 450..30 -> volume 0.25..1.7 (closer = much louder)
                    let vol = {
                        let t = (1.0 - ((dist - 30.0) / (450.0 - 30.0))).clamp(0.0, 1.0);
                        0.25 + t * 1.45
                    };
                    a.play_enemy_step_with_volume(vol);
                }
            }

            // Flashlight overlay (dibujar ANTES del HUD/minimapa para que la UI quede encima)
            {
                // Centro desplazado hacia delante + sacudida si te persigue/ve
                let look_dx = player.a.cos();
                let look_dy = player.a.sin();
                let offset_px = 90.0;           // how far to push the light forward
                // Determinar visibilidad para sacudida más fuerte y luz más cerrada
                let seen = enemy.sees_player(&maze, player.pos.x, player.pos.y, block_size);
                // Sacudida: aumenta al ser visto/en persecución y al estar cerca
                let chasing = enemy.is_chasing();
                let dxp = enemy.x - player.pos.x;
                let dyp = enemy.y - player.pos.y;
                let dist_now = (dxp*dxp + dyp*dyp).sqrt();
                let near_t = (1.0 - (dist_now / 500.0)).clamp(0.0, 1.0);
                // Base shake if seen; add more when chasing; plus proximity term
                let mut shake_amp = 0.0;
                if seen { shake_amp += 12.0; }
                if chasing { shake_amp += 8.0; }
                shake_amp += 10.0 * near_t;
                let ttime = d.get_time() as f32;
                let shake_x = (ttime * 29.0).sin() * shake_amp + (ttime * 21.0).cos() * (shake_amp * 0.55);
                let shake_y = (ttime * 31.0).sin() * (shake_amp * 0.9);
                let cx = (window_width as f32) * 0.5 + look_dx * offset_px + shake_x;
                let cy = (window_height as f32) * 0.5 + look_dy * (offset_px * 0.45) + shake_y;
                // Reducir radio al ser visto y cuando está más cerca
                let dx = enemy.x - player.pos.x;
                let dy = enemy.y - player.pos.y;
                let dist = (dx*dx + dy*dy).sqrt();
                let proximity = (1.0 - (dist / 600.0)).clamp(0.0, 1.0);
                // Make it darker: smaller base and min radius; stronger seen shrink
                let base_r = 300.0;     // much darker baseline
                let min_r = 140.0;      // much tighter minimum
                let t = if seen { (0.6 + 0.6 * proximity).clamp(0.0, 1.0) } else { 0.0 };
                let r0 = base_r * (1.0 - t) + min_r * t;
                let hw = (window_width as f32) * 0.5;
                let hh = (window_height as f32) * 0.5;
                let r_max = (hw*hw + hh*hh).sqrt() + 64.0; // asegurar esquinas cubiertas
                let segs: i32 = 96; // fewer segments for performance
                // Aplicar ~70% de oscuridad fuera del radio con borde suave
                let base_alpha: u8 = 178; // ~70% darkness (0.7 * 255)
                let feather: f32 = 36.0;  // slightly narrower feather for fewer ring draws
                let inner_soft_start = r0.max(0.0);
                let inner_soft_end = (r0 + feather).min(r_max);

                // 1) Borde suave: de 0 -> base_alpha en [r0 .. r0+feather]
                let steps = 6; // fewer steps to reduce draw calls
                for s in 0..steps {
                    let t0 = s as f32 / steps as f32;
                    let t1 = (s + 1) as f32 / steps as f32;
                    let ri = inner_soft_start + (inner_soft_end - inner_soft_start) * t0;
                    let ro = inner_soft_start + (inner_soft_end - inner_soft_start) * t1;
                    let a = ((base_alpha as f32) * t1).round().clamp(0.0, 255.0) as u8;
                    d.draw_ring(
                        Vector2 { x: cx, y: cy },
                        ri,
                        ro,
                        0.0,
                        360.0,
                        segs,
                        Color::new(0, 0, 0, a),
                    );
                }

                // 2) Sólido exterior: un anillo grande con ~70% de oscuridad
                if inner_soft_end < r_max {
                    d.draw_ring(
                        Vector2 { x: cx, y: cy },
                        inner_soft_end,
                        r_max,
                        0.0,
                        360.0,
                        segs,
                        Color::new(0, 0, 0, base_alpha),
                    );
                }
            }

            // Panic red tint overlay when seen or very near
            {
                let enemy_sees = enemy.sees_player(&maze, player.pos.x, player.pos.y, block_size);
                let dx = enemy.x - player.pos.x;
                let dy = enemy.y - player.pos.y;
                let dist = (dx*dx + dy*dy).sqrt();
                let near_t = (1.0 - (dist / 600.0)).clamp(0.0, 1.0);
                if enemy_sees || near_t > 0.0 {
                    // Blend intensity: stronger when seen, otherwise scale by proximity
                    let base = if enemy_sees { 110 } else { 0 };
                    let extra = (near_t * 120.0) as i32;
                    let raw = base + extra;
                    // 25% less intensity overall
                    let alpha = ((raw as f32) * 0.75).round().clamp(0.0, 180.0) as u8;
                    d.draw_rectangle(0, 0, window_width, window_height, Color::new(180, 10, 24, alpha));
                }
            }

            // HUD: simple FPS only
            d.draw_text(&format!("FPS: {}", fps_now), 10, 10, 20, Color::WHITE);
            // HUD pequeño: estado de audio y bandera "Seen"
            let audio_ok = if audio.is_some() { "Audio: OK" } else { "Audio: OFF" };
            d.draw_text(audio_ok, 10, 30, 18, Color::WHITE);
            if enemy.sees_player(&maze, player.pos.x, player.pos.y, block_size) {
                d.draw_text("Seen", 10, 50, 18, Color::RED);
            }
            if player.sprinting {
                d.draw_text("SPRINT", 10, 40, 20, Color::RED);
            }
            let remaining = orbs.iter().filter(|o| o.active).count();
            let bottom_y = window_height - 28;
            d.draw_text(&format!("Orbs: {} / {}", score, score + remaining), 10, bottom_y, 22, Color::WHITE);

            // Mensajes de estado
            match game_state {
                GameState::Escaping => {
                    let msg = "¡Todos los orbs! Busca la salida blanca (g).";
                    let tw = d.measure_text(msg, 22);
                    d.draw_text(msg, (window_width - tw)/2, 12, 22, Color::WHITE);
                }
                GameState::Won => {
                    // Style like the menu: red gradient + vignette + glowing text
                    for i in 0..window_height {
                        let t = i as f32 / window_height as f32;
                        let r = (24.0 + 120.0 * t) as u8;
                        d.draw_line(0, i, window_width, i, Color::new(r, 8, 16, 255));
                    }
                    let cx = (window_width as f32) * 0.5;
                    let cy = (window_height as f32) * 0.5;
                    for k in 0..6 {
                        let alpha = (24 + k * 12) as u8;
                        let inner = (window_height as f32 * (0.48 + k as f32 * 0.035)).min(window_height as f32);
                        let outer = inner + 22.0;
                        d.draw_ring(
                            Vector2 { x: cx, y: cy },
                            inner as f32,
                            outer as f32,
                            0.0,
                            360.0,
                            64,
                            Color::new(220, 20, 40, alpha),
                        );
                    }
                    let title = "You Escaped!";
                    let ts = 60;
                    let tw = d.measure_text(title, ts);
                    let tx = (window_width - tw)/2;
                    let ty = window_height/2 - 70;
                    for (ox, oy, col) in [(-2,0, Color::new(255,50,80,120)), (2,0, Color::new(255,50,80,120)), (0,2, Color::new(255,50,80,120)), (0,-2, Color::new(255,50,80,120))] {
                        d.draw_text(title, tx+ox, ty+oy, ts, col);
                    }
                    d.draw_text(title, tx, ty, ts, Color::new(255, 230, 210, 255));
                    let hint = "ENTER: next level | ESC: exit";
                    let hw = d.measure_text(hint, 28);
                    d.draw_text(hint, (window_width - hw)/2, ty + 90, 28, Color::new(240, 220, 220, 255));
                }
                GameState::Caught => {
                    let msg = "GAME OVER - Te atrapó (ENTER: menú, ESC: salir)";
                    let tw = d.measure_text(msg, 36);
                    d.draw_rectangle(0, 0, window_width, window_height, Color::new(0,0,0,200));
                    d.draw_text(msg, (window_width - tw)/2, window_height/2 - 18, 36, Color::RED);
                }
                _ => {}
            }

            // Minimap (arriba derecha) según nivel — dibujado después de la linterna para que permanezca visible
            if cfg.show_minimap {
                draw_minimap(&mut d, &maze, &player, &orbs, &enemy, window_width);
            }

            // (overlay de Caught ya manejado en el match anterior)
        }

    // Salir/avanzar en pantallas finales
        if game_state == GameState::Won && (window.is_key_pressed(KeyboardKey::KEY_ENTER) || window.is_key_pressed(KeyboardKey::KEY_KP_ENTER)) {
            // avanzar nivel y volver a menú
            selected_level = (selected_level + 1) % 3;
            game_state = GameState::Menu;
        }
        if game_state == GameState::Won && window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            break;
        }
        if game_state == GameState::Caught && window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            break;
        }

        // pacing por set_target_fps
    }
}
