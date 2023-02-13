use crate::{Deg, Matrix4, Vector3, Vector4, Zero};

#[derive(Debug)]
pub struct Controller {
    position: Vector3<f32>,
    yaw: Deg<f32>,
    pitch: Deg<f32>,
}

impl Controller {
    const MIN_PITCH_DEG: f32 = -89.0;
    const MAX_PITCH_DEG: f32 = 89.0;

    pub const fn new(position: Vector3<f32>, yaw: Deg<f32>, pitch: Deg<f32>) -> Self {
        Self {
            position,
            yaw,
            pitch,
        }
    }

    pub fn get_transform(&self) -> Matrix4<f32> {
        Matrix4::from_translation(self.position) * self.get_rotation_matrix()
    }

    pub fn translate_local(&mut self, translation: Vector3<f32>) {
        let rotation = self.get_rotation_matrix();
        let direction = Vector4::new(translation.x, translation.y, translation.z, 1.0);
        let translation = {
            let translation = rotation * direction;
            Vector3::new(translation.x, translation.y, translation.z)
        };
        self.translate_global(translation);
    }

    pub fn translate_global(&mut self, translation: Vector3<f32>) {
        self.position += translation;
    }

    pub fn rotate_yaw(&mut self, yaw: Deg<f32>) {
        self.yaw += yaw;
    }

    pub fn rotate_pitch(&mut self, pitch: Deg<f32>) {
        self.pitch += pitch;
        self.pitch = Deg(self.pitch.0.clamp(Self::MIN_PITCH_DEG, Self::MAX_PITCH_DEG));
    }

    #[inline]
    fn get_rotation_matrix(&self) -> Matrix4<f32> {
        Matrix4::from_angle_y(self.yaw) * Matrix4::from_angle_x(self.pitch)
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self::new(Vector3::zero(), Deg::zero(), Deg::zero())
    }
}
