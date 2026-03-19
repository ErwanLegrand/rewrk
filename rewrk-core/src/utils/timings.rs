use std::ops::{Add, AddAssign};
use std::time::Duration;

#[derive(Debug, Default)]
pub struct RuntimeTimings {
    /// The total runtime duration waiting on the producer.
    pub producer_wait_runtime: Duration,
    /// The total runtime duration waiting on the requests to execute.
    pub execute_wait_runtime: Duration,
}

impl Add for RuntimeTimings {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            producer_wait_runtime: self.producer_wait_runtime
                + rhs.producer_wait_runtime,
            execute_wait_runtime: self.execute_wait_runtime + rhs.execute_wait_runtime,
        }
    }
}

impl AddAssign for RuntimeTimings {
    fn add_assign(&mut self, rhs: Self) {
        self.producer_wait_runtime += rhs.producer_wait_runtime;
        self.execute_wait_runtime += rhs.execute_wait_runtime;
    }
}

impl FromIterator<Self> for RuntimeTimings {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        let mut total = Self::default();
        for slf in iter {
            total += slf;
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_zero() {
        let t = RuntimeTimings::default();
        assert_eq!(t.producer_wait_runtime, Duration::ZERO);
        assert_eq!(t.execute_wait_runtime, Duration::ZERO);
    }

    #[test]
    fn test_add_assign() {
        let mut a = RuntimeTimings {
            producer_wait_runtime: Duration::from_millis(10),
            execute_wait_runtime: Duration::from_millis(20),
        };
        let b = RuntimeTimings {
            producer_wait_runtime: Duration::from_millis(5),
            execute_wait_runtime: Duration::from_millis(15),
        };
        a += b;
        assert_eq!(a.producer_wait_runtime, Duration::from_millis(15));
        assert_eq!(a.execute_wait_runtime, Duration::from_millis(35));
    }

    #[test]
    fn test_add() {
        let a = RuntimeTimings {
            producer_wait_runtime: Duration::from_millis(10),
            execute_wait_runtime: Duration::from_millis(20),
        };
        let b = RuntimeTimings {
            producer_wait_runtime: Duration::from_millis(5),
            execute_wait_runtime: Duration::from_millis(15),
        };
        let c = a + b;
        assert_eq!(c.producer_wait_runtime, Duration::from_millis(15));
        assert_eq!(c.execute_wait_runtime, Duration::from_millis(35));
    }

    #[test]
    fn test_from_iterator() {
        let timings = vec![
            RuntimeTimings {
                producer_wait_runtime: Duration::from_millis(10),
                execute_wait_runtime: Duration::from_millis(5),
            },
            RuntimeTimings {
                producer_wait_runtime: Duration::from_millis(20),
                execute_wait_runtime: Duration::from_millis(10),
            },
        ];
        let total: RuntimeTimings = timings.into_iter().collect();
        assert_eq!(total.producer_wait_runtime, Duration::from_millis(30));
        assert_eq!(total.execute_wait_runtime, Duration::from_millis(15));
    }
}
