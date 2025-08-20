use std::{sync::Arc, time::Duration};

use russh::{
    client::{self, Handle, Handler},
    keys::{PublicKey, load_secret_key},
};
use tokio::net::ToSocketAddrs;
use tracing::info;

use crate::{
    HiveLibError,
    errors::SshError,
    hive::node::{Node, Target},
};

struct Client {}

impl From<russh::Error> for HiveLibError {
    fn from(value: russh::Error) -> Self {
        HiveLibError::SshError(SshError::RusshError(value))
    }
}

impl Handler for Client {
    type Error = HiveLibError;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

struct Session {
    session: Handle<Client>,
}

impl Session {
    async fn connect(node: &Node) -> Result<(), HiveLibError> {
        let host = node.target.get_preffered_host()?;
        let key_pair = load_secret_key("/home/marsh/.ssh/id_ed25519.pub", None)
            .map_err(|err| HiveLibError::SshError(SshError::KeysError(err)))?;

        // load ssh certificate
        // let mut openssh_cert = None;
        // if openssh_cert_path.is_some() {
        //     openssh_cert = Some(load_openssh_certificate(openssh_cert_path.unwrap())?);
        // }

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::MAX),
            ..<_>::default()
        });
        let sh = Client {};

        let mut session = client::connect(config, node.target.as_preferred_as_tuple()?, sh).await?;

        Ok(())
    }
}
