use raylib::prelude::*;

/// Player: FPS-like, con radio para colisiones.
pub struct Player {
    pub pos: Vector2,
    pub a: f32,         // yaw en radianes (render usa esto)
    pub fov: f32,

    pub sprinting: bool,

    pub speed_walk: f32,
    pub speed_sprint: f32,
    pub mouse_sens: f32,

    pub radius: f32,    // 🔴 radio de colisión en unidades de mundo (px)
}

impl Player {
    pub fn new(x: f32, y: f32, angle: f32) -> Self {
        Self {
            pos: Vector2::new(x, y),
            a: angle,
            // Puedes cambiar a 60° si prefieres: std::f32::consts::FRAC_PI_3
            fov: std::f32::consts::FRAC_PI_2,

            // 🔸 velocidades más realistas para tu escala de mapa (BLOCK=64)
            speed_walk: 50.0,
            speed_sprint: 100.0,
            mouse_sens: 0.0032,

            sprinting: false,
            // 🔴 ~1/5 del tamaño de celda: “acolchonado” contra paredes
            radius: 12.0,
        }
    }

    /// `is_free(wx, wy)` debe validar SI el círculo con centro (wx,wy) cabe sin tocar pared.
    pub fn update<F>(&mut self, rl: &mut RaylibHandle, dt: f32, is_free: F)
    where
        F: Fn(f32, f32) -> bool,
    {
        // 1) Mouse look
        let md = rl.get_mouse_delta();
        self.a += md.x * self.mouse_sens;
        if self.a > std::f32::consts::PI { self.a -= 2.0 * std::f32::consts::PI; }
        else if self.a < -std::f32::consts::PI { self.a += 2.0 * std::f32::consts::PI; }

        // 2) Direcciones (WASD)
        let fwd_x = self.a.cos();
        let fwd_y = self.a.sin();
        let right_x = -fwd_y;
        let right_y =  fwd_x;

        let mut dir_x = 0.0;
        let mut dir_y = 0.0;
        if rl.is_key_down(KeyboardKey::KEY_W) { dir_x += fwd_x; dir_y += fwd_y; }
        if rl.is_key_down(KeyboardKey::KEY_S) { dir_x -= fwd_x; dir_y -= fwd_y; }
        if rl.is_key_down(KeyboardKey::KEY_D) { dir_x += right_x; dir_y += right_y; }
        if rl.is_key_down(KeyboardKey::KEY_A) { dir_x -= right_x; dir_y -= right_y; }

        let len = (dir_x*dir_x + dir_y*dir_y).sqrt();
        if len > 0.0001 { dir_x /= len; dir_y /= len; }

        // 3) Sprint (mantener Shift)
        self.sprinting = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        let speed = if self.sprinting { self.speed_sprint } else { self.speed_walk };

        let move_x = dir_x * speed * dt;
        let move_y = dir_y * speed * dt;

        // 4) Colisión con slide por ejes
        let try_move_x = self.pos.x + move_x;
        if is_free(try_move_x, self.pos.y) {
            self.pos.x = try_move_x;
        }
        let try_move_y = self.pos.y + move_y;
        if is_free(self.pos.x, try_move_y) {
            self.pos.y = try_move_y;
        }
    }
}
