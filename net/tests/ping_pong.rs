//! A simple test that sets up a client and server, then sends and awaits a reply.
mod utils;

use std::borrow::Cow;
use std::str::FromStr;

use anyhow::Result;
use log::*;

use ammo_framer::{Message, MessageIdentifier, MessageKind};

use ammo_net::*;

use utils::wait_for_message;

async fn ping_pong_impl() -> Result<()> {
    ammo_logging::log_to_stderr();

    let server = Server::new(
        ServerConfigBuilder::default()
            .local_addr(std::net::SocketAddr::try_from((
                std::net::IpAddr::from_str("127.0.0.1")?,
                0,
            ))?)
            .build()?,
    )
    .await?;
    tokio::spawn(server.clone().listening_loop());

    let server_addr = server
        .local_addr()
        .expect("Should always get a local address");
    let conn = NetworkConnectionHandle::connect(
        NetworkConnectionConfig {
            expect_first_message: false,
            ..Default::default()
        },
        server_addr,
    )
    .await?;
    debug!("Connected to server");

    // The server won't connect until we send a message.
    let msg = Message::new(
        MessageKind::Event,
        MessageIdentifier {
            namespace: 1,
            id: 2,
        },
        Cow::Borrowed(&[1, 2, 3]),
    );
    conn.send_message(&msg)?;
    debug!("Message sent");

    let server_conn = server.await_connection().await?.unwrap();
    debug!("Got server connection");

    let got_msg = wait_for_message(&server_conn).await?;
    debug!("Got message sent to server");

    assert_eq!(msg, got_msg);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ping_pong() {
    let res = tokio::time::timeout(std::time::Duration::from_secs(5), ping_pong_impl()).await;
    assert!(res.is_ok(), "{:?}", res);
    let inner_res = res.unwrap();
    assert!(inner_res.is_ok(), "{:?}", inner_res);
}
