use quinn::{NewConnection, SendStream, RecvStream};

use super::*;

pub async fn main(args: &MainArgs, addr: SocketAddr, stop: StopTx) -> Result<()> {
    info!("Starting server...");
    
    let (cert, pkey) = server_get_certs(args).await?;
    
    let server_crypto = {
        let mut sc = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert, pkey)?;
        if !args.alpn.is_empty() {
            sc.alpn_protocols.extend(args.alpn.iter().map(|p|p.as_bytes().to_owned()));
        }
        sc
    };
    
    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
    
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .keep_alive_interval(std::time::Duration::from_secs(1).into())
        .max_idle_timeout(Some(std::time::Duration::from_secs(5).try_into()?))
        .max_concurrent_uni_streams(0_u8.into());
    
    let (endpoint, mut incoming) = quinn::Endpoint::server(server_config, addr).context("Opening server-socket/endpoint")?;
    info!("Now listening on {}", endpoint.local_addr()?);
    
    let cli_stop = stop.clone();
    tokio::task::spawn_blocking(move || {
        use rustyline::{*, error::ReadlineError};
        let mut rl = Editor::<()>::new();
        loop {
            match rl.readline(">> ") {
                Ok(_line) => {
                    //
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
        
        cli_stop.send(()).expect("failed to tell mainloop to stop");
    });
    
    let mut main_stop_rx = stop.subscribe();
    loop {
        let conn = tokio::select! {
            biased; Some(conn) = incoming.next() => conn,
            _ = main_stop_rx.recv() => {
                info!("Received stop signal.");
                endpoint.close(quinn::VarInt::default(), "server shutting down".as_bytes());
                break
            },
            else => continue
        };
        
        let remote = conn.remote_address();
        let client_stop = stop.clone();
        info!("Client {remote} is connecting...");
        tokio::spawn(async move {
            if let Err(err) = server_handle(conn, client_stop).await {
                error!("Client {remote} disconnected due to an error: {err:?}");
            } else {
                info!("Client {remote} disconnected.");
            }
        });
    }
    
    endpoint.wait_idle().await;
    info!("Server stopped; goodbye!");
    Ok(())
}

async fn server_handle(conn: quinn::Connecting, stop: StopTx) -> Result<()> {
    let remote = conn.remote_address();
    let NewConnection {
        connection,
        mut uni_streams,
        mut bi_streams,
        mut datagrams,
        ..
    } = conn.await?;
    
    let mut mainloop_stop = stop.subscribe();
    while let Err(tokio::sync::broadcast::error::TryRecvError::Empty) = mainloop_stop.try_recv() {
        tokio::select! {
            biased; _ = mainloop_stop.recv() => break,
            
            Some(bi) = bi_streams.next() => match bi {
                Ok(bi) => {
                    tokio::spawn(server_handle_bi_stream(connection.clone(), stop.clone(), bi));
                },
                Err(e) => {error!("Unable to handle incoming bi-stream by {remote}: {:?}", e); break},
            },
            
            Some(uni) = uni_streams.next() => match uni {
                Ok(uni) => {
                    tokio::spawn(server_handle_uni_stream(connection.clone(), stop.clone(), uni));
                },
                Err(e) => {error!("Unable to handle incoming uni-stream by {remote}: {:?}", e); break},
            },
            
            Some(dg) = datagrams.next() => match dg {
                Ok(dg) => {
                    tokio::spawn(server_handle_datagram(connection.clone(), stop.clone(), dg));
                },
                Err(e) => {error!("Unable to handle incoming datagram by {remote}: {:?}", e); break},
            },
            
            else => continue
        };
    }
    
    connection.close(quinn::VarInt::default(), "Client closed connection".as_bytes());
    Ok(())
}

async fn server_handle_bi_stream(conn: quinn::Connection, stop: Arc<tokio::sync::broadcast::Sender<()>>, streams: (SendStream, RecvStream)) -> Result<()> {
    let remote = conn.remote_address();
    let mut stop = stop.subscribe();
    
    let (mut cmd_tx, mut cmd_rx) = streams;
    
    while let Err(tokio::sync::broadcast::error::TryRecvError::Empty) = stop.try_recv() {
        // Await request...
        let mut bytes_len = [0u8; 4];
        
        match cmd_rx.read_exact(&mut bytes_len).await {
            Ok(r) => r,
            Err(quinn::ReadExactError::FinishedEarly) => break,
            Err(e) => Err(e).context("reading request length")?,
        }
        
        let bytes_len = u32::from_be_bytes(bytes_len) as usize;
        
        if bytes_len == 0 {
            break
        }
        
        let mut bytes = Vec::from_iter(std::iter::repeat(0u8).take(bytes_len)).into_boxed_slice();
        cmd_rx.read_exact(&mut bytes).await.context("reading request")?;
        
        let bytes = std::str::from_utf8(&bytes).context("converting server response bytes into text")?;
        debug!("[{remote}] >> {bytes}");
        
        let bytes = format!("Hello, {remote}!");
        debug!("[{remote}] << {bytes}");
        cmd_tx.write_all(&(bytes.len() as u32).to_be_bytes()).await.context("writing response length")?;
        cmd_tx.write_all(bytes.as_bytes()).await.context("writing response")?;
    }
    
    cmd_tx.finish().await?;
    Ok(())
}

async fn server_handle_uni_stream(conn: quinn::Connection, stop: Arc<tokio::sync::broadcast::Sender<()>>, stream: RecvStream) -> Result<()> {
    todo!()
}

async fn server_handle_datagram(conn: quinn::Connection, stop: Arc<tokio::sync::broadcast::Sender<()>>, datagram: bytes::Bytes) -> Result<()> {
    todo!()
}

async fn server_get_certs(args: &MainArgs) -> Result<(Vec<rustls::Certificate>, rustls::PrivateKey)> {
    
    if let (Some(key_path), Some(cert_path)) = (&args.key, &args.cert) {
        let key = tokio::fs::read(key_path).await.context("failed to read private key")?;
        
        let key = if key_path.extension().map_or(false, |x| x == "der") {
            rustls::PrivateKey(key)
        } else {
            let pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut &*key)
                .context("malformed PKCS #8 private key")?;
            match pkcs8.into_iter().next() {
                Some(x) => rustls::PrivateKey(x),
                None => {
                    let rsa = rustls_pemfile::rsa_private_keys(&mut &*key)
                        .context("malformed PKCS #1 private key")?;
                    match rsa.into_iter().next() {
                        Some(x) => rustls::PrivateKey(x),
                        None => {
                            anyhow::bail!("no private keys found");
                        }
                    }
                }
            }
        };
        
        let cert_chain = tokio::fs::read(cert_path).await.context("failed to read certificate chain")?;
        let cert_chain = if cert_path.extension().map_or(false, |x| x == "der") {
            vec![rustls::Certificate(cert_chain)]
        } else {
            rustls_pemfile::certs(&mut &*cert_chain)
                .context("invalid PEM-encoded certificate")?
                .into_iter()
                .map(rustls::Certificate)
                .collect()
        };
        
        return Ok((cert_chain, key))
    }
    
    warn!("Neither certificate nor key are provided; using self-signed cert/key-pair...");
    
    let path = std::env::current_dir().expect("no current directory to place generated cert/key-pair in");
    let cert_path = path.join("cert.der");
    let key_path = path.join("key.der");
    info!("Cert/Key-Directory: {path:?}");
    
    let cert = tokio::fs::read(&cert_path).await;
    let key = tokio::fs::read(&key_path).await;
    
    if let (Ok(cert), Ok(key)) = (cert, key) {
        info!("Found and loaded self-signed certificate and key, reusing...");
        let key = rustls::PrivateKey(key);
        let cert = rustls::Certificate(cert);
        return Ok((vec![cert], key))
    }
    
    info!("Generating self-signed certificate...");
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let key = cert.serialize_private_key_der();
    let cert = cert.serialize_der().unwrap();
    tokio::fs::create_dir_all(&path).await.context("failed to create certificate directory")?;
    
    info!("Saving self-signed certificate to {cert_path:?} ...");
    tokio::fs::write(&cert_path, &cert).await.context("failed to write certificate")?;
    
    info!("Saving self-signed private key to {key_path:?} ...");
    tokio::fs::write(&key_path, &key).await.context("failed to write private key")?;
    
    let key = rustls::PrivateKey(key);
    let cert = rustls::Certificate(cert);
    Ok((vec![cert], key))
}
