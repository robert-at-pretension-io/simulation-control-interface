use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::{ mpsc, oneshot};

use futures_util::SinkExt;

use tokio_tungstenite;

use std::collections::HashMap;

use std::net::SocketAddr;

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
    connection_status : ConnectionStatus,
    ip_address : SocketAddr,
    ws_oneshot_transmitter : Option<oneshot::Sender<String>>,
}

impl Connection {
    fn new(ip_address : SocketAddr, connection_status : Option<ConnectionStatus>, ws_oneshot_transmitter : Option<oneshot::Sender<String>>) -> Self {
        Connection {
            connection_status : connection_status.unwrap_or(ConnectionStatus::default()),
            ip_address,
            ws_oneshot_transmitter
        }
    }
}

// async fn add_connection(ip_address : String) -> {
//     let new_connection = Connection::new(IpAddr::from(ip_address), connection_status);
//     let (mut tx, mut rx) = watch::channel::<Connection>(10);
// }


async fn ws_connection(mut tx_status_manager : mpsc::Sender<Connection>, stream : TcpStream, mut ws_receiver : oneshot::Receiver<String>, ws_transmitter : oneshot::Sender<String>) {
    println!("Inside the ws_connection function.");

    let ip_address = stream.peer_addr().unwrap();

    while let Ok(stuff) = ws_receiver.try_recv() {
        println!("Got the following stuff: {:?}", stuff);
    }
                    
    tx_status_manager.send(Connection::new(ip_address, None, Some(ws_transmitter))).await.expect("The connection was closed :[");
                        println!("new client! Let's try upgradding them to a websocket connection on port 80!");
    
                        
    
                        let mut ws_stream = tokio_tungstenite::accept_async(stream).await.expect("failed to accept websocket.");
    
                        let message =  tungstenite::Message::text(String::from("hello from the server!"));
                                
                        ws_stream.send(message).await.unwrap();
    
                        let address = ws_stream.get_ref().peer_addr().unwrap();
    
                        while let Some(stuff) = ws_stream.try_next().await.unwrap() {
                            if stuff.is_close() {println!("The client is trying to close the connection"); 

                            tx_status_manager.send(Connection::new(ip_address, Some(ConnectionStatus::ClosedConnection), None)).await.expect("The connection was closed :[");
                            break;
                        }
                            println!("The server says: ooooo boy, someone to talk to! The person at {} said \"{}\"", address, stuff.to_string());
                        }
}

#[tokio::main]
async fn main()  {
    let mut listener = TcpListener::bind("127.0.0.1:80").await.unwrap();

    let (status_updater_tx,mut status_updater_rx) = mpsc::channel::<Connection>(10);


    // Connection status manager, user/connection states are updated here but no "functionality" is really performed.
tokio::spawn(async move {
    let mut connection_status = HashMap::<SocketAddr, ConnectionStatus>::new();

    while let Some(connection) = status_updater_rx.recv().await {
       


        match connection.connection_status {
            ConnectionStatus::Online => {

                let ws_oneshot_transmitter = connection.ws_oneshot_transmitter.expect("The connection must go through!");

                ws_oneshot_transmitter.send(String::from("Added the connection")).expect("Could not send the message, the channel was closed :[");
                connection_status.insert(connection.ip_address, connection.connection_status);
                println!("Total connections:\n");
            }
            ConnectionStatus::NeedsWebRtcUpgrade => {
                todo!();
            }
            ConnectionStatus::ClosedConnection => {
                connection_status.insert(connection.ip_address, ConnectionStatus::ClosedConnection).expect("A connection must have a pre-existing status before closing?... right?");
            }
            
        }
                
                connection_status.iter().for_each(|v | {println!("{} has status: {:?}",v.0, v.1);});

        }
    

});

    // ----------   Process handler   ----------
    // This needs two oneshot channels. 
    // 1. status manager -> process handler
    // 2. process handler -> websocket manager
    //
    // This will enable the status to be updated ONLY by the status manager so there is a "ground-truth" of connection state
    //
    // This method of processing the new states will also allow the process handler to spawn new green threads as needed



        // websocket manager
        while let Some(Ok(stream)) = listener.next().await {
            let ( tx,rx) = oneshot::channel::<String>();

            println!("made it inside the while loop!");
            let status_updater_tx_clone = status_updater_tx.clone();
            tokio::spawn(  async {ws_connection( status_updater_tx_clone, stream, rx, tx).await});
        }
}
