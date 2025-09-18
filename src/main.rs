#![allow(unused_imports)]
#![allow(dead_code)]

mod line;
mod framebuffer;
mod maze;
mod player;
mod process_events;
mod casters;
mod textures;
mod render3d;
mod enemy;
mod audio_manager;

use textures::TextureManager;
use raylib::prelude::*;
use audio_manager::AudioManager;
use std::thread;
use std::time::Duration;
use framebuffer::Framebuffer;
use maze::{Maze, load_maze};
use player::Player;
use process_events::process_events;
use casters::cast_ray;
use render3d::{render_3d, draw_sprite_world, draw_sprites_sorted};
use rand::seq::SliceRandom;
use enemy::Enemy;
use std::path::Path;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GameState { Menu, Playing, Escaping, Won, Caught }

// Simplified menu: just "Play". We keep enum stub if needed.
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
    // L3: sin minimapa; brillo más fuerte
    2 => LevelCfg { file: "maze3.txt", enemy_enabled: true,  show_minimap: false, brightness: 1.30 },
    _ => LevelCfg { file: "maze1.txt", enemy_enabled: true,  show_minimap: true,  brightness: 1.0 },
    }
}

// Tamaño de celda en unidades de mundo
pub const BLOCK: f32 = 64.0;

// ---------- ORBS ----------
struct Orb { x: f32, y: f32, active: bool }

fn is_free_cell(maze: &maze::Maze, i: usize, j: usize) -> bool {
    if j >= maze.len() || i >= maze[j].len() { return false; }
    let c = maze[j][i];
    c == ' ' || c == 'g'
}
fn is_safe_cell(maze: &maze::Maze, i: usize, j: usize) -> bool {
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
fn spawn_orbs_in_empty_cells(maze: &maze::Maze, block: f32, count: usize) -> Vec<Orb> {
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

    // textura persistente para blitear el framebuffer cada frame
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
    // Preload teto.gif texture for menu (single frame; GIF animation not handled)
    let tex_teto = Image::load_image("assets/teto.gif")
        .ok()
        .and_then(|img| window.load_texture_from_image(&raylib_thread, &img).ok());

    let mut zbuffer = vec![f32::INFINITY; framebuffer.width as usize];
    let mode_3d = true;
    let mut game_state = GameState::Menu;
    // Simplified menu: Enter starts next level; no menu index needed

    // para delta time
    let mut last_time = window.get_time();

    while !window.window_should_close() {
        // dt
    let now = window.get_time();
    let dt = (now - last_time) as f32;
    last_time = now;

    // Menu input & drawing
    let mut touched_exit = false;
    if matches!(game_state, GameState::Menu) {
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) || window.is_key_pressed(KeyboardKey::KEY_KP_ENTER) {
            let start_idx = selected_level.clamp(0, 2);
            cfg = level_cfg(start_idx);
            maze = load_maze(cfg.file);
            let (o, s, p, e) = reset_game(&maze, block_size);
            orbs = o; score = s; player = p; enemy = e;
            enemy.active = false;
            enemy_spawn_timer = if start_idx == 0 { 0.5 } else { 12.0 };
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
                        // buscar celda libre lejana
                        let mut best: Option<(usize,usize,f32)> = None;
                        for (j,row) in maze.iter().enumerate() {
                            for (i,&c) in row.iter().enumerate() {
                                if c == ' ' {
                                    let wx = (i as f32 + 0.5) * BLOCK;
                                    let wy = (j as f32 + 0.5) * BLOCK;
                                    let dx = wx - player.pos.x; let dy = wy - player.pos.y;
                                    let d2 = dx*dx + dy*dy;
                                    // al menos 10-12 celdas de distancia
                                    if d2 > 10.0*BLOCK*10.0*BLOCK {
                                        if best.map(|b| d2 > b.2).unwrap_or(true) {
                                            best = Some((i,j,d2));
                                        }
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
            // Menu screen: retro look (single Play that cycles levels)
            let mut d = window.begin_drawing(&raylib_thread);
            d.clear_background(Color::BLACK);
            // Title
            let title = "Teto Escape";
            let ts = 64;
            let tw = d.measure_text(title, ts);
            d.draw_text(title, (window_width - tw)/2 - 150, 60, ts, Color::new(255, 240, 120, 255));
            // Single Play option with preview of next level
            let base_x = 120; let base_y = 240;
            let label = format!("Play (Level {})", (selected_level % 3) + 1);
            d.draw_text(&label, base_x, base_y, 40, Color::YELLOW);
            d.draw_text("ENTER: Play  |  ESC: Exit", base_x, base_y + 56, 22, Color::new(200,200,200,255));
            // Right panel for teto.gif with slight bobbing animation
            let panel_x = (window_width as f32 * 0.60) as i32;
            d.draw_rectangle(panel_x, 0, window_width - panel_x, window_height, Color::new(16, 16, 24, 255));
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
                    Color::WHITE);
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
            // sin tinte verde en el enemigo cuando persigue

            // Reemplaza esta llamada:
            // render_3d(&mut framebuffer, &maze, block_size, &player, &texman, &mut zbuffer);

            // Por esta:
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
            // Removed framebuffer vignette; flashlight overlay is drawn later on top

            // sprites depth-sorted
            let mut sprites: Vec<(&str, f32, f32, char, f32, f32)> = Vec::new();
            for (_idx, o) in orbs.iter().enumerate().filter(|(_,o)| o.active).map(|(i,o)|(i,o)) {
                sprites.push(("orb", o.x, o.y, 'o', 28.0, 0.10));
            }
            if cfg.enemy_enabled && enemy.active {
                // Siempre usar la imagen frontal (enemy_n.png)
                // Bigger enemy so it fills better
                sprites.push(("enemy", enemy.x, enemy.y, 'N', 90.0, 0.08));
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
            // Capture WASD state before borrowing window mutably for drawing (avoids borrow conflict)
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
            // Subir framebuffer a textura y dibujar de un golpe (mucho más rápido)
            framebuffer.upload_to_texture(&mut fb_tex);
            // Scale the low-res framebuffer texture to the full window
            let src = Rectangle { x: 0.0, y: 0.0, width: fb_tex.width() as f32, height: fb_tex.height() as f32 };
            let dst = Rectangle { x: 0.0, y: 0.0, width: window_width as f32, height: window_height as f32 };
            let origin = Vector2 { x: 0.0, y: 0.0 };
            d.draw_texture_pro(&fb_tex, src, dst, origin, 0.0, Color::WHITE);

            // Footsteps SFX triggers strictly from WASD key movement
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

            // Flashlight-like overlay (draw BEFORE HUD/minimap so UI stays on top)
            {
                // Center offset forward + camera shake when chased/seen
                let look_dx = player.a.cos();
                let look_dy = player.a.sin();
                let offset_px = 90.0;           // how far to push the light forward
                // Determine visibility early for stronger shake & tighter light
                let seen = enemy.sees_player(&maze, player.pos.x, player.pos.y, block_size);
                // Shake: amplitude increases a lot when seen/chasing, and when very near
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
                // shrink radius when seen and when closer
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
                let r_max = (hw*hw + hh*hh).sqrt() + 64.0; // ensure corners fully covered
                let segs: i32 = 96; // fewer segments for performance
                // Apply a consistent 70% darkness outside the flashlight radius with a soft edge
                let base_alpha: u8 = 178; // ~70% darkness (0.7 * 255)
                let feather: f32 = 36.0;  // slightly narrower feather for fewer ring draws
                let inner_soft_start = r0.max(0.0);
                let inner_soft_end = (r0 + feather).min(r_max);

                // 1) Soft edge: ramp up from 0 -> base_alpha across [r0 .. r0+feather]
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

                // 2) Solid outside: a single large ring at exactly 70% darkness
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

            // HUD
            let real_fps = if dt > 0.000_01 { (1.0 / dt).round() as i32 } else { fps_now as i32 };
            let ms = (dt * 1000.0).max(0.0);
            d.draw_text(&format!("FPS: {}  |  real: {}  |  {:.1} ms", fps_now, real_fps, ms), 10, 10, 20, Color::WHITE);
            // Tiny HUD diagnostics: audio status and seen flag
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
                    let msg = "¡Has escapado! ENTER: siguiente nivel | ESC: salir";
                    let tw = d.measure_text(msg, 36);
                    d.draw_rectangle(0, 0, window_width, window_height, Color::new(0,0,0,160));
                    d.draw_text(msg, (window_width - tw)/2, window_height/2 - 18, 36, Color::YELLOW);
                }
                GameState::Caught => {
                    let msg = "GAME OVER - Te atrapó (ENTER: menú, ESC: salir)";
                    let tw = d.measure_text(msg, 36);
                    d.draw_rectangle(0, 0, window_width, window_height, Color::new(0,0,0,200));
                    d.draw_text(msg, (window_width - tw)/2, window_height/2 - 18, 36, Color::RED);
                }
                _ => {}
            }

            // minimap (arriba derecha) según nivel — drawn after flashlight so it stays visible
            if cfg.show_minimap {
                draw_minimap(&mut d, &maze, &player, &orbs, &enemy, window_width);
            }

            // (overlay de Caught ya manejado en el match anterior)
        }

        // salir/avanzar en pantallas finales
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
