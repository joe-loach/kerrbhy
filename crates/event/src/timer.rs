use std::time::Instant;

pub struct Timer {
    start: Option<Instant>,
    last: Option<Instant>,
    curr: Option<Instant>,
}

impl Timer {
    pub(crate) fn new() -> Self {
        Self {
            start: None,
            last: None,
            curr: None,
        }
    }

    pub fn dt(&self) -> f32 {
        match (self.curr, self.last) {
            (Some(a), Some(b)) => a.duration_since(b).as_secs_f32(),
            (Some(a), None) => a.duration_since(self.start.unwrap()).as_secs_f32(),
            (None, Some(_)) => unreachable!(),
            (None, None) => 0.0,
        }
    }

    pub(crate) fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    pub(crate) fn tick(&mut self) {
        if self.start.is_none() {
            return;
        }
        self.last = self.curr.take();
        self.curr = Some(Instant::now());
    }
}
