use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::mpsc;

use futures_util::SinkExt;

use tokio_tungstenite;

use std::collections::HashMap;


use std::net::SocketAddr;

use tracing::{event, instrument, Instrument};
use tracing_subscriber;
use log::info;

#[derive(Debug)]
enum ConnectionStatus {
    ClosedConnection,
    Online,
    NeedsWebRtcUpgrade,
    //WithPartner,
    //WaitingForPartner
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Online
    }
}

#[derive(Debug)]
struct Connection {
    connection_status: ConnectionStatus,
    ip_address: SocketAddr,
    // ws_oneshot_transmitter: Option<mpsc::Sender<String>>,
}

impl Connection {
    fn new(
        ip_address: SocketAddr,
        connection_status: Option<ConnectionStatus>,
        // ws_oneshot_transmitter: Option<mpsc::Sender<String>>,
    ) -> Self {
        Connection {
            connection_status: connection_status.unwrap_or(ConnectionStatus::default()),
            ip_address,
            // ws_oneshot_transmitter,
        }
    }
}

#[instrument]
async fn ws_connection(
    mut tx_status_manager: mpsc::Sender<Connection>,
    stream: TcpStream,
    //ws_transmitter: mpsc::Sender<String>,
) {

    let ip_address = stream.peer_addr().unwrap();

    tx_status_manager
        .send(Connection::new(ip_address, None))
        .await
        .expect("The connection was closed :[");
    info!("new client! Let's try upgradding them to a websocket connection on port 80!");

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept websocket.");

    let message = tungstenite::Message::text(String::from("hello from the server!"));

    ws_stream.send(message).await.unwrap();

    let address = ws_stream.get_ref().peer_addr().unwrap();

    while let Some(stuff) = ws_stream.try_next().await.unwrap() {
        if stuff.is_close() {
            info!("The client is trying to close the connection");

            tx_status_manager
                .send(Connection::new(
                    ip_address,
                    Some(ConnectionStatus::ClosedConnection),
                    
                ))
                .await
                .expect("The connection was closed :[");
            break;
        }
        info!(
            "The server says: ooooo boy, someone to talk to! The person at {} said \"{}\"",
            address,
            stuff.to_string()
        );
    }
}

#[instrument]
async fn status_manager(mut status_updater_rx :  tokio::sync::mpsc::Receiver<Connection>) {

    let mut connection_status = HashMap::<SocketAddr, ConnectionStatus>::new();

    while let Some(connection) = status_updater_rx.recv().await {
        match connection.connection_status {
            ConnectionStatus::Online => {
                // let mut ws_oneshot_transmitter = connection
                //     .ws_oneshot_transmitter
                //     .expect("The connection must go through!");

                // ws_oneshot_transmitter
                //     .send(String::from("Added the connection"))
                //     .await
                //     .expect("fugck, couldn't send the message");
                connection_status.insert(connection.ip_address, connection.connection_status);
                
            }
            ConnectionStatus::NeedsWebRtcUpgrade => {
                todo!();
            }
            ConnectionStatus::ClosedConnection => {
                connection_status.insert(connection.ip_address, ConnectionStatus::ClosedConnection).expect("A connection must have a pre-existing status before closing?... right?");
            }
        }
        println!("All connections status:");
        connection_status.iter().for_each(|v| {
            println!("{} has status: {:?}", v.0, v.1);
        });
    }

}

#[tokio::main]
async fn main() {

    //tracing_subscriber::fmt::Sub
    

    let mut listener = TcpListener::bind("127.0.0.1:80").await.unwrap();


    
    let (status_updater_tx, status_updater_rx) = mpsc::channel::<Connection>(10);

    // websocket manager
    while let Some(Ok(stream)) = listener.next().await {
        // let mut tx_clone = tx.clone();
        let status_updater_tx_clone = status_updater_tx.clone();
        tokio::spawn(async  { ws_connection(status_updater_tx_clone, stream).await });
    }

    // Connection status manager, user/connection states are updated here but no "functionality" is really performed.
    tokio::spawn(async {
        status_manager(status_updater_rx).await
    });

    // ----------   Process handler   ----------
    // This needs two oneshot channels.
    // 1. status manager -> process handler
    // 2. process handler -> websocket manager
    //
    // This will enable the status to be updated ONLY by the status manager so there is a "ground-truth" of connection state
    //
    // This method of processing the new states will also allow the process handler to spawn new green threads as needed

    // let (mut tx, mut rx) = mpsc::channel::<String>(10);

    // tokio::spawn(async move {
    //     match rx.recv().await {
    //         Some(stuff) => {println!("Got the following stuff: {:?}", stuff);},
    //         None => {println!("🤷");} 
            
    //     }
    // }).await;



}
