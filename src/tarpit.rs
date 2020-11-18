use log::info;
use std::{
    borrow::Cow,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use log::warn;
use tokio::io::AsyncWriteExt;
use tokio::time::{delay_for, timeout};

use super::metrics::{Metrics, Token};

async fn send_chunk(
    sock: &mut tokio::net::TcpStream,
    delay: &Duration,
    time_out: &Duration,
    token: Token,
    metrics: &Arc<Metrics>,
    chunk: &[u8],
) -> Result<Token, (usize, u64, Cow<'static, str>)> {
    delay_for(*delay).await;
    match timeout(
        *time_out,
        sock.write_all(chunk)
    )
    .await {
        Ok(Ok(_)) => if let Err(error) = metrics.sent_chunk(&token) {
            Err(match metrics.disconnect(token) {
                Ok((connections, connection_time)) => (
                    connections,
                    connection_time,
                    Cow::Borrowed(error),
                ),
                Err(failure) => (
                    0usize,
                    0u64,
                    Cow::Owned(format!("{}\", \"{}", error, failure)),
                ),
            })
        } else {
            Ok(token)
        },
        Err(error) => {
          Err(match metrics.disconnect(token) {
              Ok((connections, connection_time)) => (
                  connections,
                  connection_time,
                  Cow::Borrowed("time out"),
              ),
              Err(failure) => (
                  0usize,
                  0u64,
                  Cow::Owned(format!("{}\", \"{}", error, failure)),
              ),
          })
        },
        Ok(Err(error)) => {
          Err(match metrics.disconnect(token) {
              Ok((connections, connection_time)) => (
                  connections,
                  connection_time,
                  Cow::Owned(format!("{}", error)),
              ),
              Err(failure) => (
                  0usize,
                  0u64,
                  Cow::Owned(format!("{}\", \"{}", error, failure)),
              ),
          })
        },
    }
}

pub(crate) async fn tarpit_connection(
    mut sock:   tokio::net::TcpStream,
    peer:       SocketAddr,
    delay:      Duration,
    time_out:   Duration,
    mut token:  Token,
    metrics:    Arc<Metrics>,
    banner:     Arc<Vec<u8>>,
) -> Result<(), &'static str> {
    sock.set_recv_buffer_size(1)
        .unwrap_or_else(|err| warn!("set_recv_buffer_size(), error: {}", err));

    sock.set_send_buffer_size(16)
        .unwrap_or_else(|err| warn!("set_send_buffer_size(), error: {}", err));

    'otter: loop {
        if rand::random::<u8>() == 0x42 {
            match send_chunk(
                &mut sock,
                &delay,
                &time_out,
                token,
                &metrics,
                b"Meow Meow Meow, but anymeow:\r\n",
            ).await {
                Ok(the_token) => {
                    token = the_token;
                    metrics.sent_easteregg(&token)?;
                },
                Err((connected, connection_time, error)) => {
                    info!(
                        "disconnect, peer: {}, duration: {:.2?}, error: \"{}\", clients: {}",
                        peer,
                        connection_time,
                        error,
                        connected,
                    );
                    break 'otter;
                },
            }
        }

        for chunk in banner.chunks(16) {
            match send_chunk(
                &mut sock,
                &delay,
                &time_out,
                token,
                &metrics,
                chunk,
            ).await {
                Ok(the_token) => {
                    token = the_token;
                },
                Err((connected, connection_time, error)) => {
                    info!(
                        "disconnect, peer: {}, duration: {:.2?}, error: \"{}\", clients: {}",
                        peer,
                        connection_time,
                        error,
                        connected,
                    );
                    break 'otter;
                },
            }
        }

        metrics.sent_banner(&token)?;
    }
    Ok(())
}
