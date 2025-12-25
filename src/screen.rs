// Screen rendering module
// Based on terminal3d by Liam Ilan (https://github.com/liam-ilan/terminal3d)

use std::*;
use std::io::Write;
use crossterm::{
    execute,
    terminal,
    cursor
};

const DEFAULT_TERMINAL_DIMENSIONS: (u16, u16) = (80, 24);

// RGB color for a pixel
#[derive(Copy, Clone, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    pub fn white() -> Rgb {
        Rgb { r: 255, g: 255, b: 255 }
    }

    pub fn black() -> Rgb {
        Rgb { r: 0, g: 0, b: 0 }
    }
}

// Setup ability to get dimensions out of matrix arrays.
pub trait Dim {
    const WIDTH: usize;
    const HEIGHT: usize;
}

impl<const WIDTH: usize, const HEIGHT: usize> Dim for [[bool; WIDTH]; HEIGHT] {
    const WIDTH: usize = WIDTH;
    const HEIGHT: usize = HEIGHT;
}

// Create pixel trait.
pub trait Pixel: Dim + ops::IndexMut<usize, Output=[bool; 2]> + Clone {
    fn new() -> Self;
    fn to_char(&self) -> char;
}

// Pixel types, represent a single char.
pub type BlockPixel = [[bool; 2]; 2];
impl Pixel for BlockPixel {
    fn new() -> BlockPixel { [[false; BlockPixel::WIDTH]; BlockPixel::HEIGHT] }
    fn to_char(&self) -> char {
        match self {
            [[false, false], [false, false]] => ' ',
            [[true, false], [false, false]] => '▘',
            [[false, true], [false, false]] => '▝',
            [[true, true], [false, false]] => '▀',
            [[false, false], [true, false]] => '▖',
            [[true, false], [true, false]] => '▌',
            [[false, true], [true, false]] => '▞',
            [[true, true], [true, false]] => '▛',
            [[false, false], [false, true]] => '▗',
            [[true, false], [false, true]] => '▚',
            [[false, true], [false, true]] => '▐',
            [[true, true], [false, true]] => '▜',
            [[false, false], [true, true]] => '▄',
            [[true, false], [true, true]] => '▙',
            [[false, true], [true, true]] => '▟',
            [[true, true], [true, true]] => '█'
        }
    }
}

pub type BrailePixel = [[bool; 2]; 4];
impl Pixel for BrailePixel {
    fn new() -> BrailePixel { [[false; BrailePixel::WIDTH]; BrailePixel::HEIGHT] }
    fn to_char(&self) -> char {
        let mut unicode: u32 = 0;
        if self[0][0] { unicode |= 1 << 0 }
        if self[1][0] { unicode |= 1 << 1 }
        if self[2][0] { unicode |= 1 << 2 }

        if self[0][1] { unicode |= 1 << 3 }
        if self[1][1] { unicode |= 1 << 4 }
        if self[2][1] { unicode |= 1 << 5 }

        if self[3][0] { unicode |= 1 << 6 }
        if self[3][1] { unicode |= 1 << 7 }

        unicode |= 0x28 << 8;

        char::from_u32(unicode).unwrap()
    }
}

// Simple 2d point wrapper.
#[derive(Copy, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32
}

impl Point {
    // Create a new point.
    pub fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
}

// Cell with on/off and color
#[derive(Copy, Clone)]
pub struct ColorCell {
    pub on: bool,
    pub color: Rgb,
}

impl ColorCell {
    pub fn new() -> ColorCell {
        ColorCell { on: false, color: Rgb::white() }
    }
}

// Wrapper for a "screen" to render.
pub struct Screen {
    pub width: u16,
    pub height: u16,
    content: Vec<Vec<ColorCell>>,
}

impl Screen {
    // Create a new screen, sized to the terminal.
    pub fn new() -> Screen {
        // Clear term and go to 0, 0.
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        ).unwrap();

        // Get initial terminal size
        let (terminal_width, terminal_height) = match terminal::size() {
            Ok(dim) => dim,
            Err(_) => DEFAULT_TERMINAL_DIMENSIONS
        };

        // Create screen with initial buffer (use BrailePixel dimensions as default)
        let width = terminal_width * 2;  // BrailePixel::WIDTH = 2
        let height = (terminal_height.saturating_sub(1)) * 4;  // BrailePixel::HEIGHT = 4

        Screen {
            content: vec![vec![ColorCell::new(); width as usize]; height as usize],
            width,
            height
        }
    }

    // Resize braile screen to fit terminal width and height.
    pub fn fit_to_terminal<T: Pixel>(&mut self) {
        let (terminal_width, terminal_height) = match terminal::size() {
            Ok(dim) => dim,
            Err(_) => DEFAULT_TERMINAL_DIMENSIONS
        };

        self.resize(
            terminal_width * T::WIDTH as u16,
            (terminal_height - 1) * T::HEIGHT as u16
        );
    }

    // Write a value with color to a coord on the screen.
    pub fn write_color(&mut self, val: bool, point: &Point, color: Rgb) {
        // Fix: use >= 0 instead of > 0 to include edge pixels
        let x_in_bounds = point.x >= 0 && point.x < self.width as i32;
        let y_in_bounds = point.y >= 0 && point.y < self.height as i32;
        if x_in_bounds && y_in_bounds {
            self.content[point.y as usize][point.x as usize] = ColorCell { on: val, color };
        }
    }

    // Write a value (white) to a coord on the screen.
    pub fn write(&mut self, val: bool, point: &Point) {
        self.write_color(val, point, Rgb::white());
    }

    // Clears the whole screen by resetting existing buffer (no allocation)
    pub fn clear(&mut self) {
        for row in &mut self.content {
            for cell in row {
                cell.on = false;
                cell.color = Rgb::white();
            }
        }
    }

    // Resizes the screen - always recreate to avoid corruption
    pub fn resize(&mut self, width: u16, height: u16) {
        // Only resize if dimensions changed
        if width != self.width || height != self.height {
            // Always create fresh buffer to avoid any corruption
            self.content = vec![vec![ColorCell::new(); width as usize]; height as usize];
            self.width = width;
            self.height = height;
        }
    }

    // Draw a colored line with Bresenham's line algorithm.
    pub fn line_color(&mut self, start: &Point, end: &Point, start_color: Rgb, end_color: Rgb) {
        let delta_x = (end.x - start.x).abs();
        let step_x: i32 = if start.x < end.x {1} else {-1};
        let delta_y = -(end.y - start.y).abs();
        let step_y: i32 = if start.y < end.y {1} else {-1};
        let mut err = delta_x + delta_y;

        let mut x = start.x;
        let mut y = start.y;

        let total_steps = (delta_x.abs() + (-delta_y).abs()).max(1) as f32;
        let mut step = 0;

        loop {
            // Interpolate color
            let t = step as f32 / total_steps;
            let color = Rgb::new(
                ((1.0 - t) * start_color.r as f32 + t * end_color.r as f32) as u8,
                ((1.0 - t) * start_color.g as f32 + t * end_color.g as f32) as u8,
                ((1.0 - t) * start_color.b as f32 + t * end_color.b as f32) as u8,
            );

            self.write_color(true, &Point::new(x, y), color);

            if x == end.x && y == end.y { break; }

            let curr_err = err;
            if 2 * curr_err >= delta_y {
                err += delta_y;
                x += step_x;
            }
            if 2 * curr_err <= delta_x {
                err += delta_x;
                y += step_y;
            }
            step += 1;
        }
    }

    // Draw a colored line clipped to specified bounds
    pub fn line_color_clipped(
        &mut self,
        start: &Point,
        end: &Point,
        start_color: Rgb,
        end_color: Rgb,
        clip_x_min: i32,
        clip_x_max: i32,
        clip_y_min: i32,
        clip_y_max: i32,
    ) {
        let delta_x = (end.x - start.x).abs();
        let step_x: i32 = if start.x < end.x {1} else {-1};
        let delta_y = -(end.y - start.y).abs();
        let step_y: i32 = if start.y < end.y {1} else {-1};
        let mut err = delta_x + delta_y;

        let mut x = start.x;
        let mut y = start.y;

        let total_steps = (delta_x.abs() + (-delta_y).abs()).max(1) as f32;
        let mut step = 0;

        loop {
            // Only draw if within clip bounds
            if x >= clip_x_min && x < clip_x_max && y >= clip_y_min && y < clip_y_max {
                let t = step as f32 / total_steps;
                let color = Rgb::new(
                    ((1.0 - t) * start_color.r as f32 + t * end_color.r as f32) as u8,
                    ((1.0 - t) * start_color.g as f32 + t * end_color.g as f32) as u8,
                    ((1.0 - t) * start_color.b as f32 + t * end_color.b as f32) as u8,
                );
                self.write_color(true, &Point::new(x, y), color);
            }

            if x == end.x && y == end.y { break; }

            let curr_err = err;
            if 2 * curr_err >= delta_y {
                err += delta_y;
                x += step_x;
            }
            if 2 * curr_err <= delta_x {
                err += delta_x;
                y += step_y;
            }
            step += 1;
        }
    }

    // Render the screen with colors and status bar
    pub fn render_with_status<PixelType: Pixel>(&self, status: &str) {
        let pixel_height = PixelType::HEIGHT;
        let pixel_width = PixelType::WIDTH;
        let real_row_width = self.width.div_ceil(pixel_width as u16) as usize;
        let num_rows = self.height.div_ceil(pixel_height as u16) as usize;

        // Pre-allocate buffer with generous capacity
        let estimated_size = real_row_width * num_rows * 30 + 100;
        let mut buffer = Vec::<u8>::with_capacity(estimated_size);

        // Move cursor to home position and reset color state
        buffer.extend_from_slice(b"\x1b[H\x1b[0m");

        // Pre-allocate row buffers outside the loop
        let mut real_row: Vec<(PixelType, Rgb)> = vec![(PixelType::new(), Rgb::black()); real_row_width];
        let mut color_accum: Vec<(u32, u32, u32, u32)> = vec![(0, 0, 0, 0); real_row_width];

        let mut current_color: Option<Rgb> = None;
        let mut row_idx = 0;

        while row_idx < self.height as usize {
            // Reset buffers instead of reallocating
            for i in 0..real_row_width {
                real_row[i] = (PixelType::new(), Rgb::black());
                color_accum[i] = (0, 0, 0, 0);
            }

            for subpixel_y in 0..pixel_height {
                let y = row_idx + subpixel_y;
                if y >= self.height as usize {
                    break;
                }

                let row = &self.content[y];
                for real_x in 0..real_row_width {
                    for subpixel_x in 0..pixel_width {
                        let x = real_x * pixel_width + subpixel_x;
                        if x >= self.width as usize {
                            break;
                        }

                        let cell = &row[x];
                        real_row[real_x].0[subpixel_y][subpixel_x] = cell.on;
                        if cell.on {
                            color_accum[real_x].0 += cell.color.r as u32;
                            color_accum[real_x].1 += cell.color.g as u32;
                            color_accum[real_x].2 += cell.color.b as u32;
                            color_accum[real_x].3 += 1;
                        }
                    }
                }
            }

            // Compute average colors
            for i in 0..real_row_width {
                if color_accum[i].3 > 0 {
                    let count = color_accum[i].3;
                    real_row[i].1 = Rgb::new(
                        (color_accum[i].0 / count) as u8,
                        (color_accum[i].1 / count) as u8,
                        (color_accum[i].2 / count) as u8,
                    );
                }
            }

            // Build output for this row
            for i in 0..real_row_width {
                let (ref pixel, ref color) = real_row[i];
                let ch = pixel.to_char();
                if ch != ' ' {
                    // Only change color if different
                    if current_color.map_or(true, |c| c.r != color.r || c.g != color.g || c.b != color.b) {
                        // Manual formatting to avoid allocation
                        buffer.extend_from_slice(b"\x1b[38;2;");
                        write_u8_to_buffer(&mut buffer, color.r);
                        buffer.push(b';');
                        write_u8_to_buffer(&mut buffer, color.g);
                        buffer.push(b';');
                        write_u8_to_buffer(&mut buffer, color.b);
                        buffer.push(b'm');
                        current_color = Some(*color);
                    }
                    let mut char_buf = [0u8; 4];
                    buffer.extend_from_slice(ch.encode_utf8(&mut char_buf).as_bytes());
                } else {
                    buffer.push(b' ');
                }
            }

            // Clear to end of line and newline
            buffer.extend_from_slice(b"\x1b[K\r\n");

            row_idx += pixel_height;
        }

        // Reset color and add centered status bar
        buffer.extend_from_slice(b"\x1b[0m");
        let terminal_width = real_row_width;
        let status_len = status.chars().count();
        let padding = if terminal_width > status_len {
            (terminal_width - status_len) / 2
        } else {
            0
        };
        for _ in 0..padding {
            buffer.push(b' ');
        }
        buffer.extend_from_slice(status.as_bytes());
        buffer.extend_from_slice(b"\x1b[K");

        // Write entire frame at once with lock held
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(&buffer);
        let _ = handle.flush();
    }
}

// Helper to write u8 as decimal without allocation
fn write_u8_to_buffer(buffer: &mut Vec<u8>, n: u8) {
    if n >= 100 {
        buffer.push(b'0' + n / 100);
        buffer.push(b'0' + (n / 10) % 10);
        buffer.push(b'0' + n % 10);
    } else if n >= 10 {
        buffer.push(b'0' + n / 10);
        buffer.push(b'0' + n % 10);
    } else {
        buffer.push(b'0' + n);
    }
}
