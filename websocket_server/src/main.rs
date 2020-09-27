use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::{broadcast, mpsc, oneshot};

use std::sync::Arc;
use tokio::sync::Mutex;

use futures_util::SinkExt;

use tokio_tungstenite;

use std::net::IpAddr;

enum ConnectionStatus {
    Offline,
    Online,
    WithPartner,
    WaitingForPartner
}


impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Online
    }
}

struct Connection {
    connection_status : ConnectionStatus,
    ip_address : IpAddr
}

impl Connection {
    fn new(ip_address : IpAddr, connection_status : Option<ConnectionStatus>) -> Self {
        Connection {
            connection_status : connection_status.unwrap_or(ConnectionStatus::default()),
            ip_address
        }
    }
}

#[tokio::main]
async fn main()  {
    let mut listener = TcpListener::bind("127.0.0.1:80").await.unwrap();


    let shared_connections = Arc::new(Mutex::new(Vec::<Connection>::new()));

    let (mut new_connection_channel_tx,mut new_connection_channel_rx) = mpsc::channel::<ConnectionStatus>::(10);

    tokio::spawn(async move {

        while let Some(stream) = listener.next().await {
            match stream {
                Err(e) => {println!("{:?}", e)}
                Ok(mut stream) => {

                    

                    println!("new client! Let's try upgradding them to a websocket connection on port 80!");

                    

                    let mut ws_stream = tokio_tungstenite::accept_async(stream).await.expect("failed to accept websocket.");

                    let message =  tungstenite::Message::text(String::from("hello from the server!"));
                            
                    ws_stream.send(message).await.unwrap();

                    let address = ws_stream.get_ref().peer_addr().unwrap();

                    while let Some(stuff) = ws_stream.try_next().await.unwrap() {
                        println!("The server says: ooooo boy, someone to talk to! The person at {} said {}", address, stuff);
                    }
                }
            }
        }
    }).await.unwrap();
}
