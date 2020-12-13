use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::Receiver,
};

use tokio::time;

use futures_util::SinkExt;

use tokio_tungstenite::WebSocketStream;

use std::collections::{HashMap, HashSet};

use tungstenite::Message;

use log::{info, warn};
use tracing::{instrument, Level};

use models::{
    Client, Command,  Entity, Envelope, EntityDetails, EntityTypes
};

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
    mut tx_server_state_manager: mpsc::Sender<(Envelope, Option<mpsc::Sender<Envelope>>)>,

    stream: TcpStream,
) {
    let (goes_to_specific_ws_client_tx, mut goes_to_specific_ws_client_rx) =
        mpsc::channel::<Envelope>(10);

    let this_client = Client {
        username: None,
        email: None,
        user_id: uuid::Uuid::new_v4(),
        current_socket_addr: Some(stream.peer_addr().unwrap()),
    };

    tx_server_state_manager
        .send((Envelope{command: Command::ClientCommand(ClientCommand::ServerInitiated(this_client.clone())),sender: Entity::Client, receiver: Entity::Server}, Some(goes_to_specific_ws_client_tx)))
        .await
        .expect("The connection was closed before even getting to update the status within a system. The odds of this happening normally are extremely low... Like someone would have to connection and then almost instantaneously close the connection:[");

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept websocket.");

    send_message(
        &mut ws_stream,
        tungstenite::Message::binary(
            Envelope {
                command: Command::ClientCommand(ClientCommand::ServerInitiated(
                    this_client.clone(),
                )),
                sender: Entity::Server,
                receiver: Entity::Client,
            }
            .serialize(),
        ),
    )
    .await;

    loop {
        tokio::select! {

            control_message = goes_to_specific_ws_client_rx.next() => {
            if control_message.is_some() {

                ws_stream.send(tungstenite::Message::Binary(control_message.unwrap().serialize())).await.unwrap();

            }
            },
            // This is the interface for incoming messages from the client to the server... Errors can be dealt with here before sending the message off to the server global state manager
            val = ws_stream.try_next() => {
                let val = val.unwrap();

                if let Some(value) = val {
                    match value {
                        Message::Text(text) => {info!("received text: {:?}", text);},
                        Message::Binary(bin) => {
                            match Envelope::deserialize(&bin) {
                                Ok(control_message) => {
                                    tx_server_state_manager.send((control_message, None)).await.unwrap();
                                },
                                Err(oh_boy) => {info!("Error receiving message from ws client: {:?}", oh_boy)}
                            }
                        },
                        Message::Close(reason) => {
            info!("The client is trying to close the connection for the following reason: {:?}", reason);
            let package = Envelope{command: Command::ServerAndClientCommand(ServerAndClientCommand::ClosedConnection(this_client.clone())), sender: Entity::Client, receiver: Entity::Server};
            tx_server_state_manager
                .send((package,None))
                .await
                .expect("The connection was closed :[");
                break // gets out of the loop... should deallocate everything?


                        }
                        Message::Ping(_)  => {
                            info!("ping message received")
                        }
                        Message::Pong(_) => {
                            info!("pong message received")
                        }
                    }

                }



            },
        };
    }
}

#[instrument]
async fn send_messages_to_all_online_clients(
    clients: &mut Vec<mpsc::Sender<Envelope>>,
    message: Envelope,
) {
    for client in clients.iter_mut() {
        let message = message.clone();
        client.send(message).await.unwrap();
    }
}

#[instrument]
async fn game_loop(
    mut status_processer_notifier: tokio::sync::mpsc::Sender<u64>,
    round_interval: u64,
) {
    let mut interval = time::interval(time::Duration::from_secs(round_interval));
    let mut round_number: u64 = 1;

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
async fn server_global_state_manager(
    mut global_state_update_transceiver: Receiver<(Envelope, Option<mpsc::Sender<Envelope>>)>,
) {
    let mut online_connections = HashMap::<uuid::Uuid, (Client, mpsc::Sender<Envelope>)>::new();

    let (status_processer_notifier_tx, mut status_processer_notifier_rx) = mpsc::channel::<u64>(10);

    tokio::spawn(async move { game_loop(status_processer_notifier_tx, 60).await });

    let mut current_round = 0;

    loop {
        tokio::select! {

            game_notifier = status_processer_notifier_rx.next() => {
                if game_notifier.is_some() {
                    current_round = game_notifier.unwrap();
                    println!("The new round is starting!! {} ", game_notifier.unwrap());
                }

            },

            some_connection = global_state_update_transceiver.recv() => {
                let (control_message, client_controller_channel) = some_connection.unwrap();

            info!("Received connection in status_manager");
            let first_clone = control_message.clone();

                if control_message.receiver == Entity::Server {
                    

                }
                else {
                    info!("The following was received by the server but not addressed to the server: {:?}", first_clone);
                }

            // match control_message {
                
            //     Envelope::SdpRequest(sdp, message_direction) => {
            //         info!("Received SdpRequest message with sdp: {:?}",sdp);
            //         match message_direction {
            //             MessageDirection::ClientToClient(flow) => {
            //                 info!("resending the following information: {:?}",first_clone );
            //                 let receiver = flow.receiver.clone();
            //                 online_connections.get_mut(&receiver).unwrap().1.send(first_clone.switch_direction()).await.unwrap();
            //             }
            //             MessageDirection::ClientToServer(_) | MessageDirection::ServerToClient(_) => {
            //                 info!("This type of message should only be between clients in order to setup sdprequest/response/ice-handling");
            //             }
            //         }
            //     }
            //     Envelope::SdpResponse(sdp,message_direction) => {

            //     }
            //     Envelope::ServerInitiated(client) => {

            //         let mut client_connection = client_controller_channel.unwrap();

            //         let message_direction = MessageDirection::ServerToClient(client.clone());
            //         client_connection.send(Envelope::ClientInfo(message_direction)).await.unwrap();
            //         let client_id = client.user_id;

            //         online_connections.insert(client_id, (client, client_connection));

            //         let (mut clients, mut client_connections) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();


            //         send_messages_to_all_online_clients(&mut client_connections, Envelope::OnlineClients(clients, current_round)).await

            //     }
            //     Envelope::ClientInfo(message_direction) => {
            //         match message_direction {
            //             MessageDirection::ClientToClient(_) | MessageDirection::ServerToClient(_) => {info!("Not sure how to implement this");}
            //             MessageDirection::ClientToServer(client) => {

            //                 info!("received the following updated client info: {:?}", client);
            //                 match online_connections.get_mut(&client.user_id) {
            //                     Some((old_client, client_connection)) => {
            //                         old_client.replace_with_newer_values(client).unwrap()
            //                     },
            //                     None => {
            //                         // nooo
            //                     }
            //                 }

            //                 let (mut clients, mut client_connections) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();


            //         send_messages_to_all_online_clients(&mut client_connections, Envelope::OnlineClients(clients, current_round)).await
            //             }
            //         }
            //         // ? Needs to be more specific what this should do on the server... maybe this problem will be taken care of when the Envelope are segmented based on where the message should be interpretted
            //     }
            //     Envelope::Message(_message, _direction) => {
            //         // This is exactly why the controlMessages need to be segmented based on where they're needed!
            //     }
            //     Envelope::OnlineClients(_clients, _round_number) => {
            //         // oof just not useful.
            //     }
            //     Envelope::ReadyForPartner(client) => {
            //         info!("{:?} would like to get partner please", client.user_id);
            //         let keys : HashSet<uuid::Uuid> = online_connections.keys().cloned().collect();
            //         let (mut clients, _) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();


            //         let (_, temp_client_connection) = online_connections.get_mut(&client.user_id).unwrap();
            //         temp_client_connection.send(Envelope::OnlineClients(clients, current_round)).await.unwrap();
            //     }
            //     Envelope::ClosedConnection(client) => {

            //         online_connections.remove_entry(&client.user_id);

            //         let keys : HashSet<uuid::Uuid> = online_connections.keys().cloned().collect();
            //         let (mut clients, mut client_connections) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();

            //         send_messages_to_all_online_clients(&mut client_connections, Envelope::OnlineClients(clients, current_round)).await
            //     }


            // }
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
        mpsc::channel::<(Envelope, Option<mpsc::Sender<Envelope>>)>(10);

    tokio::spawn(async {
        info!("setting up a status manager");
        server_global_state_manager(global_state_updater_rx).await
    });

    // websocket manager
    while let Some(Ok(stream)) = listener.next().await {
        let global_state_updater_tx_clone = global_state_updater_tx.clone();

        tokio::spawn(async {
            establish_and_maintain_each_client_ws_connection(global_state_updater_tx_clone, stream)
                .await
        });
    }
}
