pub mod config;
mod error;
mod protocol;
mod sftp_session;
mod ssh_keys;
mod storage;
mod try_buf;

use crate::config::DrayConfig;
use anyhow::{Error, bail};
use bytes::Bytes;
use futures::{
    future::{ready, Ready},
    Future,
};
use log::{debug, error, info};
use protocol::request::Request;
use sftp_session::SftpSession;
use tokio::sync::RwLock;
use std::{convert::TryFrom, pin::Pin, sync::Arc};
use storage::{s3::S3ObjectStorage, ObjectStorage};
use thrussh::{
    server::{run, Auth, Config, Handler, Server, Session},
    ChannelId, CryptoVec,
};
use thrussh_keys::{
    key::{self, KeyPair},
    PublicKeyBase64,
};

pub struct DraySshServer {
    dray_config: Arc<DrayConfig>,
    object_storage: Arc<dyn ObjectStorage>,
    sftp_session: RwLock<Option<SftpSession>>,
}

impl DraySshServer {
    pub fn new(dray_config: DrayConfig) -> DraySshServer {
        let object_storage = Arc::from(S3ObjectStorage::new(&dray_config.s3));

        DraySshServer {
            dray_config: Arc::from(dray_config),
            object_storage: object_storage.clone(),
            sftp_session: RwLock::from(Option::None),
        }
    }

    pub async fn health_check(&self) -> Result<(), Error> {
        self.object_storage.health_check().await
    }

    pub async fn run_server(self) -> Result<(), Error> {
        let ssh_config = Config {
            keys: vec![KeyPair::generate_ed25519().unwrap()],
            ..Default::default()
        };

        let ssh_config = Arc::new(ssh_config);

        run(ssh_config, &self.dray_config.host.clone(), self)
            .await
            .map_err(Error::from)
    }

    async fn auth_publickey(
        self,
        user: String,
        public_key: key::PublicKey,
    ) -> Result<(DraySshServer, Auth), Error> {
        let authorized_keys = match self
            .object_storage
            .get_authorized_keys_fingerprints(&user)
            .await
        {
            Ok(authorized_keys) => authorized_keys,
            Err(error) => {
                error!(
                    "Error during public key authentication for {}: {}",
                    user, error
                );
                return Err(error);
            }
        };

        let public_key_fingerprint = public_key.fingerprint();

        match authorized_keys.contains(&public_key_fingerprint) {
            true => {
                info!(
                    "Successfully authenticated {} with public key authentication",
                    user
                );

                {
                    let mut sftp_session = self.sftp_session.write().await;
                    *sftp_session = Some(SftpSession::new(self.object_storage.clone(), user));
                }

                Ok((self, Auth::Accept))
            }
            false => {
                info!("Rejected public key authentication attempt from {}", user);
                Ok((self, Auth::Reject))
            }
        }
    }

    async fn data(self, channel: ChannelId, request: Request, mut session: Session) -> Result<(DraySshServer, Session), Error> {
        {
            let sftp_session = &*self.sftp_session.read().await;
            
            let sftp_session = match sftp_session {
                Some(sftp_session) => sftp_session,
                None => bail!("Missing SFTP session!"),
            };

            let response = sftp_session.handle_request(request).await;

            session.data(channel, CryptoVec::from(Bytes::from(&response).to_vec()));
        }

        Ok((self, session))
    }
}

impl Server for DraySshServer {
    type Handler = Self;

    fn new(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        DraySshServer {
            dray_config: self.dray_config.clone(),
            object_storage: self.object_storage.clone(),
            sftp_session: RwLock::from(Option::None),
        }
    }
}

impl Handler for DraySshServer {
    type Error = Error;

    #[allow(clippy::type_complexity)]
    type FutureAuth =
        Pin<Box<dyn Future<Output = Result<(DraySshServer, Auth), Self::Error>> + Send>>;

    type FutureBool = Ready<Result<(Self, Session, bool), anyhow::Error>>;

    type FutureUnit = Pin<Box<dyn Future<Output = Result<(Self, Session), anyhow::Error>> + Send>>;

    fn auth_publickey(self, user: &str, public_key: &key::PublicKey) -> Self::FutureAuth {
        let public_key = key::parse_public_key(&public_key.public_key_bytes()).unwrap();
        Box::pin(self.auth_publickey(user.to_owned(), public_key))
    }

    fn subsystem_request(
        self,
        channel: ChannelId,
        name: &str,
        mut session: Session,
    ) -> Self::FutureUnit {
        if "sftp" == name {
            debug!("starting sftp subsystem");
            session.channel_success(channel);
        } else {
            debug!("failed to start unsupported subsystem {}", name);
            session.channel_failure(channel);
        }

        Box::pin(ready(Ok((self, session))))
    }

    fn data(self, channel: ChannelId, data: &[u8], mut session: Session) -> Self::FutureUnit {
        match Request::try_from(data) {
            Ok(request) => Box::pin(self.data(channel, request, session)),
            Err(_) => {
                let response_bytes = Bytes::from(&SftpSession::build_invalid_request_message_response()).to_vec();
                session.data(channel, CryptoVec::from(response_bytes));
                Box::pin(ready(Ok((self, session))))
            }
        }
    }

    fn finished_bool(self, b: bool, session: Session) -> Self::FutureBool {
        ready(Ok((self, session, b)))
    }

    fn finished(self, session: Session) -> Self::FutureUnit {
        Box::pin(ready(Ok((self, session))))
    }

    fn finished_auth(self, auth: Auth) -> Self::FutureAuth {
        Box::pin(ready(Ok((self, auth))))
    }
}
