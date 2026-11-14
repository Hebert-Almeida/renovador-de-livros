use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};

/// Fonte de tempo injetável (testes usam [`FakeClock`]).
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Relógio manual para testes.
#[derive(Debug)]
pub struct FakeClock(Mutex<DateTime<Utc>>);

impl FakeClock {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self(Mutex::new(start))
    }
    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
    pub fn set(&self, to: DateTime<Utc>) {
        *self.0.lock().unwrap() = to;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_clock_advances() {
        let c = FakeClock::new(DateTime::<Utc>::UNIX_EPOCH);
        c.advance(Duration::seconds(5));
        assert_eq!(c.now().timestamp(), 5);
    }
}
