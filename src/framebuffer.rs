use raylib::prelude::*;

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
        let background_color = Color::WHITE;
        let color_buffer = vec![background_color; size];
        Self {
            color_buffer,
            width,
            height,
            background_color,
            current_color: Color::BLACK,
        }
    }

    pub fn clear(&mut self) {
        self.color_buffer.fill(self.background_color);
    }

    pub fn set_pixel(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.color_buffer[index] = self.current_color;
        }
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn swap_buffers(&self, window: &mut RaylibHandle, thread: &RaylibThread) {
        let mut d = window.begin_drawing(thread);
        d.clear_background(Color::BLACK);

        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.color_buffer[(y * self.width + x) as usize];
                if color != self.background_color {
                    d.draw_pixel(x as i32, y as i32, color);
                }
            }
        }
    }
}
