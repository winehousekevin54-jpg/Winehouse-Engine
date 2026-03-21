use glam::{Mat4, Vec3};

pub struct Camera {
    pub azimuth: f32,   // horizontal rotation (radians)
    pub elevation: f32, // vertical rotation (radians)
    pub distance: f32,  // distance from target
    pub target: Vec3,
    pub fov_y: f32, // vertical FOV (radians)
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            azimuth: std::f32::consts::FRAC_PI_4,
            elevation: 0.5,
            distance: 5.0,
            target: Vec3::ZERO,
            fov_y: std::f32::consts::FRAC_PI_4,
            aspect: 1.0,
            near: 0.01,
            far: 1000.0,
        }
    }

    pub fn position(&self) -> Vec3 {
        let cos_e = self.elevation.cos();
        let sin_e = self.elevation.sin();
        let x = self.distance * cos_e * self.azimuth.sin();
        let y = self.distance * sin_e;
        let z = self.distance * cos_e * self.azimuth.cos();
        self.target + Vec3::new(x, y, z)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn view_proj(&self) -> Mat4 {
        self.proj_matrix() * self.view_matrix()
    }

    pub fn orbit(&mut self, delta_az: f32, delta_el: f32) {
        self.azimuth += delta_az;
        self.elevation = (self.elevation + delta_el)
            .clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * (1.0 + factor)).clamp(0.5, 500.0);
    }

    pub fn set_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
