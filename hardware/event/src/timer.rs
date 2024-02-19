use std::time::Instant;

struct Times {
    start: Instant,
    current: Instant,
    previous: Option<Instant>,
}

impl Times {
    fn push(&mut self, time: Instant) {
        self.previous = Some(self.current);
        self.current = time;
    }
}

pub struct Timer {
    times: Option<Times>,
}

impl Timer {
    pub(crate) fn new() -> Self {
        Self { times: None }
    }

    pub fn dt(&self) -> f32 {
        if let Some(Times {
            current,
            previous,
            start,
        }) = self.times
        {
            let duration = match previous {
                Some(prev) => current.duration_since(prev),
                None => current.duration_since(start),
            };
            duration.as_secs_f32()
        } else {
            0.0
        }
    }

    pub(crate) fn start(&mut self) {
        let start = Instant::now();
        self.times = Some(Times {
            start,
            current: start,
            previous: None,
        });
    }

    pub(crate) fn tick(&mut self) {
        if let Some(times) = self.times.as_mut() {
            times.push(Instant::now());
        }
    }
}
