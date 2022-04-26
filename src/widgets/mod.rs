use iui::draw::*;

pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }
}

impl From<&Color> for SolidBrush {
    fn from(color: &Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: 1.0,
        }
    }
}

pub mod leds;
