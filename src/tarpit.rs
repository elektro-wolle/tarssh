use std::sync::Arc;
use std::time::Duration;

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
) -> Result<Option<Token>, &'static str> {
    if let Err(_err) = timeout(
        *time_out,
        sock.write_all(chunk)
    )
    .await
    .unwrap_or_else(
      |_| Err(std::io::Error::new(std::io::ErrorKind::Other, "timed out"))
    ) {
        metrics.disconnect(token)
    } else {
        delay_for(*delay).await;
        if let Err(error) = metrics.sent_chunk(&token) {
            Err(error)
        } else {
            Ok(Some(token))
        }
    }
}

pub(crate) async fn tarpit_connection(
    mut sock:   tokio::net::TcpStream,
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
          if let Ok(Some(t)) = send_chunk(
              &mut sock,
              &delay,
              &time_out,
              token,
              &metrics,
              b"Meow Meow Meow, but anymeow:\r\n",
          ).await {
              token = t;
              metrics.sent_easteregg(&token)?;
          } else {
              break 'otter;
          }
        }

        for chunk in banner.chunks(16) {
            if let Ok(Some(t)) = send_chunk(
                &mut sock,
                &delay,
                &time_out,
                token,
                &metrics,
                chunk,
            ).await {
                token = t;
            } else {
                break 'otter;
            }
        }

        metrics.sent_banner(&token)?;
    }
    Ok(())
}
