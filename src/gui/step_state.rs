/// State of execution. This is used to step per-instruction and to pause the CPU.
#[derive(Clone)]
pub struct StepState {
    pub paused: bool,
    pub step: bool,
}

impl Default for StepState {
    fn default() -> Self {
        Self {
            paused: true,
            step: true,
        }
    }
}

impl StepState {
    pub fn step(&mut self) {
        self.step = true;
        self.paused = true;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}
