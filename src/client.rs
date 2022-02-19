use super::*;
use clap::Parser;

pub async fn main(args: &MainArgs, addr: String, system_certs: bool, stop: StopTx) -> Result<()> {
    info!("Starting client...");
    
    let mut roots = rustls::RootCertStore::empty();
    
    let cert = args.cert.clone().unwrap_or_else(|| "./cert.der".into());
    match tokio::fs::read(&cert).await {
        Ok(cert) => {
            let cert = rustls::Certificate(cert);
            roots.add(&cert).context("adding certificate to temporary ca-root")?;
        },
        Err(err) => warn!("Failed to load certificate {cert:?}: {err}")
    }
    
    if system_certs {
        info!("Loading system root certificates...");
        let certs = rustls_native_certs::load_native_certs()?
                .drain(..)
                .map(|c|c.0)
                .collect::<Vec<_>>();
        roots.add_parsable_certificates(&certs);
    }
    
    let client_crypto = {
        info!("Creating crypto-config with {} certificate root(s) and {} ALPN protocol(s)...", roots.len(), args.alpn.len());
        
        let mut cc = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        
        cc.enable_early_data = true;
        cc.enable_sni = true;
        
        if !args.alpn.is_empty() {
            cc.alpn_protocols.extend(args.alpn.iter().map(|p|p.as_bytes().to_owned()));
        }
        
        cc
    };
    
    let endpoint = {
        let mut config = quinn::ClientConfig::new(Arc::new(client_crypto));
        Arc::get_mut(&mut config.transport)
            .unwrap()
            .keep_alive_interval(std::time::Duration::from_secs(1).into());
        
        let mut e = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
        e.set_default_client_config(config);
        e
    };
    
    info!("Created local endpoint: {:?}", endpoint.local_addr());
    
    let hostname = args.host.clone().unwrap_or_else(|| addr.clone());
    let hostname = hostname.rsplit_once(':').map(|(h,_)|h).unwrap_or(&hostname);
    
    let addr = match addr.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(err) => {
            error!("{err}");
            info!("Resolving address: {}", &addr);
            let names = tokio::net::lookup_host(addr)
                .await
                .context("resolving hostname")?
                .collect::<Vec<_>>();
            let sa = names.first().cloned().expect("no results");
            debug!("Found {} target(s)", names.len());
            debug!("Using {sa}");
            sa
        },
    };
    
    info!("Connecting to {addr}, using hostname '{hostname}'...");
    let conn = endpoint.connect(addr, hostname)
        .context("initial connection to remote server")?;
    
    let quinn::NewConnection {
        connection,
        mut uni_streams,
        mut bi_streams,
        mut datagrams,
        ..
    } = conn
        .await
        .context("completing connection to remote server")?;
    
    use rustyline::{*, error::ReadlineError};
    let mut rl = Editor::<()>::new();
    
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                
                /*
                let (mut cmd_tx, mut cmd_rx)  = conn.connection.open_bi().await?;
                
                // Write command...
                let bytes = line.as_bytes();
                cmd_tx.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
                cmd_tx.write_all(bytes).await?;
                
                // ...await response...
                let mut bytes_len = [0u8; 4];
                cmd_rx.read_exact(&mut bytes_len).await?;
                let bytes_len = u32::from_be_bytes(bytes_len) as usize;
                let mut bytes = Vec::from_iter(std::iter::repeat(0u8).take(bytes_len)).into_boxed_slice();
                cmd_rx.read_exact(&mut bytes).await?;
                
                let bytes = std::str::from_utf8(&bytes).context("converting server response bytes into text")?;
                info!("<< {bytes}");
                */
            },
            Err(ReadlineError::Interrupted) => {
                warn!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                warn!("CTRL-D");
                break
            },
            Err(err) => {
                error!("{err:?}");
                break
            }
        }
    }
    
    //cmd_tx.finish().await?;
    endpoint.close(quinn::VarInt::default(), "REPL closed".as_bytes());
    Ok(())
}

/// quicrepl client: Internal commandline.
#[derive(Debug, Parser)]
#[clap(author, about = None, long_about = None)]
pub enum ClientCommand {
    Send,
    Recv
}
