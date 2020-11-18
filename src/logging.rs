use env_logger;
use log::LevelFilter;

pub(crate) fn init(
    verbosity:  u8,
    timestamps: bool,
    ident:      bool,
    level:      bool,
) {
    env_logger::Builder::from_default_env()
        .filter(
            None,
            match verbosity {
                0 => LevelFilter::Off,
                1 => LevelFilter::Info,
                2 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
        )
        .format_timestamp(if timestamps {
            Some(env_logger::fmt::TimestampPrecision::Millis)
        } else {
            None
        })
        .format_module_path(ident)
        .format_level(level)
        .init();
}
