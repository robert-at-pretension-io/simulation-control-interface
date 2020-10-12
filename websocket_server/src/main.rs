use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::mpsc;


use futures_util::SinkExt;

use tokio_tungstenite;

use std::collections::HashMap;


use std::net::SocketAddr;

use tracing::{instrument, Level};
use tracing_subscriber::{fmt::format::FmtSpan};
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
    ws_mpsc_transmitter: Option<mpsc::Sender<String>>,
}

impl Connection {
    fn new(
        ip_address: SocketAddr,
        connection_status: Option<ConnectionStatus>,
        ws_mpsc_transmitter: Option<mpsc::Sender<String>>,
    ) -> Self {
        Connection {
            connection_status: connection_status.unwrap_or(ConnectionStatus::default()),
            ip_address,
            ws_mpsc_transmitter,
        }
    }
}

#[instrument]
async fn ws_connection(
    mut tx_status_manager: mpsc::Sender<Connection>,
     process_complete_channel_tx_rx : (mpsc::Sender<String>,mpsc::Receiver<String>),
    stream: TcpStream,
    //ws_transmitter: mpsc::Sender<String>,
) {

    let (tx, mut rx) = process_complete_channel_tx_rx;

    let ip_address = stream.peer_addr().unwrap();

    tx_status_manager
        .send(Connection::new(ip_address, None,Some(tx)))
        .await
        .expect("The connection was closed before even getting to update the status within a system. The odds of this happening normally are extremely low... Like someone would have to connection and then almost instantaneously close the connection:[");

    info!("new client! Let's try upgradding them to a websocket connection on port 80!");

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept websocket.");

    info!("successfully upgraded connection to stream.");

    let message = tungstenite::Message::text(String::from("hello from the server!"));

    ws_stream.send(message).await.unwrap();

    let address = ws_stream.get_ref().peer_addr().unwrap();


    loop { 
        tokio::select! {

            val = rx.next() => {
            if val.is_some() {

                info!("looky! {:?}... \n This indicates that the status manager is able to comunicate back to the client through the websocket!", &val);

                let message = tungstenite::Message::text(String::from("The server acknowledges that you're online!"));     
                
                ws_stream.send(message).await.unwrap();
            
            }
            },
            val = ws_stream.try_next() => {
                let val = val.unwrap();
                if val.is_some() { info!("{:?}", val); }
        if val.unwrap().is_close() {
            info!("The client is trying to close the connection");

            tx_status_manager
                .send(Connection::new(
                    ip_address,
                    Some(ConnectionStatus::ClosedConnection),
                    None
                ))
                .await
                .expect("The connection was closed :[");
            break;
        }

            },
        };
    }


}

#[instrument]
async fn status_manager(mut status_updater_rx :  tokio::sync::mpsc::Receiver<Connection>) {

    let mut connection_status = HashMap::<SocketAddr, ConnectionStatus>::new();

    while let Some(connection) = status_updater_rx.recv().await {

        info!("Received connection in status_manager");

        match connection.connection_status {
            ConnectionStatus::Online => {

                connection.ws_mpsc_transmitter.unwrap().send(String::from("sweet sweet acknowledgement!")).await;

                // ws_mpsc_transmitter
                //     .send(String::from("Added the connection"))
                //     .await
                //     .expect("fugck, couldn't send the message");
                connection_status.insert(connection.ip_address, connection.connection_status);
                
            }
            ConnectionStatus::NeedsWebRtcUpgrade => {
                info!("Would like to ");
            }
            ConnectionStatus::ClosedConnection => {
                connection_status.insert(connection.ip_address, ConnectionStatus::ClosedConnection).expect("A connection must have a pre-existing status before closing?... right?");
            }
        }
        info!("All connections status:");
        connection_status.iter().for_each(|v| {
            info!("{} has status: {:?}", v.0, v.1);
        });
    }

}

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .compact()
    .with_level(true)
    .with_target(false)
    //.with_span_events(FmtSpan::FULL)
    .init();
    
    

    let mut listener = TcpListener::bind("127.0.0.1:80").await.unwrap();


    
    let (status_updater_tx, status_updater_rx) = mpsc::channel::<Connection>(10);
    
    // Connection status manager, user/connection states are updated here but no "functionality" is really performed.
    tokio::spawn(async {
        info!("setting up a status manager");
        status_manager(status_updater_rx).await
    });

    // websocket manager
    while let Some(Ok(stream)) = listener.next().await {
        // let mut tx_clone = tx.clone();
        let status_updater_tx_clone = status_updater_tx.clone();
    
        let process_complete_channel_tx_rx = mpsc::channel(10);

        // this is a point in the program where the channel should be split up:
        // Both the tx and rx should will be sent to ws_connection. The tx part will then be sent to the status updater as a message

        tokio::spawn(async  { ws_connection(status_updater_tx_clone, process_complete_channel_tx_rx ,stream).await });
    }



    // ----------   Process handler   ----------
    // This needs two mpsc channels.
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
