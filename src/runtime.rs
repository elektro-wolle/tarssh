use exitcode;
use futures::stream::StreamExt;
use futures_util::future::FutureExt;
use log::info;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Instant,
};
use super::{errx, metrics::Metrics};

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

pub(crate) struct Runtime {
    runtime: tokio::runtime::Runtime,
    startup: Instant,
}

impl Runtime {
    pub(crate) fn new(
        threads: Option<Option<usize>>,
    ) -> Self {
        let mut runtime = tokio::runtime::Builder::new();
        let scheduler = if let Some(threaded) = threads {
            runtime.threaded_scheduler();
            if let Some(threads) = threaded {
                let threads = threads.min(512).max(1);
                runtime.core_threads(threads);
                format!("threaded, threads: {}", threads)
            } else {
                "threaded".to_owned()
            }
        } else {
            runtime.basic_scheduler();
            "basic".to_owned()
        };

        info!(
            "init, version: {}, scheduler: {}",
            env!("CARGO_PKG_VERSION"),
            scheduler,
        );

        let runtime = runtime
            .enable_all()
            .build()
            .unwrap_or_else(|err| errx(exitcode::UNAVAILABLE, format!("tokio, error: {:?}", err)));

        Self {
            runtime,
            startup: Instant::now()
        }
    }

    pub(crate) fn start(&self) -> Instant {
        self.startup
    }

    pub(crate) fn wait(
        &mut self,
        metrics: Arc<Metrics>,
    ) {
        self.block_on(
            async {
                let interrupt = tokio::signal::ctrl_c().into_stream().map(|_| "interrupt");

                #[cfg(unix)]
                let mut term = signal(SignalKind::terminate()).unwrap_or_else(|error| {
                    errx(exitcode::UNAVAILABLE, format!("signal(), error: {}", error))
                });

                #[cfg(unix)]
                let interrupt = futures_util::stream::select(
                    interrupt,
                    term.recv().into_stream().map(|_| "terminated")
                );

                if let Some(signal) = interrupt.boxed().next().await {
                    info!("{}", signal);
                };
            }
        );

        info!(
            "shutdown, uptime: {:.2?}, clients: {}",
            self.startup.elapsed(),
            metrics.connections(),
        )
    }
}

impl Deref for Runtime {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl DerefMut for Runtime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime
    }
}
