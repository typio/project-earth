// https://github.com/sotrh/learn-wgpu/blob/master/code/intermediate/tutorial12-camera/src/camera.rs

use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use winit::event::*;
use winit::keyboard::KeyCode;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    inverse_view: [[f32; 4]; 4],
    inverse_proj: [[f32; 4]; 4],
}

pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,

    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,

    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>>(
        position: V,
        yaw: Y,
        pitch: Y,
        size: winit::dpi::PhysicalSize<u32>,
        fovy: Y,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),

            aspect: size.width as f32 / size.height as f32,
            fovy: fovy.into(),
            znear,
            zfar,

            // Camera Controller
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed: 4.0,
            sensitivity: 0.4,
        }
    }

    pub fn get_uniform(&self) -> CameraUniform {
        CameraUniform {
            inverse_view: self.get_matrix().invert().unwrap().into(),
            inverse_proj: perspective(self.fovy, self.aspect, self.znear, self.zfar)
                .invert()
                .unwrap()
                .into(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.aspect = new_size.width as f32 / new_size.height as f32;
    }

    pub fn get_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        )
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::KeyE => {
                self.amount_up = amount;
                true
            }
            KeyCode::KeyQ => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 0.5,
            MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { y: scroll, .. }) => {
                -*scroll as f32
            }
        };
    }

    pub fn update_camera(&mut self, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = self.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        self.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        self.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        let (pitch_sin, pitch_cos) = self.pitch.0.sin_cos();
        let scrollward =
            Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        self.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        self.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        self.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        self.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        if self.pitch < -Rad(SAFE_FRAC_PI_2) {
            self.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if self.pitch > Rad(SAFE_FRAC_PI_2) {
            self.pitch = Rad(SAFE_FRAC_PI_2);
        }
    }
}
