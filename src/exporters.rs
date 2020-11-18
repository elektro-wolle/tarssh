use log::info;

use hyper::{
    Body, Request, Response, Server,
    server::{
        Builder,
        conn::AddrIncoming,
    },
    service::{make_service_fn, service_fn},
};

use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::Arc,
};

use super::{
    metrics::Metrics,
    runtime::Runtime,
};

pub(crate) struct Exporter {
    inner: Vec<Builder<AddrIncoming>>,
}

impl Exporter {
    pub(crate) fn new(
        runtime: &mut Runtime,
        listen: Vec<SocketAddr>,
    ) -> Self {
        Self {
            inner: listen.iter().map(|address| {
                let listener = runtime.block_on(async { Server::bind(&address) });
                info!("listen, addr: {}", address);
                listener
            }).collect()
        }
    }

    pub(crate) fn spawn(
        self,
        runtime: &Runtime,
    ) -> Arc<Metrics> {
        let metrics = Arc::new(Metrics::new(runtime.start()));

        for exporter in self.inner {
            let metrics = metrics.clone();
            runtime.spawn(
                exporter.serve(
                    make_service_fn(
                        move |_connection| {
                            let metrics = metrics.clone();
                            async move {
                                Ok::<_, Infallible>(
                                    service_fn(
                                        move |req: Request<Body>| {
                                            let metrics = metrics.clone();
                                            async move {
                                                metrics.handle(req).await
                                            }
                                        }
                                    )
                                )
                            }
                        }
                    )
                )
            );
        }

        metrics
    }
}

impl Metrics {
    pub(crate) async fn handle(
        &self,
        _request: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        Ok(Response::new(Body::from(self.export())))
    }
}
