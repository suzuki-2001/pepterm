// 3D camera and projection module
// Based on terminal3d by Liam Ilan (https://github.com/liam-ilan/terminal3d)

use crate::{model, screen};
use crate::screen::Rgb;

// Simple 3d point wrapper.
#[derive(Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Point {
    // Create a new point.
    pub fn new(x: f32, y: f32, z: f32) -> Point {
        Point { x, y, z }
    }
}

pub struct Camera {
    // Location of the camera
    pub coordinates: Point,

    // In Radians.
    // Operations applied in order: yaw, pitch, roll,
    // Starting from z+ direction.
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,

    // viewport parameters.
    pub viewport_distance: f32,

    // In radians
    pub viewport_fov: f32,

    // Screen to render.
    pub screen: screen::Screen
}

#[allow(dead_code)]
impl Camera {
    // Create a new camera.
    pub fn new(
        coordinates: Point,
        yaw: f32, pitch: f32, roll: f32,
        viewport_distance: f32, viewport_fov: f32,
    ) -> Camera {
        Camera {
            coordinates,
            yaw, pitch, roll,
            viewport_distance, viewport_fov,
            screen: screen::Screen::new()
        }
    }

    // Convert world to camera coordinates.
    fn world_to_camera(&self, point: &Point) -> Point {
        // Compute trig values for camera angles.
        let (s_yaw, s_pitch, s_roll) = (self.yaw.sin(), self.pitch.sin(), self.roll.sin());
        let (c_yaw, c_pitch, c_roll) = (self.yaw.cos(), self.pitch.cos(), self.roll.cos());

        // Compute deltas between camera and point position.
        let delta_x = point.x - self.coordinates.x;
        let delta_y = point.y - self.coordinates.y;
        let delta_z = point.z - self.coordinates.z;

        // Undo yaw.
        let unyawed_x = delta_x * c_yaw - delta_z * s_yaw;
        let unyawed_y = delta_y;
        let unyawed_z = delta_x * s_yaw + delta_z * c_yaw;

        // Undo pitch.
        let unpitched_x = unyawed_x;
        let unpitched_y = unyawed_y * c_pitch - unyawed_z * s_pitch;
        let unpitched_z = unyawed_y * s_pitch + unyawed_z * c_pitch;

        // Undo roll.
        let unrolled_x = unpitched_x * c_roll - unpitched_y * s_roll;
        let unrolled_y = unpitched_x * s_roll + unpitched_y * c_roll;
        let unrolled_z = unpitched_z;

        Point::new(unrolled_x, unrolled_y, unrolled_z)
    }

    // Convert camera to screen coordinates.
    fn camera_to_screen(&self, point: &Point) -> screen::Point {
        // Project onto viewport coordinates.
        let viewport_x = point.x * self.viewport_distance / point.z;
        let viewport_y = point.y * self.viewport_distance / point.z;

        // Compute viewport width and height based on screen width, height, and fov.
        let viewport_width = 2. * self.viewport_distance * (self.viewport_fov / 2.).tan();
        let viewport_height = (self.screen.height as f32 / self.screen.width as f32) * viewport_width;

        // Project to screen coordinates.
        let screen_x = (viewport_x / viewport_width + 0.5) * self.screen.width as f32;
        let screen_y = (1.0 - (viewport_y / viewport_height + 0.5)) * self.screen.height as f32;

        // Round.
        screen::Point::new(screen_x.round() as i32, screen_y.round() as i32)
    }

    // Plot points of a given model.
    pub fn plot_model_points(&mut self, model: &model::Model) {
        for point in model.points.iter() {
            self.write(true, &model.model_to_world(point));
        }
    }

    // Plot edges of a given model (white).
    pub fn plot_model_edges(&mut self, model: &model::Model) {
        for edge in model.edges.iter() {
            self.edge(
                &model.model_to_world(&edge.0),
                &model.model_to_world(&edge.1)
            );
        }
    }

    // Plot colored edges of a given model.
    pub fn plot_model_colored_edges(&mut self, model: &model::Model) {
        for edge in model.colored_edges.iter() {
            self.edge_color(
                &model.model_to_world(&edge.start),
                &model.model_to_world(&edge.end),
                edge.start_color,
                edge.end_color
            );
        }
    }

    // Plot a 3d point.
    pub fn write(&mut self, val: bool, point: &Point) {
        let camera_point = self.world_to_camera(point);
        if camera_point.z >= self.viewport_distance {
            self.screen.write(val, &self.camera_to_screen(&camera_point));
        }
    }

    // Plot a 3d edge (white).
    pub fn edge(&mut self, start: &Point, end: &Point) {
        self.edge_color(start, end, Rgb::white(), Rgb::white());
    }

    // Check if a point in camera space is within the view frustum (with margin)
    #[inline]
    fn is_in_frustum(&self, camera_point: &Point) -> bool {
        if camera_point.z < self.viewport_distance {
            return false;
        }
        // Calculate frustum bounds at this depth with some margin
        let half_width = camera_point.z * (self.viewport_fov / 2.0).tan() * 1.5;
        let aspect = self.screen.height as f32 / self.screen.width as f32;
        let half_height = half_width * aspect;

        camera_point.x.abs() <= half_width && camera_point.y.abs() <= half_height
    }

    // Plot a 3d edge with color (handles clipping and color interpolation)
    pub fn edge_color(&mut self, start: &Point, end: &Point, start_color: Rgb, end_color: Rgb) {
        let camera_start = self.world_to_camera(start);
        let camera_end = self.world_to_camera(end);
        let clip_start = camera_start.z < self.viewport_distance;
        let clip_end = camera_end.z < self.viewport_distance;

        if clip_start && clip_end { return; }

        // No clipping needed - check frustum and draw
        if !clip_start && !clip_end {
            // Frustum culling: skip if both points are outside on the same side
            if !self.is_in_frustum(&camera_start) && !self.is_in_frustum(&camera_end) {
                let both_left = camera_start.x < 0.0 && camera_end.x < 0.0;
                let both_right = camera_start.x > 0.0 && camera_end.x > 0.0;
                let both_up = camera_start.y > 0.0 && camera_end.y > 0.0;
                let both_down = camera_start.y < 0.0 && camera_end.y < 0.0;

                if both_left || both_right || both_up || both_down {
                    let z_min = camera_start.z.min(camera_end.z);
                    let half_width = z_min * (self.viewport_fov / 2.0).tan() * 1.5;
                    let aspect = self.screen.height as f32 / self.screen.width as f32;
                    let half_height = half_width * aspect;

                    if (both_left && camera_start.x < -half_width && camera_end.x < -half_width) ||
                       (both_right && camera_start.x > half_width && camera_end.x > half_width) ||
                       (both_up && camera_start.y > half_height && camera_end.y > half_height) ||
                       (both_down && camera_start.y < -half_height && camera_end.y < -half_height) {
                        return;
                    }
                }
            }
            self.screen.line_color(
                &self.camera_to_screen(&camera_start),
                &self.camera_to_screen(&camera_end),
                start_color, end_color
            );
            return;
        }

        // Handle clipping with color interpolation
        let (clipped, unclipped, clipped_color, unclipped_color) = if clip_start {
            (camera_start, camera_end, start_color, end_color)
        } else {
            (camera_end, camera_start, end_color, start_color)
        };

        let lambda = (self.viewport_distance - clipped.z) / (unclipped.z - clipped.z);
        let new_clipped = Point::new(
            lambda * (unclipped.x - clipped.x) + clipped.x,
            lambda * (unclipped.y - clipped.y) + clipped.y,
            self.viewport_distance
        );

        let clip_color = Rgb::new(
            ((1.0 - lambda) * clipped_color.r as f32 + lambda * unclipped_color.r as f32) as u8,
            ((1.0 - lambda) * clipped_color.g as f32 + lambda * unclipped_color.g as f32) as u8,
            ((1.0 - lambda) * clipped_color.b as f32 + lambda * unclipped_color.b as f32) as u8,
        );

        self.screen.line_color(
            &self.camera_to_screen(&new_clipped),
            &self.camera_to_screen(&unclipped),
            clip_color, unclipped_color
        );
    }

    // Plot a model into a specific viewport section of the screen.
    pub fn plot_model_in_viewport(
        &mut self,
        model: &model::Model,
        camera_pos: Point,
        yaw: f32,
        pitch: f32,
        viewport_x_offset: u16,
        viewport_width: u16,
        viewport_height: u16,
    ) {
        // Temporarily override camera parameters for this viewport
        let orig_coords = self.coordinates;
        let orig_yaw = self.yaw;
        let orig_pitch = self.pitch;

        self.coordinates = camera_pos;
        self.yaw = yaw;
        self.pitch = pitch;

        let aspect = viewport_height as f32 / viewport_width as f32;
        let clip_x_min = viewport_x_offset as i32;
        let clip_x_max = (viewport_x_offset + viewport_width) as i32;
        let clip_y_min = 0;
        let clip_y_max = viewport_height as i32;

        for edge in model.colored_edges.iter() {
            let start = model.model_to_world(&edge.start);
            let end = model.model_to_world(&edge.end);

            let camera_start = self.world_to_camera(&start);
            let camera_end = self.world_to_camera(&end);

            let clip_start = camera_start.z < self.viewport_distance;
            let clip_end = camera_end.z < self.viewport_distance;

            if clip_start && clip_end { continue; }

            let (screen_start, screen_end, start_color, end_color) = if !clip_start && !clip_end {
                let s = self.camera_to_viewport_screen(&camera_start, viewport_width, viewport_height, aspect);
                let e = self.camera_to_viewport_screen(&camera_end, viewport_width, viewport_height, aspect);
                (s, e, edge.start_color, edge.end_color)
            } else {
                let (clipped, unclipped, clipped_color, unclipped_color) = if clip_start {
                    (camera_start, camera_end, edge.start_color, edge.end_color)
                } else {
                    (camera_end, camera_start, edge.end_color, edge.start_color)
                };

                let lambda = (self.viewport_distance - clipped.z) / (unclipped.z - clipped.z);
                let new_clipped = Point::new(
                    lambda * (unclipped.x - clipped.x) + clipped.x,
                    lambda * (unclipped.y - clipped.y) + clipped.y,
                    self.viewport_distance
                );

                let clip_color = Rgb::new(
                    ((1.0 - lambda) * clipped_color.r as f32 + lambda * unclipped_color.r as f32) as u8,
                    ((1.0 - lambda) * clipped_color.g as f32 + lambda * unclipped_color.g as f32) as u8,
                    ((1.0 - lambda) * clipped_color.b as f32 + lambda * unclipped_color.b as f32) as u8,
                );

                let s = self.camera_to_viewport_screen(&new_clipped, viewport_width, viewport_height, aspect);
                let e = self.camera_to_viewport_screen(&unclipped, viewport_width, viewport_height, aspect);
                (s, e, clip_color, unclipped_color)
            };

            let offset_start = screen::Point::new(screen_start.x + viewport_x_offset as i32, screen_start.y);
            let offset_end = screen::Point::new(screen_end.x + viewport_x_offset as i32, screen_end.y);

            self.screen.line_color_clipped(
                &offset_start, &offset_end, start_color, end_color,
                clip_x_min, clip_x_max, clip_y_min, clip_y_max
            );
        }

        // Restore original camera parameters
        self.coordinates = orig_coords;
        self.yaw = orig_yaw;
        self.pitch = orig_pitch;
    }

    // Convert camera to screen coordinates for a specific viewport
    fn camera_to_viewport_screen(&self, point: &Point, viewport_width: u16, viewport_height: u16, aspect: f32) -> screen::Point {
        let viewport_x = point.x * self.viewport_distance / point.z;
        let viewport_y = point.y * self.viewport_distance / point.z;

        let vp_width = 2. * self.viewport_distance * (self.viewport_fov / 2.).tan();
        let vp_height = aspect * vp_width;

        let screen_x = (viewport_x / vp_width + 0.5) * viewport_width as f32;
        let screen_y = (1.0 - (viewport_y / vp_height + 0.5)) * viewport_height as f32;

        screen::Point::new(screen_x.round() as i32, screen_y.round() as i32)
    }
}
