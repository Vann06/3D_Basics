use raylib::prelude::*;
use crate::player::Player;
use crate::maze::Maze;

fn is_free(map: &Maze, block: usize, wx: f32, wy: f32) -> bool {
    let i = (wx / block as f32).floor() as isize;
    let j = (wy / block as f32).floor() as isize;
    if i < 0 || j < 0 { return false; }
    let (i,j)=(i as usize, j as usize);
    if j >= map.len() || i >= map[0].len() { return false; }
    let c = map[j][i];
    c == ' '
}

// helper: ¿la celda en (wx,wy) es la salida?
fn is_exit(map: &Maze, block: usize, wx: f32, wy: f32) -> bool {
    let i = (wx / block as f32).floor() as isize;
    let j = (wy / block as f32).floor() as isize;
    if i < 0 || j < 0 { return false; }
    let (i,j)=(i as usize, j as usize);
    if j >= map.len() || i >= map[0].len() { return false; }
    map[j][i] == 'g'
}

pub fn process_events(
    rl: &mut RaylibHandle,
    player: &mut Player,
    maze: &Maze,
    block: usize,
) -> bool {
    // rotación con mouse
    let md = rl.get_mouse_delta();
    player.a += md.x * player.mouse_sens;
    if player.a >  std::f32::consts::PI { player.a -= 2.0*std::f32::consts::PI; }
    if player.a < -std::f32::consts::PI { player.a += 2.0*std::f32::consts::PI; }

    // WASD
    let fwd = (player.a.cos(), player.a.sin());
    let right = (-fwd.1, fwd.0);

    let mut dir = (0.0f32, 0.0f32);
    if rl.is_key_down(KeyboardKey::KEY_W) { dir.0 += fwd.0; dir.1 += fwd.1; }
    if rl.is_key_down(KeyboardKey::KEY_S) { dir.0 -= fwd.0; dir.1 -= fwd.1; }
    if rl.is_key_down(KeyboardKey::KEY_D) { dir.0 += right.0; dir.1 += right.1; }
    if rl.is_key_down(KeyboardKey::KEY_A) { dir.0 -= right.0; dir.1 -= right.1; }

    let len = (dir.0*dir.0 + dir.1*dir.1).sqrt();
    if len > 0.0001 { dir.0/=len; dir.1/=len; }

    // sprint
    let dt = rl.get_frame_time();
    let sprint_pressed = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
    player.sprinting = sprint_pressed && len>0.0;

    let speed = if player.sprinting { player.speed_sprint } else { player.speed_walk };
    let dx = dir.0 * speed * dt;
    let dy = dir.1 * speed * dt;

    let mut touched_exit = false;

    // colisión separada por ejes (slide) + detección de salida
    let newx = player.pos.x + dx;
    if is_exit(maze, block, newx, player.pos.y) { touched_exit = true; }
    if is_free(maze, block, newx, player.pos.y) { player.pos.x = newx; }

    let newy = player.pos.y + dy;
    if is_exit(maze, block, player.pos.x, newy) { touched_exit = true; }
    if is_free(maze, block, player.pos.x, newy) { player.pos.y = newy; }

    touched_exit
}
