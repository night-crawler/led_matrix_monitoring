use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageBuffer, ImageEncoder, Luma};
use std::io::{Cursor, Write};

use crate::constants::{HEIGHT, WIDTH};
use crate::render::unit_interval::{NumUnitIntervalExt, UnitInterval};

pub struct Renderer {
    buf: ImageBuffer<Luma<u8>, Vec<u8>>,
    max_brightness: u8,
}

impl Renderer {
    pub fn new() -> Self {
        let buf = ImageBuffer::new(WIDTH, HEIGHT);
        Renderer {
            buf,
            max_brightness: 255,
        }
    }

    fn validate_mid_point(mid_point: u32, max_height: u32) -> anyhow::Result<()> {
        if mid_point < max_height {
            return Err(anyhow::anyhow!(
                "Mid point must be greater than max height: {mid_point} < {max_height}"
            ));
        }
        if (mid_point as i32 - max_height as i32) < 0 {
            return Err(anyhow::anyhow!(
                "Full length of the bar must be positive: {mid_point} - {max_height} < 0"
            ));
        }
        Ok(())
    }

    pub fn render_cpu(
        &mut self,
        mid_point: u32,
        max_height: u32,
        cpu_load: &[u8],
    ) -> anyhow::Result<()> {
        Self::validate_mid_point(mid_point, max_height)?;

        for (index, &load) in cpu_load.iter().enumerate().take((WIDTH * 2) as usize) {
            let index = index as u32;
            let x = index % WIDTH;

            if index >= WIDTH {
                self.render_vertical_bar(load as u64, 100, x, mid_point, mid_point + max_height)?;
            } else {
                self.render_vertical_bar(load as u64, 100, x, mid_point, mid_point - max_height)?;
            }
        }

        Ok(())
    }

    pub fn plot_ui(
        &mut self,
        mid_point: u32,
        max_height: u32,
        data_points: &[(u64, u64)],
    ) -> anyhow::Result<()> {
        Self::validate_mid_point(mid_point, max_height)?;
        if data_points.is_empty() {
            return Ok(());
        }

        let max_rx = data_points
            .iter()
            .take(WIDTH as usize)
            .map(|&(rx, _)| rx)
            .max()
            .unwrap_or(0);
        let max_tx = data_points
            .iter()
            .take(WIDTH as usize)
            .map(|&(tx, _)| tx)
            .max()
            .unwrap_or(0);

        if max_rx == 0 && max_tx == 0 {
            return Ok(());
        }

        for (index, &(rx, tx)) in data_points.iter().enumerate().take(WIDTH as usize) {
            let x = index as u32;

            self.render_vertical_bar(rx, max_rx, x, mid_point, mid_point - max_height)?;
            self.render_vertical_bar(tx, max_tx, x, mid_point, mid_point + max_height)?;
        }

        Ok(())
    }

    pub fn render_horizontal_bar(
        &mut self,
        value: u64,
        max_value: u64,
        start_y: u32,
        start_x: u32,
        end_x: u32,
    ) -> anyhow::Result<()> {
        let max_value = max_value.max(value);

        let range = start_x.min(end_x)..start_x.max(end_x);
        if range.contains(&WIDTH) {
            return Err(anyhow::anyhow!(
                "A range of {start_x} to {end_x} exceeds the display width: {WIDTH}"
            ));
        }

        let max_width = range.count();

        let load = value.to_unit(max_value);
        let length: u32 = load.scale(max_width);
        let max_brightness: u8 = load.scale(self.max_brightness);

        let range = if start_x < end_x {
            start_x..(start_x + length)
        } else {
            (start_x - length)..start_x
        };

        for x in range {
            let distance = UnitInterval::new_sigmoid_range_abs(x, start_x, max_width, 6);
            let brightness = distance.scale(max_brightness);
            self.buf.put_pixel(x, start_y, Luma([brightness]));
        }

        Ok(())
    }

    pub fn render_vertical_bar(
        &mut self,
        value: u64,
        max_value: u64,
        start_x: u32,
        start_y: u32,
        end_y: u32,
    ) -> anyhow::Result<()> {
        let max_value = max_value.max(value);

        let range = start_y.min(end_y)..start_y.max(end_y);
        if range.contains(&HEIGHT) {
            return Err(anyhow::anyhow!(
                "A range of {start_y} to {end_y} exceeds the display height: {HEIGHT}"
            ));
        }

        let max_height = range.count();

        let load = value.to_unit(max_value);
        let length: u32 = load.scale(max_height);
        let max_brightness: u8 = load.scale(self.max_brightness);

        let range = if start_y < end_y {
            start_y..(start_y + length)
        } else {
            (start_y - length)..start_y
        };

        for y in range {
            let distance = UnitInterval::new_sigmoid_range_abs(y, start_y, max_height, 6);
            let brightness = distance.scale(max_brightness);
            self.buf.put_pixel(start_x, y, Luma([brightness]));
        }

        Ok(())
    }

    pub fn render_average_cpu(
        &mut self,
        start_x: u32,
        start_y: u32,
        end_y: u32,
        load: &[u8],
    ) -> anyhow::Result<()> {
        let avg_load = load.iter().map(|&l| l as u64).sum::<u64>() / load.len() as u64;
        self.render_vertical_bar(avg_load, 100, start_x, start_y, end_y)?;
        self.render_vertical_bar(avg_load, 100, start_x + 1, start_y, end_y)?;
        Ok(())
    }
    pub fn save_to_in_memory_png(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let cursor = Cursor::new(&mut buffer);
        let encoder = PngEncoder::new(cursor);
        encoder.write_image(
            &self.buf,
            self.buf.width(),
            self.buf.height(),
            ExtendedColorType::L8,
        )?;
        Ok(buffer)
    }

    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let mut file = std::fs::File::create(path)?;
        let buf = self.save_to_in_memory_png()?;
        file.write_all(&buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOAD: [u8; 16] = [
        50, 100, 50, 100, 25, 30, 35, 40, 100, 50, 55, 60, 65, 70, 75, 100,
    ];

    #[test]
    fn test_render_cpu() {
        let mut renderer = Renderer::new();
        assert!(renderer.render_cpu(10, 10, &LOAD).is_ok());
        renderer.save_to_file("./target/cpu.png").unwrap();

        renderer.render_average_cpu(7, 20, 10, &[100; 16]).unwrap();
        renderer.save_to_file("./target/cpu_avg.png").unwrap();
    }

    #[test]
    fn test_render_io() {
        let data_points = [
            (100, 100),
            (200, 200),
            (300, 300),
            (400, 400),
            (0, 0),
            (600, 600),
            (700, 700),
            (800, 800),
            (900, 900),
        ];

        let mut renderer = Renderer::new();
        assert!(renderer.plot_ui(27, 7, &data_points).is_ok());
        renderer.save_to_file("./target/network_io.png").unwrap();
    }

    #[test]
    fn test_render_horizontal_bar() {
        let mut renderer = Renderer::new();
        assert!(renderer.render_horizontal_bar(100, 100, 33, 0, 9).is_ok());
        assert!(renderer.render_horizontal_bar(100, 100, 32, 9, 0).is_ok());
        renderer.save_to_file("./target/temp.png").unwrap();
    }

    #[test]
    fn test_render_vertical_bar() {
        let mut renderer = Renderer::new();
        assert!(renderer.render_vertical_bar(100, 100, 0, 0, 10).is_ok());
        assert!(renderer.render_vertical_bar(100, 100, 8, 10, 0).is_ok());
        renderer.save_to_file("./target/vertical_bar.png").unwrap();
    }
}
