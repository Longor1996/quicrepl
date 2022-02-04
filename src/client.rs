use super::*;
use clap::Parser;

pub async fn main(args: &MainArgs, addr: SocketAddr, stop: StopTx) -> Result<()> {
    info!("Starting client...");
    
    let mut roots = rustls::RootCertStore::empty();
    
    let cert = args.cert.clone().unwrap_or_else(|| "./cert.der".into());
    let cert = tokio::fs::read(cert).await?;
    let cert = rustls::Certificate(cert);
    
    roots.add(&cert).context("adding certificate to temporary ca-root")?;
    
    let client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    
    let mut config = quinn::ClientConfig::new(Arc::new(client_crypto));
    Arc::get_mut(&mut config.transport)
        .unwrap()
        .keep_alive_interval(std::time::Duration::from_secs(1).into());
    
    let mut endpoint = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
    
    endpoint.set_default_client_config(config);
    info!("Created local endpoint: {:?}", endpoint.local_addr());
    
    let hostname = args.host.clone().unwrap_or_else(|| "localhost".to_owned());
    info!("Connecting to {}, assuming hostname '{}' ...", addr, hostname);
    let conn = endpoint.connect(addr, &hostname)?;
    
    let quinn::NewConnection {
        connection,
        mut uni_streams,
        mut bi_streams,
        mut datagrams,
        ..
    } = conn.await?;
    
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
