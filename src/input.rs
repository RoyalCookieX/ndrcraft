#[derive(Clone, Copy, Debug, Default)]
pub struct Axis {
    pub negative: bool,
    pub positive: bool,
}

impl Axis {
    pub fn get_value(&self) -> Option<f32> {
        if !(self.negative ^ self.positive) {
            return None;
        }
        let negative_value = if self.negative { -1.0 } else { 0.0 };
        let positive_value = if self.positive { 1.0 } else { 0.0 };
        Some(negative_value + positive_value)
    }
}
