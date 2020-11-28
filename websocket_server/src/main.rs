use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::Receiver,
};

use tokio::time;

use futures_util::SinkExt;

use tokio_tungstenite::WebSocketStream;

use std::collections::HashMap;

use log::info;
use tracing::{instrument, Level};

use models::{Client, ControlMessages, MessageDirection};

#[instrument()]
async fn send_message(stream: &mut WebSocketStream<TcpStream>, message: tungstenite::Message) {
    match message {
        tungstenite::Message::Binary(message) => {
            info!(
                "Sending BINARY Message From Server To Client : {:?}",
                &message
            );
            stream
                .send(tungstenite::Message::Binary(message))
                .await
                .unwrap();
        }
        tungstenite::Message::Text(message) => {
            info!(
                "Sending TEXT Message From Server To Client : {:?}",
                &message
            );
            stream
                .send(tungstenite::Message::Text(message))
                .await
                .unwrap();
        }
        tungstenite::Message::Close(_)
        | tungstenite::Message::Ping(_)
        | tungstenite::Message::Pong(_) => {
            //
        }
    }
}

#[instrument]
async fn establish_and_maintain_each_client_ws_connection(
    mut tx_server_state_manager: mpsc::Sender<(ControlMessages, Option<mpsc::Sender<ControlMessages>>)>,

    stream: TcpStream,
) {

    let (goes_to_specific_ws_client_tx, mut goes_to_specific_ws_client_rx) =  mpsc::channel::<ControlMessages>(10);

    let this_client = Client {
        username: None,
        email: None,
        user_id: uuid::Uuid::new_v4().to_string(),
        current_socket_addr: Some(stream.peer_addr().unwrap()),
    };


    tx_server_state_manager
        .send((ControlMessages::ServerInitiated(this_client.clone()), Some(goes_to_specific_ws_client_tx)))
        .await
        .expect("The connection was closed before even getting to update the status within a system. The odds of this happening normally are extremely low... Like someone would have to connection and then almost instantaneously close the connection:[");

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept websocket.");

    send_message(&mut ws_stream, tungstenite::Message::binary(
        ControlMessages::ServerInitiated(this_client.clone()).serialize()
    )).await;


    loop {
        tokio::select! {

            control_message = goes_to_specific_ws_client_rx.next() => {
            if control_message.is_some() {
                ws_stream.send(tungstenite::Message::Binary(control_message.unwrap().serialize())).await.unwrap();

            }
            },
            val = ws_stream.try_next() => {
                let val = val.unwrap();
                if val.is_some() { info!("{:?}", val); }

                if val.unwrap().is_close() {
            info!("The client is trying to close the connection");



            tx_server_state_manager
                .send((ControlMessages::ClosedConnection(this_client.clone()),None))
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
async fn server_global_state_manager(mut global_state_update_transceiver: Receiver<(ControlMessages, Option<mpsc::Sender<ControlMessages>>)> )  {
    let mut online_connections = HashMap::<Client, mpsc::Sender<ControlMessages>>::new();

    let (status_processer_notifier_tx, mut status_processer_notifier_rx) = mpsc::channel::<i32>(10);

    tokio::spawn(async move { game_loop(status_processer_notifier_tx, 60).await });

    loop {
        tokio::select! {

            game_notifier = status_processer_notifier_rx.next() => {
                if game_notifier.is_some() {
                    println!("The new round is starting!! {} ", game_notifier.unwrap());
                }

            },

            some_connection = global_state_update_transceiver.recv() => {
                let (control_message, client_controller_channel) = some_connection.unwrap();

            info!("Received connection in status_manager");

            match control_message {
                ControlMessages::ServerInitiated(client) => {

                    let mut client_connection = client_controller_channel.unwrap();


                    client_connection.send(ControlMessages::ClientInfo(client.clone())).await.unwrap();

                    online_connections.insert(client, client_connection);


                }
                ControlMessages::ClientInfo(_client) => {
                    // ? Needs to be more specific what this should do on the server... maybe this problem will be taken care of when the ControlMessages are segmented based on where the message should be interpretted
                }
                ControlMessages::Message(_message, _direction) => {
                    // This is exactly why the controlMessages need to be segmented based on where they're needed!
                }
                ControlMessages::OnlineClients(_clients, _round_number) => {
                    // oof just not useful.
                }
                ControlMessages::ReadyForPartner(client) => {
                    info!("{:?} would like to get partner please", client.user_id);
                }
                ControlMessages::ClosedConnection(client) => {

                    online_connections.remove_entry(&client);

                }
                
                
            }
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

    let (global_state_updater_tx, global_state_updater_rx) =
        mpsc::channel::<(ControlMessages, Option<mpsc::Sender<ControlMessages>>)>(10);

    // Connection status manager, user/connection states are updated here but no "functionality" is really performed.
    tokio::spawn(async {
        info!("setting up a status manager");
        server_global_state_manager(global_state_updater_rx).await
    });

    // websocket manager
    while let Some(Ok(stream)) = listener.next().await {
        let global_state_updater_tx_clone = global_state_updater_tx.clone();


        // this is a point in the program where the channel should be split up:
        // Both the tx and rx should will be sent to ws_connection. The tx part will then be sent to the status updater as a message

        tokio::spawn(async {
            establish_and_maintain_each_client_ws_connection(
                global_state_updater_tx_clone,
                stream,
            )
            .await
        });
    }
}
