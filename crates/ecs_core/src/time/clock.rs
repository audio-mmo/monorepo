use std::time::Instant;

use anyhow::{anyhow, bail, Result};

use super::TimeSource;

/// Implementation of [TimeSource], which assumes a fixed given simulation time and fixed period.
#[derive(Debug)]
pub struct Clock {
    process_epoch: Instant,
    tick_counter: u64,
    tick_granularity: f64,
    simulation_time: f64,
}

#[derive(Debug, Default)]
pub struct ClockBuilder {
    simulation_time: Option<f64>,
    tick_counter: Option<u64>,
    tick_granularity: Option<f64>,
}

impl ClockBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn simulation_time(mut self, simulation_time: f64) -> Self {
        self.simulation_time = Some(simulation_time);
        self
    }

    pub fn tick_counter(mut self, tick_count: u64) -> Self {
        self.tick_counter = Some(tick_count);
        self
    }

    pub fn tick_granularity(mut self, tick_duration: f64) -> Self {
        self.tick_granularity = Some(tick_duration);
        self
    }

    pub fn build(self) -> Result<Clock> {
        let simulation_time = self
            .simulation_time
            .ok_or_else(|| anyhow!("Must specify simulation_time"))?;
        let tick_counter = self
            .tick_counter
            .ok_or_else(|| anyhow!("Must specify tick_counter"))?;
        let tick_granularity = self
            .tick_granularity
            .ok_or_else(|| anyhow!("Must set tick_granularity"))?;

        if simulation_time.is_nan() || simulation_time < 0.0 || simulation_time.is_infinite() {
            bail!("Got invalid simulation_time {}", simulation_time);
        }

        if tick_granularity <= 0.0 || tick_granularity.is_nan() || tick_granularity.is_infinite() {
            bail!("Got invalid tick_granularity {}", tick_granularity);
        }

        Ok(Clock {
            simulation_time,
            tick_counter,
            tick_granularity,
            process_epoch: Instant::now(),
        })
    }
}

impl Clock {
    /// Advance the clock by one tick
    pub fn advance(&mut self) {
        self.advance_multiple(1);
    }

    /// Advance the clock by 0 or more ticks.
    ///
    /// Ticks are u32 because advancing by more than that at a time won't ever work.
    pub fn advance_multiple(&mut self, ticks: u32) {
        self.simulation_time += self.tick_granularity * ticks as f64;
        self.tick_counter += ticks as u64;
    }
}

impl TimeSource for Clock {
    fn get_process_time(&self) -> f64 {
        let now = Instant::now();
        (now - self.process_epoch).as_secs_f64()
    }

    fn get_simulation_time(&self) -> f64 {
        self.simulation_time
    }

    fn get_tick_counter(&self) -> u64 {
        self.tick_counter
    }

    fn get_tick_granularity(&self) -> f64 {
        self.tick_granularity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use float_eq::assert_float_eq;

    #[test]
    fn test_advancing() {
        // Tolerence for fp comparisons in this test.
        const TOL: f64 = 0.01;

        let test_epoch = Instant::now();
        let mut clock = ClockBuilder::new()
            .simulation_time(5.0)
            .tick_counter(10)
            .tick_granularity(0.5)
            .build()
            .unwrap();

        // Start off by making sure all our fields make sense.
        assert_float_eq!(clock.get_simulation_time(), 5.0, abs <= TOL);
        assert_eq!(clock.get_tick_counter(), 10);
        assert_float_eq!(clock.get_tick_granularity(), 0.5, abs <= TOL);

        // Let's advance by one tick and see what happens.
        clock.advance();
        assert_float_eq!(clock.get_simulation_time(), 5.5, abs <= TOL);
        assert_eq!(clock.get_tick_counter(), 11);

        // And can we go by more than one?
        clock.advance_multiple(3);
        assert_float_eq!(clock.get_simulation_time(), 7.0, abs <= TOL);
        assert_eq!(clock.get_tick_counter(), 14);

        // Let's test "process" time.  We'll do so by sleeping for a bit, then comparing what we did versus it.
        std::thread::sleep(std::time::Duration::from_millis(200));

        assert_float_eq!(
            clock.get_process_time(),
            (Instant::now() - test_epoch).as_secs_f64(),
            abs <= TOL
        );
    }
}
