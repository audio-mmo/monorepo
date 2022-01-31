//! helpers for logging.

/// Log to stdout.
///
/// If called multiple times in the same process, only applies once.
pub fn log_to_stderr() {
    static ONCE: std::sync::Once = std::sync::Once::new();

    ONCE.call_once(|| {
        env_logger::builder()
            .format(|buf, record| {
                use std::io::Write;

                let now = time::OffsetDateTime::now_utc();

                writeln!(
                    buf,
                    "{} {} time={} target={}",
                    record.level(),
                    record.args(),
                    now,
                    record.target()
                )
            })
            .init();
    });
}
