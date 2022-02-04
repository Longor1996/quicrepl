use std::{path::PathBuf, net::SocketAddr};
use clap::Parser;

/// quicrepl - The simple quic repl tool.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct MainArgs {
    /// TLS certificate in PEM format
    #[clap(parse(from_os_str), short = 'c', long = "cert", env = "QUICREPL_CERT")]
    pub cert: Option<PathBuf>,
    
    /// TLS private key in PEM format
    #[clap(parse(from_os_str), short = 'k', long = "key", requires = "cert", env = "QUICREPL_KEY")]
    pub key: Option<PathBuf>,
    
    /// Hostname of the certificate/remote
    #[clap(short = 'h', long = "host", env = "QUICREPL_HOST")]
    pub host: Option<String>,
    
    /// Shall we be a client or a server?
    #[clap(subcommand)]
    pub what: ClientOrServer,
}

impl MainArgs {
    pub fn get() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Parser)]
pub enum ClientOrServer {
    /// Start as a client and connect to a server.
    Client {
        /// Address to connect to
        #[structopt(default_value = "[::1]:4433", env = "QUICREPL_ADDR")]
        addr: SocketAddr,
    },
    
    /// Start as a server and respond to clients.
    Server {
        /// Address to listen on
        #[structopt(default_value = "[::1]:4433", env = "QUICREPL_ADDR")]
        addr: SocketAddr,
    },
}
