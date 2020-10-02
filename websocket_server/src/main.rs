use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::{ mpsc, oneshot};

use futures_util::SinkExt;

use tokio_tungstenite;

use std::net::SocketAddr;

#[derive(Debug)]
enum ConnectionStatus {
    Offline,
    Online,
    NeedsWebRtcUpgrade,
    WithPartner,
    WaitingForPartner
}


impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Online
    }
}

#[derive(Debug)]
struct Connection {
    connection_status : ConnectionStatus,
    ip_address : SocketAddr
}

impl Connection {
    fn new(ip_address : SocketAddr, connection_status : Option<ConnectionStatus>) -> Self {
        Connection {
            connection_status : connection_status.unwrap_or(ConnectionStatus::default()),
            ip_address
        }
    }
}

// async fn add_connection(ip_address : String) -> {
//     let new_connection = Connection::new(IpAddr::from(ip_address), connection_status);
//     let (mut tx, mut rx) = watch::channel::<Connection>(10);
// }


async fn ws_connection(mut rx : mpsc::Sender<Connection>, stream : TcpStream) {
    println!("Inside the ws_connection function.");

    let ip_address = stream.peer_addr().unwrap();
                    
    rx.send(Connection::new(ip_address, None)).await;
                        println!("new client! Let's try upgradding them to a websocket connection on port 80!");
    
                        
    
                        let mut ws_stream = tokio_tungstenite::accept_async(stream).await.expect("failed to accept websocket.");
    
                        let message =  tungstenite::Message::text(String::from("hello from the server!"));
                                
                        ws_stream.send(message).await.unwrap();
    
                        let address = ws_stream.get_ref().peer_addr().unwrap();
    
                        while let Some(stuff) = ws_stream.try_next().await.unwrap() {
                            if (stuff.is_close()) {println!("The client is trying to close the connection"); todo!() }
                            println!("The server says: ooooo boy, someone to talk to! The person at {} said \"{}\"", address, stuff.to_string());
                        }
}

#[tokio::main]
async fn main()  {
    let mut listener = TcpListener::bind("127.0.0.1:80").await.unwrap();

    let (mut status_updater_tx,mut status_updater_rx) = mpsc::channel::<Connection>(10);

    // Connection status manager
tokio::spawn(async move {
    let mut connection_vec = Vec::<Connection>::new();
    while let response = status_updater_rx.recv().await {
        match response {
            Some(connection) => {
                connection_vec.push(connection);
                println!("Total connections:\n");
                connection_vec.iter().for_each(|v | {println!("{}",v.ip_address);});
            }
            None => {println!("The connection seems to be closed.")}
        }
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

            println!("made it inside the while loop!");
            let tx_clone = status_updater_tx.clone();
            tokio::spawn(  async {ws_connection( tx_clone, stream).await});
        }
}
