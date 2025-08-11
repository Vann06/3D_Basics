use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;
use crate::casters::cast_ray;
use raylib::prelude::*;

pub fn render_3d(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    block_size: usize,
    player: &Player,
) {
    let num_rays = framebuffer.width;
    let hh = framebuffer.height as f32 / 2.0;

    framebuffer.set_current_color(Color::RED);

    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32;
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        let d = cast_ray(framebuffer, maze, player, a, block_size, false);

        if d > 0.0 {
            let stake_height = (hh / d) * 100.0;
            let half_stake_height = stake_height / 2.0;

            let stake_top = (hh - half_stake_height).max(0.0) as u32;
            let stake_bottom = (hh + half_stake_height).min(framebuffer.height as f32) as u32;

            for y in stake_top..stake_bottom {
                let tx = intersect.tx as u32;

                let ty = ((y as f32 - stake_top as f32) / (stake_bottom - stake_top) as f32 * 64.0) as u32;

                let color = texman.get_pixel_color(intersect.impact, tx, ty);

                framebuffer.set_current_color(color);
                framebuffer.set_pixel(i, y);
            }

        }
    }
}
