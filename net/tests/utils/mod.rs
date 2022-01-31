use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

use ammo_framer::Message;

use ammo_net::Connection;

pub async fn wait_for_message(conn: &Arc<dyn Connection>) -> Result<Message<'static>> {
    let mut msg = None;

    loop {
        conn.receive_messages(&mut |x| {
            msg = Some(x.clone_static());
            Ok(false)
        })?;

        if let Some(m) = msg {
            return Ok(m);
        }

        log::debug!("Looping while waiting for message");
        sleep(Duration::from_millis(1)).await;
    }
}
