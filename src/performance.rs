use std::time;

#[derive(Debug)]
pub struct ScopedTimer {
    name: String,
    start: time::Instant,
}

impl ScopedTimer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            start: time::Instant::now(),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        log::info!("{} -> {:.2}s", self.name, duration.as_secs_f32());
    }
}
