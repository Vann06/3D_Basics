use raylib::prelude::*;
use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;
use crate::textures::TextureManager;

pub struct Intersect {
    pub distance: f32,
    pub impact: char,
    pub tx: usize,
}
pub fn cast_ray(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    a: f32,
    block_size: usize,
    draw: bool,
) -> Intersect {
    let mut d = 0.0;
    framebuffer.set_current_color(Color::WHITE);

    loop {
        let cos = d * a.cos();
        let sin = d * a.sin();

        let x = (player.pos.x + cos) as usize;
        let y = (player.pos.y + sin) as usize;

        let i = x / block_size;
        let j = y / block_size;

        if maze[j][i] != ' ' {
            let hitx = x - i * block_size;
            let hity = y - j * block_size;
            let mut maxhit = hity;
            
            if 1 < hitx && hitx < block_size - 1 {
                maxhit = hitx;
            }

            let tx = ((maxhit as f32 * 128.0) / block_size as f32) as usize;
            return Intersect{
                distance: d,
                impact: maze[j][i],
                tx: tx,
            };
        }
        

        if draw {
            framebuffer.set_pixel(x as u32, y as u32);
        }

        d += 1.0;
    }
}


pub fn render_3d(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    block_size: usize,
    player: &Player,
    texman: &TextureManager,
) {
    let num_rays = framebuffer.width;
    let hh = framebuffer.height as f32 / 2.0;

    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32;
        let ray_angle = player.a - (player.fov / 2.0) + (player.fov * current_ray);

        let intersect = cast_ray(framebuffer, maze, player, ray_angle, block_size, false);
        let d = intersect.distance;
        let impact = intersect.impact;
        let tex_x = intersect.tx;

        let angle_diff = ray_angle - player.a;
        let corrected_distance = d * angle_diff.cos();

        if corrected_distance == 0.0 {
            continue;
        }

        let stake_height = (hh / corrected_distance) * 100.0;
        let half_stake_height = stake_height / 2.0;
        let stake_top = (hh - half_stake_height).max(0.0) as u32;
        let stake_bottom = (hh + half_stake_height).min(framebuffer.height as f32) as u32;

        for y in stake_top..stake_bottom {
            let relative_y = (y - stake_top) as f32 / (stake_bottom - stake_top) as f32;
            let tex_y = (relative_y * 64.0) as u32;

            let color = texman.get_pixel_color(impact, tex_x as u32, tex_y);
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(i, y);
        }
    }
}
