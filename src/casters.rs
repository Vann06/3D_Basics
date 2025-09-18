use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;
use crate::line::line;
use raylib::prelude::*;

/// Ray march simple (pasos pequeños). Devuelve distancia al primer sólido.
/// Si `debug_draw` es true, dibuja el rayo en el framebuffer 2D.
pub fn cast_ray(
    fb: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    angle: f32,
    block_size: usize,
    debug_draw: bool,
) -> f32 {
    let step = 4.0f32; // tamaño de paso en unidades de mundo
    let mut d = 0.0f32;

    let dir = (angle.cos(), angle.sin());
    let max_dist = 2000.0;

    let mut hit = false;
    let (mut hx, mut hy) = (player.pos.x, player.pos.y);

    while d < max_dist {
        hx = player.pos.x + dir.0 * d;
        hy = player.pos.y + dir.1 * d;

        let i = (hx / block_size as f32).floor() as isize;
        let j = (hy / block_size as f32).floor() as isize;
        if i < 0 || j < 0 { break; }
        let (i,j)=(i as usize, j as usize);
        if j >= maze.len() || i >= maze[0].len() { break; }

        let c = maze[j][i];
        if c != ' ' {
            hit = true;
            break;
        }
        d += step;
    }

    if debug_draw {
        fb.set_current_color(Color::WHITE);
        line(
            fb,
            player.pos.x as i32,
            player.pos.y as i32,
            hx as i32,
            hy as i32
        );
    }

    if !hit { return 0.0; }

    // corrección de "fish-eye"
    let diff = angle - player.a;
    let d_corr = d * diff.cos().abs();

    d_corr.max(1.0)
}
