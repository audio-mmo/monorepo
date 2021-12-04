/// Trait representing something which can provide time.
///
/// there are 3 kinds of time:
///
/// - Process time, which is relative to process start.
/// - simulation time, which is relative to the simulation's start (e.g. it's always steady from the first time the
///   world launched).
/// - ticks, which measure discrete simulation steps.
///
/// Most systems and components want to work in simulation time, but ticks and knowing the tick granularity are useful
/// for networking.  Process time should never be stored; it resets to 0 on next launch.
///
/// All clock implementations must be monotonically increasing.
///
/// We represent times as f64 seconds instead of duration types for convenience, and because the duration types (even if
/// we wrote our own) start caring about a bunch of edge cases that we don't.
///
/// It is assumed that anything that actually neads wallclock time will use `SystemTime`, but that is intended to be a
/// rare requirement.
pub trait TimeSource {
    /// Get the simulation time in seconds.
    ///
    /// This is relative to simulation start, and preserved across process restarts.
    fn get_simulation_time(&self) -> f64;

    /// Get the current tick counter.  This is steady, relative to simulation start, and preserved across process restarts.
    fn get_tick_counter(&self) -> u64;

    /// Get the process time, which is roughly the length of time the process has been running.
    fn get_process_time(&self) -> f64;

    /// Get the tick granularity.
    ///
    /// It should not be assumed that `get_simulation_time` will return something such that this value evenly divides
    /// it.  It is possible for that to not be the case, especially in the event that it becomes necessary to increase
    /// or decrease the tick duration.
    fn get_tick_granularity(&self) -> f64;
}
