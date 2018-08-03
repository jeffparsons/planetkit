use slog::Logger;

pub struct LogResource {
    pub log: Logger,
}

impl LogResource {
    // Can't implement Default because it needs a
    // root logger provided from the outside world.
    pub fn new(parent_log: &Logger) -> LogResource {
        LogResource {
            log: parent_log.new(o!()),
        }
    }
}
