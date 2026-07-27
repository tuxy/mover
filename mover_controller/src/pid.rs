const HARD_MIN: f32 = 0.0;
const HARD_MAX: f32 = 65535.0;

pub struct PIDController {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    integral: f32,
    prev: f32,
}

impl PIDController {
    pub fn init(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev: 0.0,
        }
    }

    pub fn compute_iteration(&mut self, target: f32, actual: f32, dt_us: f32) -> f32 {
        let error = target - actual;
        let dt_s = dt_us / 1_000_000.0;

        let p = self.kp * error;
        self.integral += error * dt_s;

        let i = self.ki * self.integral;

        let d = if dt_s > 0.0 {
            self.kd * ((error - self.prev) / dt_s)
        } else {
            0.0
        };

        self.prev = error;

        (p + i + d).clamp(HARD_MIN, HARD_MAX)
    }
}
