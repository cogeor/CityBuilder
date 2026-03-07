#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimMap {
    Power,
    ServiceHealth,
    ServiceSafety,
    ServiceEduc,
    TrafficDensity,
    Pollution,
    LandValue,
    ZoneDemand,
}

pub const SIM_MAP_COUNT: usize = 8;

fn idx(m: SimMap) -> usize {
    match m {
        SimMap::Power          => 0,
        SimMap::ServiceHealth  => 1,
        SimMap::ServiceSafety  => 2,
        SimMap::ServiceEduc    => 3,
        SimMap::TrafficDensity => 4,
        SimMap::Pollution      => 5,
        SimMap::LandValue      => 6,
        SimMap::ZoneDemand     => 7,
    }
}

/// Double-buffered registry of named f32 scalar maps.
/// During a tick, systems read `current()` and write `next_mut()`.
/// Call `swap()` once all systems have finished.
#[derive(Debug)]
pub struct SimMapRegistry {
    buffers: [[Vec<f32>; 2]; SIM_MAP_COUNT],
    active: usize,
    width: usize,
    height: usize,
}

impl SimMapRegistry {
    pub fn new(width: usize, height: usize) -> Self {
        let len = width * height;
        let buffers: [[Vec<f32>; 2]; SIM_MAP_COUNT] = std::array::from_fn(|_| {
            [vec![0.0_f32; len], vec![0.0_f32; len]]
        });
        Self {
            buffers,
            active: 0,
            width,
            height,
        }
    }

    pub fn current(&self, map: SimMap) -> &[f32] {
        &self.buffers[idx(map)][self.active]
    }

    pub fn next_mut(&mut self, map: SimMap) -> &mut Vec<f32> {
        let next = 1 - self.active;
        &mut self.buffers[idx(map)][next]
    }

    pub fn swap(&mut self) {
        self.active = 1 - self.active;
    }

    pub fn clear_next(&mut self, map: SimMap) {
        let next = 1 - self.active;
        let len = self.width * self.height;
        let buf = &mut self.buffers[idx(map)][next];
        buf.clear();
        buf.resize(len, 0.0_f32);
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_zero_filled_buffers() {
        let reg = SimMapRegistry::new(4, 4);
        for &map in &[
            SimMap::Power, SimMap::ServiceHealth, SimMap::ServiceSafety,
            SimMap::ServiceEduc, SimMap::TrafficDensity, SimMap::Pollution,
            SimMap::LandValue, SimMap::ZoneDemand,
        ] {
            assert!(reg.current(map).iter().all(|&v| v == 0.0_f32));
        }
    }

    #[test]
    fn swap_makes_written_next_become_current() {
        let mut reg = SimMapRegistry::new(3, 3);
        let next = reg.next_mut(SimMap::Power);
        next[0] = 42.0_f32;
        assert_eq!(reg.current(SimMap::Power)[0], 0.0_f32);
        reg.swap();
        assert_eq!(reg.current(SimMap::Power)[0], 42.0_f32);
    }

    #[test]
    fn clear_next_zeroes_next_buffer() {
        let mut reg = SimMapRegistry::new(2, 2);
        {
            let next = reg.next_mut(SimMap::Pollution);
            for v in next.iter_mut() {
                *v = 7.0_f32;
            }
        }
        reg.clear_next(SimMap::Pollution);
        reg.swap();
        assert!(reg.current(SimMap::Pollution).iter().all(|&v| v == 0.0_f32));
    }
}
