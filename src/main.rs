use std::{net::SocketAddr, sync::Arc};
use futures_util::stream::StreamExt;
use anyhow::{Result, Context};
use tracing::{error, warn, info, debug, Level};
use args::MainArgs;

pub mod client;
pub mod server;
pub mod args;

pub type StopTx = Arc<tokio::sync::broadcast::Sender<()>>;
pub type StopRx = tokio::sync::broadcast::Receiver<()>;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok(); // ignore errors
    
    use args::{MainArgs, ClientOrServer};
    let args = MainArgs::get();
    
    use tracing_subscriber::FmtSubscriber;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    
    // TODO: Make tokio runtime configurable.
    
    let (stop_tx, _) = tokio::sync::broadcast::channel(16);
    let stop_tx = Arc::new(stop_tx);
    let stop_tx_ctrlc = stop_tx.clone();
    ctrlc::set_handler(move || {
        stop_tx_ctrlc.send(()).expect("failed to tell mainloop to stop");
    })?;
    
    match &args.what {
        ClientOrServer::Client { addr, system_certs } => client::main(&args, addr.clone(), *system_certs, stop_tx).await,
        ClientOrServer::Server { addr } => server::main(&args, *addr, stop_tx).await,
    }
}
