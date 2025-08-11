// textures.rs
use raylib::prelude::*;
use std::collections::HashMap;

/// Almacenamos las texturas como un buffer inmutable de pixeles (Color),
/// con su ancho y alto. Así no necesitamos &mut Image para samplear.
struct Pixmap {
    w: u32,
    h: u32,
    px: Vec<Color>,
}

impl Pixmap {
    fn new(w: u32, h: u32, px: Vec<Color>) -> Self {
        Self { w, h, px }
    }
    #[inline]
    fn sample(&self, x: u32, y: u32) -> Color {
        let xi = (x % self.w) as usize;
        let yi = (y % self.h) as usize;
        self.px[(yi * self.w as usize) + xi]
    }
}

pub struct TextureManager {
    maps: HashMap<char, Pixmap>,        // CPU pixmaps para sampleo por pixel
    textures: HashMap<char, Texture2D>, // (opcional) GPU textures si existen archivos
}

impl TextureManager {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut tm = Self {
            maps: HashMap::new(),
            textures: HashMap::new(),
        };

        // Intentar cargar archivos si existen (no hacemos panic si faltan)
        let candidates: &[(&str, char)] = &[
            ("assets/wall1.png", '1'),
            ("assets/wall2.png", '2'),
            ("assets/wall3.png", '3'),
            ("assets/wall4.png", '4'),
            ("assets/goal.png",  'g'),
        ];

        for (path, key) in candidates {
            if let Ok(img) = Image::load_image(path) {
                // Guardar textura GPU opcionalmente
                if let Ok(tex) = rl.load_texture_from_image(thread, &img) {
                    tm.textures.insert(*key, tex);
                }
                // Convertir a buffer inmutable de pixeles (Color)
                let w = img.width().max(1) as u32;
                let h = img.height().max(1) as u32;
                let data = img.get_image_data().to_vec(); // <-- conversión a Vec<Color>
                tm.maps.insert(*key, Pixmap::new(w, h, data));
            }
        }

        // Claves comunes en laberinto ASCII como "paredes"
        let fallbacks: &[char] = &['+', '-', '|', '#', '1', '2', '3', '4', 'g'];
        for &k in fallbacks {
            if !tm.maps.contains_key(&k) {
                // Generar un checker procedural para ese char
                let pm = Self::make_checker_pixmap(64, 64, Self::color_from_char(k));
                tm.maps.insert(k, pm);
            }
        }

        tm
    }

    fn color_from_char(c: char) -> Color {
        // color estable según char
        let k = c as u32;
        let r = ((k * 97) % 200 + 40) as u8;
        let g = ((k * 57) % 200 + 40) as u8;
        let b = ((k * 31) % 200 + 40) as u8;
        Color::new(r, g, b, 255)
    }

    /// Genera un checker sin usar métodos mut de Image,
    /// simplemente rellenando un Vec<Color>.
    fn make_checker_pixmap(w: u32, h: u32, base: Color) -> Pixmap {
        let mut px = Vec::with_capacity((w * h) as usize);

        // Fondo base
        for _ in 0..(w * h) {
            px.push(base);
        }

        // Overlay checker suave (cada 8x8)
        let cell = 8u32;
        for y in 0..h {
            for x in 0..w {
                if ((x / cell) + (y / cell)) % 2 == 0 {
                    let i = (y * w + x) as usize;
                    // mezclar con un blanco muy tenue
                    let c = px[i];
                    let mix = |a: u8, b: u8, t: u8| -> u8 {
                        // t ~ 24/255
                        let ta = t as u16;
                        let na = 255u16 - ta;
                        (((a as u16) * na + (b as u16) * ta) / 255) as u8
                    };
                    px[i] = Color::new(mix(c.r, 255, 24), mix(c.g, 255, 24), mix(c.b, 255, 24), 255);
                }
            }
        }

        Pixmap::new(w, h, px)
    }

    /// Sample por pixel, siempre devuelve un color.
    /// No requiere &mut self.
    pub fn get_pixel_color(&self, key: char, tx: u32, ty: u32) -> Color {
        if let Some(pm) = self.maps.get(&key) {
            return pm.sample(tx, ty);
        }
        Color::WHITE
    }

    #[allow(dead_code)]
    pub fn texture_for(&self, key: char) -> Option<&Texture2D> {
        self.textures.get(&key)
    }
}
