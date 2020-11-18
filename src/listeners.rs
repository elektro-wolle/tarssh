use log::{info, warn};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use super::{
    errx,
    tarpit::tarpit_connection,
    metrics::Metrics,
    runtime::Runtime,
};
use tokio::{
    net::TcpListener,
    time::delay_for,
};

pub(crate) struct Listeners {
    inner: Vec<TcpListener>,
}

impl Listeners {
    pub(crate) fn new(
        runtime: &mut Runtime,
        listen: Vec<SocketAddr>,
    ) -> Self {
        Self {
            inner:
                listen
                .iter()
                .map(
                    |addr| match runtime.block_on(async { TcpListener::bind(addr).await }) {
                        Ok(listener) => {
                            info!("listen, addr: {}", addr);
                            listener
                        }
                        Err(err) => {
                            errx(
                                exitcode::OSERR,
                                format!("listen, addr: {}, error: {}", addr, err),
                            );
                        }
                    },
                )
                .collect()
        }
    }

    pub(crate) fn len(
        &self,
    ) -> usize {
        self.inner.len()
    }

    pub(crate) fn spawn(
        self,
        runtime: &Runtime,
        max_clients: usize,
        delay: Duration,
        timeout: Duration,
        metrics: Arc<Metrics>,
        banner: String,
    ) {
        info!(
            "start, servers: {}, max_clients: {}, delay: {}s, timeout: {}s, banner:\n{}",
            self.len(),
            max_clients,
            delay.as_secs(),
            timeout.as_secs(),
            banner,
        );
        let banner = Arc::new(banner.into_bytes());
        for mut listener in self.inner {
            let banner = banner.clone();
            let metrics = metrics.clone();
            let server = async move {
                loop {
                    match listener.accept().await {
                        Ok((sock, peer)) => {
                            let metrics = metrics.clone();
                            match metrics.connect(max_clients, Instant::now()) {
                                Ok((connected, token)) => {
                                    info!("connect, peer: {}, clients: {}", peer, connected);
                                    tokio::spawn(
                                        tarpit_connection(
                                            sock,
                                            peer,
                                            delay,
                                            timeout,
                                            token,
                                            metrics.clone(),
                                            banner.clone()
                                        )
                                    );
                                },
                                Err(connected) => info!("reject, peer: {}, clients: {}", peer, connected),
                            }
                        }
                        Err(err) => match err.kind() {
                            std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::ConnectionReset => (),
                            _ => {
                                let wait = Duration::from_millis(100);
                                warn!("accept, err: {}, wait: {:?}", err, wait);
                                delay_for(wait).await;
                            }
                        },
                    }
                }
            };
            runtime.spawn(server);
        }
    }
}
