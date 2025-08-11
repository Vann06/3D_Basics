use raylib::prelude::*;
use crate::player::Player;
use crate::maze::Maze;

fn cell_solid(maze: &Maze, block_size: usize, wx: f32, wy: f32) -> bool {
    let i = (wx / block_size as f32).floor() as isize;
    let j = (wy / block_size as f32).floor() as isize;
    if i < 0 || j < 0 { return true; }
    let (i, j) = (i as usize, j as usize);
    if j >= maze.len() || i >= maze[j].len() { return true; }
    let c = maze[j][i];
    // libres: espacio y goal; lo demás lo tratamos como sólido
    !(c == ' ' || c == 'g')
}

/// Verifica que el CÍRCULO del jugador (centro wx,wy y radio r) no penetre paredes.
/// Muestras 8 direcciones para “acolchonar” el contacto.
fn is_free_with_radius(maze: &Maze, block_size: usize, wx: f32, wy: f32, r: f32) -> bool {
    // posiciones a muestrear alrededor del centro
    let samples = [
        (wx, wy),
        (wx + r, wy),
        (wx - r, wy),
        (wx, wy + r),
        (wx, wy - r),
        (wx + r*0.7071, wy + r*0.7071),
        (wx - r*0.7071, wy + r*0.7071),
        (wx + r*0.7071, wy - r*0.7071),
        (wx - r*0.7071, wy - r*0.7071),
    ];
    for (sx, sy) in samples {
        if cell_solid(maze, block_size, sx, sy) {
            return false;
        }
    }
    true
}

pub fn process_events(
    window: &mut RaylibHandle,
    player: &mut Player,
    maze: &Maze,
    block_size: usize,
) {
    let dt = window.get_frame_time();
    let r = player.radius;
    player.update(window, dt, |x, y| is_free_with_radius(maze, block_size, x, y, r));
}
