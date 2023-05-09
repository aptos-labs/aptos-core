pub struct TickingTimer {
    timeout: u32,
    ticks_elapsed: u32,
    paused: bool,
}

impl TickingTimer {
    pub fn new(timeout: u32) -> Self {
        Self {
            timeout,
            ticks_elapsed: 0,
            paused: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        if !self.paused {
            self.ticks_elapsed += 1;
            return self.elapsed();
        }
        false
    }

    pub(crate) fn reset(&mut self) {
        self.ticks_elapsed = 0;
        self.paused = false;
    }

    pub(crate) fn stop(&mut self) {
        self.paused = true;
    }

    pub(crate) fn elapsed(&self) -> bool {
        self.ticks_elapsed >= self.timeout
    }
}

// TODO(ibalajiarun): macro for creating timer
