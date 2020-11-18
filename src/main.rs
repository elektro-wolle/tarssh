//! A simple SSH tarpit, similar to endlessh.
//!
//! As per RFC 4253:
//!
//!   The server MAY send other lines of data before sending the version
//!   string.  Each line SHOULD be terminated by a Carriage Return and Line
//!   Feed.  Such lines MUST NOT begin with "SSH-", and SHOULD be encoded
//!   in ISO-10646 UTF-8 [RFC3629] (language is not specified).  Clients
//!   MUST be able to process such lines.
//!
//! In other words, you can fool SSH clients into waiting an extremely long time for a SSH handshake to even begin simply by waffling on endlessly.
//! My high score is just over a fortnight.
//!
//! The intent of this is to increase the cost of mass SSH scanning – even clients that immediately disconnect after the first response are delayed a little,
//! and that's one less free connection for the next attack.

#![warn(clippy::all)]
#![warn(missing_docs)]
#![warn(future_incompatible)]
#![deny(unused_must_use)]

#![cfg_attr(feature = "nightly", feature(external_doc))]
#![cfg_attr(feature = "nightly", doc(include = "../README.md"))]

/// Export some statistics.
#[cfg(feature = "exporters")]
mod exporters;
/// Listen to ssh-connections.
mod listeners;
/// Everything to do with keeping track what happend.
mod logging;
/// Collect some statistics.
mod metrics;
/// Drop privileges.
#[cfg(all(unix, feature = "drop_privs"))]
mod privilege_dropper;
/// Parallel execution of tasks.
mod runtime;
/// The actual ssh-tarpit.
mod tarpit;

use listeners::Listeners;
use log::{error, info, warn};
#[cfg(not(feature = "exporters"))]
use metrics::Metrics;
#[cfg(not(feature = "exporters"))]
use std::sync::Arc;
#[cfg(feature = "exporters")]
use exporters::Exporter;
#[cfg(all(unix, feature = "drop_privs"))]
use privilege_dropper::PrivDropConfig;
use runtime::Runtime;
use std::{
    io::{
        BufReader,
        prelude::*,
    },
    fs::File,
    net::SocketAddr,
    time::Duration,
};
use structopt::StructOpt;

#[cfg(all(unix, feature = "sandbox"))]
use rusty_sandbox::Sandbox;

#[derive(Debug, StructOpt)]
#[structopt(name = "tarssh", about = "A SSH tarpit server")]
struct Config {
    /// Listen address(es) to bind to of the tarpit.
    #[structopt(short = "l", long = "listen", default_value = "0.0.0.0:2222")]
    listen: Vec<SocketAddr>,
    /// Best-effort connection limit.
    #[structopt(short = "c", long = "max-clients", default_value = "4096")]
    max_clients: u32,
    /// Seconds between responses.
    #[structopt(short = "d", long = "delay", default_value = "10")]
    delay: u64,
    /// Socket write timeout.
    #[structopt(short = "t", long = "timeout", default_value = "30")]
    timeout: u64,
    /// Verbose level (repeat for more verbosity).
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u8,
    /// Use threads, with optional thread count.
    #[structopt(long = "threads")]
    #[allow(clippy::option_option)]
    threads: Option<Option<usize>>,
    /// Disable timestamps in logs.
    #[structopt(long)]
    disable_log_timestamps: bool,
    /// Disable module name in logs (e.g. "tarssh").
    #[structopt(long)]
    disable_log_ident: bool,
    /// Disable log level in logs (e.g. "info").
    #[structopt(long)]
    disable_log_level: bool,
    #[cfg(all(unix, feature = "drop_privs"))]
    #[structopt(flatten)]
    #[cfg(all(unix, feature = "drop_privs"))]
    privdrop: PrivDropConfig,
    /// Filename of the tarpit-message.
    #[structopt(short = "m", long = "message", default_value = "")]
    message: String,
    /// Listen address(es) to bind to of the exporter.
    #[structopt(short = "e", long = "exporter", default_value = "0.0.0.0:8080")]
    #[cfg(feature = "exporters")]
    exporter: Vec<SocketAddr>,
}

pub(crate) fn errx<M: AsRef<str>>(code: i32, message: M) -> ! {
    error!("{}", message.as_ref());
    std::process::exit(code);
}

fn main() -> std::io::Result<()> {
    let opt = Config::from_args();

    logging::init(
        opt.verbose,
        !opt.disable_log_timestamps,
        !opt.disable_log_ident,
        !opt.disable_log_level,
    );

    let mut runtime = Runtime::new(opt.threads);

    let listeners = Listeners::new(
        &mut runtime,
        opt.listen,
    );

    #[cfg(feature = "exporters")]
    let exporters = Exporter::new(
        &mut runtime,
        opt.exporter,
    );

    #[cfg(all(unix, feature = "drop_privs"))]
    opt.privdrop.drop();

    #[cfg(all(unix, feature = "sandbox"))]
    {
        let sandboxed = Sandbox::new().sandbox_this_process().is_ok();
        info!("sandbox, enabled: {}", sandboxed);
    }

    #[cfg(feature = "exporters")]
    let metrics = exporters.spawn(&runtime);
    #[cfg(not(feature = "exporters"))]
    let metrics = std::sync::Arc::new(metrics::Metrics::new(runtime.start()));

    listeners.spawn(
        &runtime,
        opt.max_clients as usize,
        Duration::from_secs(opt.delay),
        Duration::from_secs(opt.timeout),
        metrics.clone(),
        if opt.message.is_empty() {
            format!(
                "{}\r\n{}\r\n{}\r\n{}\r\n{}\r\n{}\r\n",
                "My name is Yon Yonson",
                "I live in Wisconsin.",
                "There, the people I meet",
                "As I walk down the street",
                "Say “Hey, what’s your name?”",
                "And I say:",
            )
        } else {
            BufReader::new(File::open(opt.message)?)
            .lines()
            .try_fold(
                String::new(),
                |mut result, line| if let Ok(line) = line {
                    result.push_str(&line);
                    result.push_str("\r\n");
                    Ok(result)
                } else {
                    line
                },
            )?
        },
    );

    runtime.wait(metrics);
    Ok(())
}
