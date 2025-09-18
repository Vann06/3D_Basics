use raylib::prelude::*;

pub struct Player {
    pub pos: Vector2,
    pub a: f32,            // ángulo (yaw)
    pub fov: f32,          // campo de visión
    pub speed_walk: f32,
    pub speed_sprint: f32,
    pub mouse_sens: f32,
    pub sprinting: bool,
}

impl Player {
    pub fn new(x: f32, y: f32, angle: f32) -> Self {
        Self {
            pos: Vector2::new(x,y),
            a: angle,
            fov: std::f32::consts::FRAC_PI_2, // 90°
            speed_walk: 200.0,   // ↑ más rápido
            speed_sprint: 340.0, // ↑ más rápido
            mouse_sens: 0.0025,
            sprinting: false,
        }
    }
}
