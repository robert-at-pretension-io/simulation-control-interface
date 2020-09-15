use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio::sync::{broadcast, mpsc, oneshot};


//use std::io;
use tokio_tungstenite;


#[tokio::main]
async fn main()  {
    let mut listener = TcpListener::bind("127.0.0.1:80").await.unwrap();

    //let mut WebSocketStreamVector = Vec::<tokio_tungstenite::WebSocketStream<S>>::new();

    tokio::spawn(async move {

        while let Some(stream) = listener.next().await {
            match stream {
                Ok(stream) => {
                    println!("new client! Let's try upgradding them to a websocket connection on port 80!");
                    match tokio_tungstenite::accept_async(stream).await {
                        Err(e) => {println!("sad times... an error occurred:\n {:?}", e);}
                        Ok(mut WebSocketStream) => {
                            println!("Seems it ACTUALLY worked... now just to do something with the stream");
                            let address = WebSocketStream.get_mut().peer_addr().unwrap();
                            println!("Peer at address {} has connected.", address);
                        }
                    }
    
    
                }
                Err(e) => { /* connection failed */ }
            }
        }

    }).await.unwrap();
}