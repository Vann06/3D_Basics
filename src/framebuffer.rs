use raylib::prelude::*;
use raylib::core::texture::RaylibTexture2D; // ← importa el trait para usar .update_texture()

pub struct Framebuffer {
    pub color_buffer: Vec<Color>,
    pub width: u32,
    pub height: u32,
    pub background_color: Color,
    pub current_color: Color,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let bg = Color::BLACK;
        Self {
            color_buffer: vec![bg; size],
            width,
            height,
            background_color: bg,
            current_color: Color::WHITE,
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.color_buffer.fill(self.background_color);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            self.color_buffer[(y * self.width + x) as usize] = self.current_color;
        }
    }

    #[inline]
    pub fn set_pixel_color(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            self.color_buffer[(y * self.width + x) as usize] = color;
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x < self.width && y < self.height {
            return self.color_buffer[(y * self.width + x) as usize];
        }
        self.background_color
    }

    #[inline] pub fn set_current_color(&mut self, c: Color) { self.current_color = c; }
    #[inline] pub fn set_background_color(&mut self, c: Color) { self.background_color = c; }

    /// Sube los píxeles a una textura *persistente* (¡ahora el método es de `Texture2D`!).
    pub fn upload_to_texture(&self, tex: &mut Texture2D) {
        // Convertimos &[Color] → &[u8] (RGBA8) sin copiar:
        let byte_len = self.color_buffer.len() * std::mem::size_of::<Color>();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(self.color_buffer.as_ptr() as *const u8, byte_len)
        };
    let _ = tex.update_texture(bytes);
    }

    /// Aplica un blur ligero (ansiedad) mezclando cada pixel con vecinos inmediatos.
    /// strength 0..1 controla cuánto se acerca al promedio; passes repite el efecto.
    pub fn apply_anxiety_blur(&mut self, strength: f32, passes: u32) {
        if strength <= 0.0 { return; }
        let s = strength.clamp(0.0, 1.0);
        let w = self.width as i32;
        let h = self.height as i32;
        let mut tmp: Vec<Color> = self.color_buffer.clone();
        for _ in 0..passes.min(3) { // máximo 3 pasadas para no degradar demasiado
            // intercambiar buffers (leer de color_buffer, escribir en tmp)
            for y in 1..h-1 {
                let ym = (y-1) as u32;
                let y0 = y as u32;
                let yp = (y+1) as u32;
                for x in 1..w-1 {
                    let xm = (x-1) as u32;
                    let x0 = x as u32;
                    let xp = (x+1) as u32;
                    let c  = self.get_pixel(x0,y0);
                    let c1 = self.get_pixel(xm,y0);
                    let c2 = self.get_pixel(xp,y0);
                    let c3 = self.get_pixel(x0,ym);
                    let c4 = self.get_pixel(x0,yp);
                    let avg_r = (c.r as u32 + c1.r as u32 + c2.r as u32 + c3.r as u32 + c4.r as u32) / 5;
                    let avg_g = (c.g as u32 + c1.g as u32 + c2.g as u32 + c3.g as u32 + c4.g as u32) / 5;
                    let avg_b = (c.b as u32 + c1.b as u32 + c2.b as u32 + c3.b as u32 + c4.b as u32) / 5;
                    let lerp = |a: u8, b: u32| -> u8 { ( (a as f32) * (1.0 - s) + (b as f32) * s ) as u8 };
                    let out = Color::new(lerp(c.r, avg_r), lerp(c.g, avg_g), lerp(c.b, avg_b), c.a);
                    tmp[(y0 * self.width + x0) as usize] = out;
                }
            }
            std::mem::swap(&mut self.color_buffer, &mut tmp);
        }
    }

    /// Aplica una viñeta oscura leve para reforzar ansiedad.
    pub fn apply_vignette(&mut self, intensity: f32) {
        let k = intensity.clamp(0.0, 1.0);
        if k <= 0.0 { return; }
        let w = self.width as f32;
        let h = self.height as f32;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let max_r = (cx*cx + cy*cy).sqrt();
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let d = (dx*dx + dy*dy).sqrt();
                let t = (d / max_r).clamp(0.0, 1.0);
                // Curva que solo oscurece bordes
                let fade = (t.powf(2.0)).min(1.0);
                if fade > 0.2 { // evita centro
                    let idx = (y * self.width + x) as usize;
                    let c = self.color_buffer[idx];
                    let dark = 1.0 - k * (fade - 0.2);
                    let mul = |v: u8| -> u8 { (v as f32 * dark).clamp(0.0,255.0) as u8 };
                    self.color_buffer[idx] = Color::new(mul(c.r), mul(c.g), mul(c.b), c.a);
                }
            }
        }
    }

    /// Blur circular (enmascarado): aplica el blur solo dentro de un círculo centrado.
    /// radius_ratio: 0..1, radio relativo al semimenor (min(width,height)/2). Ej: 0.5 ≈ mitad de la pantalla.
    pub fn apply_circular_blur(&mut self, strength: f32, passes: u32, radius_ratio: f32) {
        if strength <= 0.0 { return; }
        let s = strength.clamp(0.0, 1.0);
        let w = self.width as i32;
        let h = self.height as i32;
        let mut tmp: Vec<Color> = self.color_buffer.clone();
        let cx = (self.width as f32) * 0.5;
        let cy = (self.height as f32) * 0.5;
        let r_base = (self.width.min(self.height) as f32) * 0.5 * radius_ratio.clamp(0.05, 1.0);
        let r2 = r_base * r_base;
        for _ in 0..passes.min(2) { // 1-2 pasadas para costo bajo
            for y in 1..h-1 {
                let y0 = y as u32;
                let ym = (y-1) as u32;
                let yp = (y+1) as u32;
                for x in 1..w-1 {
                    let x0 = x as u32;
                    // Solo dentro del círculo
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    if dx*dx + dy*dy > r2 { continue; }
                    let xm = (x-1) as u32;
                    let xp = (x+1) as u32;
                    let c  = self.get_pixel(x0,y0);
                    let c1 = self.get_pixel(xm,y0);
                    let c2 = self.get_pixel(xp,y0);
                    let c3 = self.get_pixel(x0,ym);
                    let c4 = self.get_pixel(x0,yp);
                    let avg_r = (c.r as u32 + c1.r as u32 + c2.r as u32 + c3.r as u32 + c4.r as u32) / 5;
                    let avg_g = (c.g as u32 + c1.g as u32 + c2.g as u32 + c3.g as u32 + c4.g as u32) / 5;
                    let avg_b = (c.b as u32 + c1.b as u32 + c2.b as u32 + c3.b as u32 + c4.b as u32) / 5;
                    let lerp = |a: u8, b: u32| -> u8 { ((a as f32) * (1.0 - s) + (b as f32) * s) as u8 };
                    let out = Color::new(lerp(c.r, avg_r), lerp(c.g, avg_g), lerp(c.b, avg_b), c.a);
                    tmp[(y0 * self.width + x0) as usize] = out;
                }
            }
            std::mem::swap(&mut self.color_buffer, &mut tmp);
        }
    }
}
