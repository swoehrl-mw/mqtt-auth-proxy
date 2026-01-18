use std::sync::Arc;

use mqtt5_protocol::packet::MqttPacket;
use mqtt5_protocol::packet::connect::ConnectPacket;
use mqtt5_protocol::{FixedHeader, PacketType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

use crate::auth::handle_auth_packet;
use crate::config::{AuthMethod, Config};

pub async fn handle_connection(socket: TcpStream, config: Config) {
    tracing::info!("New connection from {:?}", socket.peer_addr());
    let remote = format!("{}:{}", config.remote.host, config.remote.port);
    let Ok(remote) = TcpStream::connect(remote).await else {
        return;
    };
    tracing::info!(
        "Established connection with remote at {:?}",
        remote.peer_addr()
    );
    // Handle TLS
    if let Some(tls) = config.tls
        && tls.enabled
    {
        let mut root_store =
            RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        if let Some(ca_cert) = tls.ca_cert {
            let cert = CertificateDer::from_pem_file(ca_cert).unwrap();
            root_store.add(cert).unwrap();
        }
        let tls_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(tls_config));
        let dnsname = ServerName::try_from(config.remote.host).unwrap();
        let remote = connector.connect(dnsname, remote).await.unwrap();
        tracing::info!("Started tls for connection");
        let mut handler = ConnectionHandler::new(socket, remote, config.auth.map(|a| a.auth)).await;
        handler.handle().await;
    } else {
        let mut handler = ConnectionHandler::new(socket, remote, config.auth.map(|a| a.auth)).await;
        handler.handle().await;
    }
}

struct ConnectionHandler<L, R>
where
    L: AsyncReadExt + AsyncWriteExt + Unpin,
    R: AsyncReadExt + AsyncWriteExt + Unpin,
{
    local: L,
    remote: R,
    auth: Option<AuthMethod>,
}

impl<L, R> ConnectionHandler<L, R>
where
    L: AsyncReadExt + AsyncWriteExt + Unpin,
    R: AsyncReadExt + AsyncWriteExt + Unpin,
{
    pub async fn new(local: L, remote: R, auth: Option<AuthMethod>) -> Self {
        ConnectionHandler {
            local,
            remote,
            auth,
        }
    }
    pub async fn handle(&mut self) {
        let mut buffer_local = [0; 1024 * 10];
        let mut buffer_remote = [0; 1024 * 10];

        loop {
            tokio::select! {
                Ok(n) = self.local.read(&mut buffer_local[..]) => {
                    if n == 0 {
                        tracing::info!("Socket closed from local side. Ending connection");
                        return;
                    }
                    self.packet_from_local(&mut buffer_local[..n]).await;
                }
                Ok(n) = self.remote.read(&mut buffer_remote[..]) => {
                    if n == 0 {
                        tracing::info!("Socket closed from remote side. Ending connection");
                        return;
                    }
                    self.packet_from_remote(&mut buffer_remote[..n]).await;
                }
            }

            if self.auth.is_none() {
                // no more auth to intercept/perform, switch to more efficient mode
                break;
            }
        }
        match tokio::io::copy_bidirectional(&mut self.local, &mut self.remote).await {
            Ok((ab, ba)) => {
                tracing::info!("Ending connection. Copied {ab} byts from local to remote and {ba} bytes from remote to local");
            }
            Err(e) => tracing::warn!("Ending connection. Got error while proxying: {e}")
        }
    }

    async fn packet_from_local(&mut self, buffer: &mut [u8]) {
        tracing::debug!("Received packet with len {} from local", buffer.len());
        if let Some(auth) = self.auth.as_ref()
            && let Ok(header) = FixedHeader::decode(&mut &buffer[..])
            && header.packet_type == PacketType::Connect
            && let Ok(mut connect) = ConnectPacket::decode_body(
                &mut &buffer[(buffer.len() - header.remaining_length as usize)..],
                &header,
            )
        {
            handle_auth_packet(auth.clone(), &mut connect);
            let mut connect_buffer = vec![];
            connect.encode(&mut connect_buffer).unwrap();
            self.remote.write_all(&connect_buffer).await.unwrap();
            let _ = self.auth.take();
        } else {
            self.remote.write_all(buffer).await.unwrap();
        }
    }

    async fn packet_from_remote(&mut self, buffer: &mut [u8]) {
        tracing::debug!("Received packet with len {} from remote", buffer.len());
        self.local.write_all(buffer).await.unwrap();
    }
}
