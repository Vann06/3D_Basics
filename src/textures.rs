use raylib::prelude::*;
use std::collections::HashMap;

/// Un pixmap inmutable (CPU) para samplear por pixel sin &mut Image.
#[derive(Clone)]
struct Pixmap {
    w: u32,
    h: u32,
    px: Vec<Color>,
}
impl Pixmap {
    fn new(w: u32, h: u32, px: Vec<Color>) -> Self { Self { w, h, px } }
    #[inline]
    fn sample(&self, x: u32, y: u32) -> Color {
        let xi = (x % self.w) as usize;
        let yi = (y % self.h) as usize;
        self.px[(yi * self.w as usize) + xi]
    }
}

pub struct TextureManager {
    maps: HashMap<char, Pixmap>,        // CPU pixmaps por clave-char
    textures: HashMap<char, Texture2D>, // opcional: GPU (no imprescindibles)
    alert_mode: bool,                   // si true, la pared '|' cambia a rojo
}

impl TextureManager {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut tm = Self {
            maps: HashMap::new(),
            textures: HashMap::new(),
            alert_mode: false,
        };

        // Candidatos a cargar de assets (si existe archivo lo usamos; si no, fallback procedural)
        let candidates: &[(&str, char)] = &[
            // Walls
            ("assets/wall1.png", '1'), ("wall1.png", '1'), ("./wall1.png", '1'), ("assets/walls/wall1.png", '1'),
            ("assets/wall2.png", '2'), ("wall2.png", '2'), ("./wall2.png", '2'), ("assets/walls/wall2.png", '2'),
            ("assets/wall3.png", '3'), ("wall3.png", '3'), ("./wall3.png", '3'), ("assets/walls/wall3.png", '3'),
            ("assets/wall4.png", '4'), ("wall4.png", '4'), ("./wall4.png", '4'), ("assets/walls/wall4.png", '4'),
            ("assets/goal.png",  'g'),
            ("assets/orb.png",   'o'),

            // Sky / Ground (repo-style names supported)
            ("assets/sky.png",      'K'),
            ("assets/skybox.png",   'K'),
            ("assets/ceiling.png",  'K'),
            ("assets/center.png",   'K'),
            ("assets/ground.png",   'G'),
            ("assets/floor.png",    'G'),

            // enemigo por orientación:
            ("assets/enemy_n.png", 'N'),
            ("assets/enemy_e.png", 'E'),
            ("assets/enemy_s.png", 'S'),
            ("assets/enemy_w.png", 'W'),

            // Alternate filenames from external repo
            ("assets/enemy.png", 'N'),
            ("assets/enemyy.png", 'N'),
            ("assets/enemy2.png", 'N'),
            ("assets/puffle.png", 'o'),
            ("assets/key.png", 'o'),
            // legacy aliases
            ("assets/center.png", '+'),
            ("assets/ground.png", '#'),
            ("assets/iglo.png", '4'),
        ];

        for (path, key) in candidates {
            if let Ok(img) = Image::load_image(path) {
                if let Ok(tex) = rl.load_texture_from_image(thread, &img) {
                    tm.textures.insert(*key, tex);
                }
                let w = img.width().max(1) as u32;
                let h = img.height().max(1) as u32;
                let data = img.get_image_data().to_vec(); // Vec<Color>
                tm.maps.insert(*key, Pixmap::new(w, h, data));
            }
        }

        // Fallbacks si faltan
    let fallbacks: &[char] = &['K', 'G', '+', '-', '|', '#', '1', '2', '3', '4', 'g', 'o', 'N', 'E', 'S', 'W'];
        for &k in fallbacks {
            if !tm.maps.contains_key(&k) {
                let pm = match k {
                    // Sky fallback (soft gradient)
                    'K' => {
                        let w = 256; let h = 128;
                        let mut px = vec![Color::BLACK; (w*h) as usize];
                        let top = Color::new(12,16,26,255);
                        let mid = Color::new(20,28,44,255);
                        for y in 0..h {
                            let t = y as f32 / (h-1) as f32;
                            let col = Self::mix(top, mid, (t*255.0) as u8);
                            for x in 0..w { px[(y*w + x) as usize] = col; }
                        }
                        Pixmap::new(w as u32, h as u32, px)
                    }
                    // Ground fallback (checker)
                    'G' => Self::make_checker_pixmap(128, 128, Color::new(48,48,52,255)),
                    // Pared tipo "pool rooms": franjas brillantes arriba/abajo
                    '|' | '-' | '+' => {
                        // Try to alias to '1' (wall1) if loaded; otherwise pool wall fallback
                        if let Some(pm) = tm.maps.get(&'1').cloned() { pm } else { Self::make_pool_wall(64, 64, false) }
                    },

                    // Goal checker verde
                    'g' => Self::make_checker_pixmap(64, 64, Color::new(30, 160, 30, 255)),

                    // Orb brillante
                    'o' => Self::make_glowing_orb(64, 64, Color::new(255, 240, 80, 255)),

                    // Enemigo de fallback (colores por orientación)
                    'N' => Self::make_enemy_flat(64, 64, Color::new(255, 120, 120, 255)),
                    'E' => Self::make_enemy_flat(64, 64, Color::new(120, 255, 120, 255)),
                    'S' => Self::make_enemy_flat(64, 64, Color::new(120, 120, 255, 255)),
                    'W' => Self::make_enemy_flat(64, 64, Color::new(255, 180, 80, 255)),

                    // Paredes/otros
                    _   => {
                        if let Some(pm) = tm.maps.get(&'1').cloned() { pm } else { Self::make_checker_pixmap(64, 64, Self::color_from_char(k)) }
                    },
                };
                tm.maps.insert(k, pm);
            }
        }

        tm
    }

    /// Cambia el modo alerta: las paredes '|' re-generan el pixmap con franjas rojas o cian.
    pub fn set_alert_mode(&mut self, alert: bool) {
        if self.alert_mode == alert { return; }
        self.alert_mode = alert;
        let pm = Self::make_pool_wall(64, 64, alert);
        self.maps.insert('|', pm);
    }

    fn color_from_char(c: char) -> Color {
        let k = c as u32;
        let r = ((k * 97) % 200 + 40) as u8;
        let g = ((k * 57) % 200 + 40) as u8;
        let b = ((k * 31) % 200 + 40) as u8;
        Color::new(r, g, b, 255)
    }

    /// Checker base
    fn make_checker_pixmap(w: u32, h: u32, base: Color) -> Pixmap {
        let mut px = vec![base; (w * h) as usize];
        let cell = 8u32;
        for y in 0..h {
            for x in 0..w {
                if ((x / cell) + (y / cell)) % 2 == 0 {
                    let i = (y * w + x) as usize;
                    let c = px[i];
                    px[i] = Self::mix(c, Color::WHITE, 24);
                }
            }
        }
        Pixmap::new(w, h, px)
    }

    /// Pared "pool": fondo negro + franjas glow arriba/abajo (cian o rojo si alerta).
    fn make_pool_wall(w: u32, h: u32, alert: bool) -> Pixmap {
        let mut px = vec![Color::BLACK; (w * h) as usize];
        let stripe_h = (h / 8).max(4);
        let bright = if alert { Color::new(255, 40, 40, 255) } else { Color::new(80, 200, 255, 255) };
        let mid    = if alert { Color::new(190, 30, 30, 255) } else { Color::new(40, 140, 220, 255) };
        let dim    = if alert { Color::new(120, 20, 20, 255) } else { Color::new(20, 90, 160, 255) };

        let paint_stripe = |px: &mut [Color], y0: u32, h: u32, w: u32| {
            for y in y0..(y0 + h).min(h + y0) {
                let t = ((y - y0) as f32) / (h as f32 - 1.0).max(1.0);
                let col = if t < 0.25 {
                    Self::mix(bright, mid, (t * 4.0 * 255.0) as u8)
                } else if t < 0.75 {
                    Self::mix(mid, dim, ((t - 0.25) * (255.0 / 0.5)) as u8)
                } else {
                    Self::mix(dim, Color::BLACK, ((t - 0.75) * (255.0 / 0.25)) as u8)
                };
                for x in 0..w {
                    let i = (y * w + x) as usize;
                    px[i] = Self::additive(px[i], col);
                }
            }
        };
        paint_stripe(&mut px, 0, stripe_h, w);
        paint_stripe(&mut px, h - stripe_h, stripe_h, w);

        // scanlines suaves en el centro
        for y in (h/2 - 4)..=(h/2 + 4) {
            for x in 0..w {
                let i = (y * w + x) as usize;
                px[i] = Self::mix(px[i], Color::new(20,20,20,255), 32);
            }
        }
        Pixmap::new(w, h, px)
    }

    /// Orb brillante
    fn make_glowing_orb(w: u32, h: u32, color: Color) -> Pixmap {
        let mut px = vec![Color::new(0,0,0,0); (w * h) as usize];
        let cx = (w as f32) * 0.5;
        let cy = (h as f32) * 0.5;
        let r  = (w.min(h) as f32) * 0.3;
        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let d  = (dx*dx + dy*dy).sqrt();
                let i  = (y * w + x) as usize;
                if d <= r {
                    let t = (1.0 - (d / r)).clamp(0.0, 1.0);
                    let core = Self::mix(color, Color::WHITE, (t * 220.0) as u8);
                    px[i] = Self::additive(px[i], core);
                    px[i].a = 255;
                } else {
                    let t = (1.0 - ((d - r) / (r*0.9))).clamp(0.0, 1.0);
                    if t > 0.0 {
                        let halo = Self::mix(color, Color::new(0,0,0,0), (200.0 * (1.0 - t)) as u8);
                        px[i] = Self::additive(px[i], halo);
                        px[i].a = (t * 180.0) as u8;
                    }
                }
            }
        }
        Pixmap::new(w, h, px)
    }

    /// Enemigo plano de fallback
    fn make_enemy_flat(w: u32, h: u32, body: Color) -> Pixmap {
        let mut px = vec![Color::new(0,0,0,0); (w*h) as usize];
        let cx = (w as f32)*0.5;
        let cy = (h as f32)*0.6;
        let rx = (w as f32)*0.23;
        let ry = (h as f32)*0.35;
        for y in 0..h {
            for x in 0..w {
                let nx = (x as f32 - cx) / rx;
                let ny = (y as f32 - cy) / ry;
                let i = (y*w + x) as usize;
                if nx*nx + ny*ny <= 1.0 {
                    px[i] = body;
                    px[i].a = 255;
                }
            }
        }
        Pixmap::new(w, h, px)
    }

    #[inline]
    fn mix(a: Color, b: Color, t: u8) -> Color {
        let ta = t as u16;
        let na = 255u16 - ta;
        let mixc = |x: u8, y: u8| -> u8 { (((x as u16)*na + (y as u16)*ta) / 255) as u8 };
        Color::new(mixc(a.r,b.r), mixc(a.g,b.g), mixc(a.b,b.b), mixc(a.a,b.a))
    }
    #[inline]
    fn additive(a: Color, b: Color) -> Color {
        let add = |x: u8, y: u8| -> u8 {
            let s = x as u16 + y as u16;
            if s > 255 { 255 } else { s as u8 }
        };
        Color::new(add(a.r,b.r), add(a.g,b.g), add(a.b,b.b), add(a.a,b.a))
    }


    /// Sample por pixel; si no existe key, blanco.
    pub fn get_pixel_color(&self, key: char, tx: u32, ty: u32) -> Color {
        if let Some(pm) = self.maps.get(&key) {
            return pm.sample(tx, ty);
        }
        Color::WHITE
    }
    /// Tamaño de la imagen (útil si quieres leerlo)
    pub fn image_size(&self, key: char) -> Option<(u32,u32)> {
        self.maps.get(&key).map(|p| (p.w, p.h))
    }

    #[allow(dead_code)]
    pub fn texture_for(&self, key: char) -> Option<&Texture2D> {
        self.textures.get(&key)
    }

    pub fn is_alert(&self) -> bool { self.alert_mode }
}
