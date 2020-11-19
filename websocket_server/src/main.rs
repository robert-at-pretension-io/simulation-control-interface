use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::mpsc;

use tokio::time;

use futures_util::SinkExt;

use tokio_tungstenite::WebSocketStream;

use std::collections::HashMap;

use std::net::SocketAddr;

use log::info;
use tracing::{instrument, Level};

use models::ControlMessages;


#[derive(Debug, PartialEq, Eq)]
enum ConnectionCommand {
    ClosedConnection(SocketAddr),
    Online,
    //NeedsWebRtcUpgrade,
    //WithPartner,
    WaitingForPartner,
}

impl Default for ConnectionCommand {
    fn default() -> Self {
        ConnectionCommand::Online
    }
}

#[derive(Debug)]
struct Connection {
    connection_id: String,
    ip_address: SocketAddr,
    ws_mpsc_transmitter: mpsc::Sender<String>,
}

impl Connection {
    fn new(
        connection_id: String,
        ip_address: SocketAddr,
        ws_mpsc_transmitter: mpsc::Sender<String>,
    ) -> Self {
        Connection {
            connection_id,
            ip_address,
            ws_mpsc_transmitter,
        }
    }
}
#[instrument()]
async fn send_message(stream : &mut WebSocketStream<TcpStream>,message : tungstenite::Message) {
    match message {
        tungstenite::Message::Binary(message) => {
            info!("Sending BINARY Message From Server To Client : {:?}", &message);
            stream.send(tungstenite::Message::Binary(message)).await.unwrap();
        }
        tungstenite::Message::Text(message) => {
            info!("Sending TEXT Message From Server To Client : {:?}", &message);
            stream.send(tungstenite::Message::Text(message)).await.unwrap();
        }
        tungstenite::Message::Close(_) | tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => {
            //
        }
    }
    
    
}


#[instrument]
async fn ws_connection(
    mut tx_status_manager: mpsc::Sender<(ConnectionCommand, Option<Connection>)>,
    process_complete_channel_tx_rx: (mpsc::Sender<String>, mpsc::Receiver<String>),
    stream: TcpStream,
    //ws_transmitter: mpsc::Sender<String>,
) {
    let random_user_uuid = uuid::Uuid::new_v4().to_string();

    let (tx, mut rx) = process_complete_channel_tx_rx;

    let ip_address = stream.peer_addr().unwrap();

    let this_connection = Connection::new(random_user_uuid.clone(), ip_address, tx);

    tx_status_manager
        .send((ConnectionCommand::Online, Some(this_connection)))
        .await
        .expect("The connection was closed before even getting to update the status within a system. The odds of this happening normally are extremely low... Like someone would have to connection and then almost instantaneously close the connection:[");

    info!("new client! Let's try upgradding them to a websocket connection on port 80!");

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept websocket.");

    info!("successfully upgraded connection to stream.");

    // let message = tungstenite::Message::text(String::from("hello from the server!"));

    let message = tungstenite::Message::binary(
        ControlMessages::ServerInitiated.serialize() );

    send_message(&mut ws_stream, message).await;

    // ws_stream.send(message).await.unwrap();
    // ws_stream.flush().await.unwrap();

    let address = ws_stream.get_ref().peer_addr().unwrap();

    loop {
        tokio::select! {

            val = rx.next() => {
            if val.is_some() {

                info!("looky! {:?}... \n This indicates that the status manager is able to comunicate back to the client through the websocket!", &val);

                let message = tungstenite::Message::text(format!("The server acknowledges that you're online! Here's the message that was sent:\n {}", val.unwrap()));

                ws_stream.send(message).await.unwrap();

            }
            },
            val = ws_stream.try_next() => {
                let val = val.unwrap();
                if val.is_some() { info!("{:?}", val); }

                if val.unwrap().is_close() {
            info!("The client is trying to close the connection");

            tx_status_manager
                .send((ConnectionCommand::ClosedConnection(address.clone()),None))
                .await
                .expect("The connection was closed :[");
            break; // I think this break makes it so that the function returns :o?
        }

            },
        };
    }
}

#[instrument]
async fn game_loop(
    mut status_processer_notifier: tokio::sync::mpsc::Sender<i32>,
    round_interval: u64,
) {
    let mut interval = time::interval(time::Duration::from_secs(round_interval));
    let mut round_number: i32 = 1;

    loop {
        interval.tick().await;
        status_processer_notifier
            .send(round_number)
            .await
            .expect("how could you fail?! Just send the new round notification");
        round_number = round_number + 1;
    }
}

#[instrument]
async fn status_manager(mut status_updater_rx: tokio::sync::mpsc::Receiver<(ConnectionCommand,Option<Connection>)>) {
    let mut online_connections = HashMap::<SocketAddr, Connection>::new();

    //Connection

    let (status_processer_notifier_tx, mut status_processer_notifier_rx) 
    = mpsc::channel::<i32>(10);

    tokio::spawn(async move { game_loop(status_processer_notifier_tx, 60).await });

    loop {
        tokio::select! {

            game_notifier = status_processer_notifier_rx.next() => {
                if game_notifier.is_some() {
                    println!("The new round is starting!! {} ", game_notifier.unwrap());
                }

            },

            some_connection = status_updater_rx.recv() => {
                let (connection_command, optional_connection) = some_connection.unwrap();

            info!("Received connection in status_manager");

            match connection_command {
                ConnectionCommand::Online => {
                    let connection = optional_connection.unwrap();
    
                    let connection_socketaddr = connection.ip_address.clone();
                    let clone_ip = connection_socketaddr.clone();
                    let clone_connection_id = connection.connection_id.clone();

                    online_connections.insert(connection_socketaddr, connection);



                    let temp_connection = online_connections.get_mut(&clone_ip).unwrap();
                    temp_connection.ws_mpsc_transmitter.send(format!("Ack from server, here's your id: {}", String::from(clone_connection_id))).await.unwrap();

                }
                ConnectionCommand::WaitingForPartner => {
                    info!("Would like to get partner please");
                }
                ConnectionCommand::ClosedConnection(socketaddr) => {
                    //online_connections.retain(|k,v| {e.connection_status != ConnectionCommand::ClosedConnection })


                    online_connections.remove_entry(&socketaddr);

                    // connection_status.insert(connection.ip_address, ConnectionCommand::ClosedConnection).expect("A connection must have a pre-existing status before closing?... right?");
                }
            }
            info!("All connections status:");
            online_connections.iter().for_each(|v| {
                info!("{} has status: {:?}", v.0, connection_command);
            });
            }



        }
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

    let (status_updater_tx, status_updater_rx) = mpsc::channel::<(ConnectionCommand, Option<Connection>)>(10);

    // Connection status manager, user/connection states are updated here but no "functionality" is really performed.
    tokio::spawn(async {
        info!("setting up a status manager");
        status_manager(status_updater_rx).await
    });

    // websocket manager
    while let Some(Ok(stream)) = listener.next().await {
        let status_updater_tx_clone = status_updater_tx.clone();

        let process_complete_channel_tx_rx = mpsc::channel(10);

        // this is a point in the program where the channel should be split up:
        // Both the tx and rx should will be sent to ws_connection. The tx part will then be sent to the status updater as a message

        tokio::spawn(async {
            ws_connection(
                status_updater_tx_clone,
                process_complete_channel_tx_rx,
                stream,
            )
            .await
        });
    }
}
