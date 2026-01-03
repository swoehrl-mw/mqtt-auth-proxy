mod auth;
pub mod config;
mod proxy;

use tokio::net::TcpListener;

pub async fn run_proxy(config: config::Config) -> std::io::Result<()> {
    let listener = TcpListener::bind(
        config
            .listener
            .clone()
            .unwrap_or_else(|| "127.0.0.1:1883".to_owned()),
    )
    .await?;
    tracing::info!("Started listener on {:?}", listener.local_addr());
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(proxy::handle_connection(socket, config.clone()));
    }
}
