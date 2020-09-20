use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::prelude::*;



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
                        Ok(mut websocket_stream) => {
                            println!("Seems it ACTUALLY worked... now just to do something with the stream");
                            

                            let address = websocket_stream.get_ref().peer_addr().unwrap();
                            
                            println!("Peer at address {} has connected.", address);


                            match websocket_stream.get_mut().write(b"Test.").await {
                                Ok(data_size) => {println!("supposedly just wrote this much data: {}!", data_size);},
                                Err(uhhh) => {println!("oh man, look at that error... wild: {}", uhhh)}
                            }
                                
                            
                        

                        while let Some(stuff) = websocket_stream.try_next().await.unwrap() {
                            println!("The server says: ooooo boy, someone to talk to! The person at {} said {}", address, stuff);




                            

                            // println!("Of course, this simply wouldn't do so I disconnected promptly. The fucker.");
                            // websocket_stream.close(None).await;
                        }


                        }
                    }
    
    
                }
                Err(e) => { /* connection failed */ }
            }
        }

    }).await.unwrap();
}