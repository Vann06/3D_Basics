// main.rs
#![allow(unused_imports)]
#![allow(dead_code)]

mod line;
mod framebuffer;
mod maze;
mod player;
mod process_events;
mod casters;
mod textures;

use textures::TextureManager;
use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use framebuffer::Framebuffer;
use maze::{Maze, load_maze};
use player::Player;
use process_events::process_events;
use casters::{cast_ray, render_3d};

// Tamaño de celda en unidades de mundo (coherente con colisiones)
pub const BLOCK: f32 = 64.0;

fn draw_cell(
    framebuffer: &mut Framebuffer,
    xo: usize,
    yo: usize,
    block_size: usize,
    cell: char,
) {
    if cell == ' ' {
        return;
    }
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

fn main() {
    let window_width = 1300;
    let window_height = 900;

    // ✅ ahora colisiones = mundo
    let block_size = BLOCK as usize;

    let (mut window, raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raycaster Example")
        .build();

    // ✅ Capturar/ocultar cursor para mouse look
    window.disable_cursor();

    // No hace falta mut aquí
    let texman = TextureManager::new(&mut window, &raylib_thread);
    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);
    framebuffer.set_background_color(Color::new(50, 50, 100, 255));

    let maze = load_maze("maze.txt");

    // Posición inicial: centro de la celda (1.5 * BLOCK, 1.5 * BLOCK)
    let mut player = Player::new(1.5 * BLOCK, 1.5 * BLOCK, 0.0);

    // Modo de visualización: 2D (top-down) o 3D (raycaster)
    let mut mode_3d = false;

    while !window.window_should_close() {
        // Cambiar modo al presionar M
        if window.is_key_pressed(KeyboardKey::KEY_M) {
            mode_3d = !mode_3d;
        }

        // ✅ Proceso de entrada (WASD + mouse + sprint con Shift)
        process_events(&mut window, &mut player, &maze, block_size);

        framebuffer.clear();

        if !mode_3d {
            // ---------- Vista 2D ----------
            render_maze(&mut framebuffer, &maze, block_size);

            // Jugador como punto amarillo
            framebuffer.set_current_color(Color::YELLOW);
            framebuffer.set_pixel(player.pos.x as u32, player.pos.y as u32);

            // Rayos de FOV solo como debug visual
            framebuffer.set_current_color(Color::WHITE);
            let num_rays = 25;
            for i in 0..num_rays {
                let t = i as f32 / num_rays as f32;
                let ray_angle = player.a - (player.fov / 2.0) + (player.fov * t);
                cast_ray(&mut framebuffer, &maze, &player, ray_angle, block_size, true);
            }
        } else {
            // ---------- Vista 3D ----------
            render_3d(&mut framebuffer, &maze, block_size, &player, &texman);
        }

        // ---- HUD + Dibujado en un solo begin_drawing ----
        // Precapturamos todo lo que use `window` **antes** de mutarlo con begin_drawing
        let fps_now = window.get_fps();
        let sprint_on = player.sprinting;

        {
            let mut d = window.begin_drawing(&raylib_thread);

            // Pintar el fondo (por si tu framebuffer no cubre toda la ventana)
            d.clear_background(Color::BLACK);

            // Dibujar el framebuffer en pantalla
            for y in 0..framebuffer.height {
                for x in 0..framebuffer.width {
                    let color = framebuffer.color_buffer[(y * framebuffer.width + x) as usize];
                    if color != framebuffer.background_color {
                        d.draw_pixel(x as i32, y as i32, color);
                    }
                }
            }

            // FPS en la esquina superior izquierda
            d.draw_text(&format!("FPS: {}", fps_now), 10, 10, 20, Color::WHITE);

            // Indicador SPRINT si está activo
            if sprint_on {
                d.draw_text("SPRINT", 10, 40, 20, Color::RED);
            }
        }

        // ~60 FPS (16 ms)
        thread::sleep(Duration::from_millis(16));
    }
}
