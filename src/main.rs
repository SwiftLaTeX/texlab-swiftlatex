use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg};
use futures::channel::mpsc;
use futures::prelude::*;
use jsonrpc::MessageHandler;
use std::error::Error;
use std::sync::Arc;
use stderrlog::{ColorChoice, Timestamp};
use texlab::server::LatexLspServer;
use texlab_distro::Distribution;
use texlab_protocol::{LatexLspClient, LspCodec};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = app_from_crate!()
        .author("")
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity"),
        )
        .arg(
            Arg::with_name("quiet")
                .long("quiet")
                .short("q")
                .help("No output printed to stderr"),
        )
        .get_matches();

    stderrlog::new()
        .module(module_path!())
        .module("jsonrpc")
        .module("texlab_citeproc")
        .module("texlab_completion")
        .module("texlab_distro")
        .module("texlab_hover")
        .module("texlab_protocol")
        .module("texlab_symbol")
        .module("texlab_syntax")
        .module("texlab_workspace")
        .verbosity(matches.occurrences_of("verbosity") as usize)
        .quiet(matches.is_present("quiet"))
        .timestamp(Timestamp::Off)
        .color(ColorChoice::Never)
        .init()
        .unwrap();

    let mut listener = TcpListener::bind("127.0.0.1:9998").await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        tokio::spawn(accept_connection(socket, addr));
    }
}

async fn accept_connection(mut socket: TcpStream, addr: std::net::SocketAddr) {
    println!("hello there! start serving {}", addr);
    let (reader, writer) = socket.split();
    let mut stdout = FramedWrite::new(writer, LspCodec);
    let mut stdin = FramedRead::new(reader, LspCodec);
    let (stdout_tx, mut stdout_rx) = mpsc::channel(0);
    let distro = Arc::new(Distribution::detect().await);
    let client = Arc::new(LatexLspClient::new(stdout_tx.clone()));
    let server = Arc::new(LatexLspServer::new(
        Arc::clone(&client),
        Arc::clone(&distro),
    ));
    let mut stdout_tx_shutdown = stdout_tx.clone();
    let mut handler = MessageHandler {
        server: Arc::clone(&server),
        client: Arc::clone(&client),
        output: stdout_tx,
    };

    tokio::join!(
        async move {
            loop {
                let message = stdout_rx.next().await.unwrap();
                if message == "kill" {
                    break;
                }
                let status = stdout.send(message).await;
                match status {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        },
        async move {
            while let Some(json) = stdin.next().await {
                match &json {
                    Ok(jsonmsg) => handler.handle(jsonmsg).await,
                    Err(_) => {
                        break;
                    }
                }
            }
            stdout_tx_shutdown.send("kill".to_string()).await.unwrap();
            println!("Connection break {}", addr);
        }
    );

    println!("Connection cleanup! {}", addr);
}