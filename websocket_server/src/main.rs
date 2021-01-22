// use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::Receiver,
};

use tokio_stream::{ StreamExt};
use futures_util::sink::SinkExt;
// use futures_util::stream::StreamExt;

// use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tokio::time;


use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

use std::{collections::{HashMap, HashSet}, net::SocketAddr};

// use tungstenite::Message;

use log::{info};
use tracing::{instrument, Level};

use models::{Client, Command, EntityDetails, EntityTypes, Envelope};

use native_tls::{Identity};
use tokio_native_tls::{ native_tls};

#[instrument()]
async fn send_message(stream: &mut WebSocketStream<tokio_native_tls::TlsStream<TcpStream>>, message: tokio_tungstenite::tungstenite::Message) {
    match message {
        tokio_tungstenite::tungstenite::Message::Binary(message) => {
            info!(
                "Sending BINARY Message From Server To Client : {:?}",
                &message
            );
            match stream
                .send(tokio_tungstenite::tungstenite::Message::Binary(message.clone()))
                .await {
                    Ok(_) => info!("sent message!"),
                    Err(err) => info!("Have error trying to send this message: {:?} \n... error: {:?}", message, err)
                }
                
        }
        tokio_tungstenite::tungstenite::Message::Text(message) => {
            info!(
                "Sending TEXT Message From Server To Client : {:?}",
                &message
            );
            match stream
                .send(tokio_tungstenite::tungstenite::Message::Text(message.clone()))
                .await
                {
                    Ok(_) => info!("sent message!"),
                    Err(err) => info!("Have error trying to send this message: {:?} \n... error: {:?}", message, err)
                }
        }
        tokio_tungstenite::tungstenite::Message::Close(_)
        | tokio_tungstenite::tungstenite::Message::Ping(_)
        | tokio_tungstenite::tungstenite::Message::Pong(_) => {
            //
        }
    }
}

#[instrument]
async fn establish_and_maintain_each_client_ws_connection(
     tx_server_state_manager: mpsc::Sender<(Envelope, Option<mpsc::Sender<Envelope>>)>,

    stream:  tokio_native_tls::TlsStream<TcpStream>,
    peer_address : SocketAddr
) {
    let (goes_to_specific_ws_client_tx, mut goes_to_specific_ws_client_rx) =
        mpsc::channel::<Envelope>(10);

        let address : Option<std::net::SocketAddr> = Some(peer_address);
    // match stream.peer_addr() {
    //     Ok(add) => {address = Some(add)},
    //     Err(err) => info!("Couldn't unwrap the stream's ip address :[ ... {:?}", err)
    // }

    let this_client = Client {
        username: None,
        email: None,
        user_id: uuid::Uuid::new_v4(),
        current_socket_addr: address,
    };

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
    .await
    .expect("failed to accept websocket.");


    let envelope = Envelope::new(
        EntityDetails::Server,
        EntityDetails::Server,
        None,
        Command::ServerInitiated(this_client.clone()),
    );

    tx_server_state_manager
        .send((envelope, Some(goes_to_specific_ws_client_tx)))
        .await
        .expect("The connection was closed before even getting to update the status within a system. The odds of this happening normally are extremely low... Like someone would have to connection and then almost instantaneously close the connection:[");


    let envelope = Envelope::new(
        EntityDetails::Server,
        EntityDetails::Client(this_client.user_id.clone()),
        None,
        Command::ServerInitiated(this_client.clone()),
    );

    send_message(
        &mut ws_stream,
        tokio_tungstenite::tungstenite::Message::binary(envelope.serialize()),
    )
    .await;


    loop {
        tokio::select! {

            control_message = goes_to_specific_ws_client_rx.recv() => {
            match control_message {
                Some(control_message) => {
                    match ws_stream.send(tokio_tungstenite::tungstenite::Message::Binary(control_message.serialize())).await {
                        Ok(_) => {info!("successfully received the control message!")},
                        Err(err) => {info!("Couldn't send the message properly due to the following err: {:?}", err)}
                    }
                }
                None => {

                }

            }

                

            
            },
            // This is the interface for incoming messages from the client to the server... Errors can be dealt with here before sending the message off to the server global state manager
            val = ws_stream.try_next() => {
                match val {
                    Ok(value) => {
                        if value.is_some() {
                        match value.unwrap() {
                                Message::Text(text) => {info!("received text: {:?}", text);},
                                Message::Binary(bin) => {
                                    match Envelope::deserialize(&bin) {
                                        Ok(control_message) => {
                                            match tx_server_state_manager.send((control_message, None)).await
                                            {
                                                Ok(_) => {},
                                                Err(err) => {info!("Received the following error: {:?}", err)}
                                        
                                            }
                                        },
                                        Err(oh_boy) => {info!("Error receiving message from ws client: {:?}", oh_boy)}
                                    }
                                },
                                Message::Close(_reason) => {
                                    return
                    
        
        
                                }
                                Message::Ping(_)  => {
                                    info!("ping message received")
                                }
                                Message::Pong(_) => {
                                    info!("pong message received")
                                }
                            }
                        }
                        
                        
    
                    }
                    Err(err) => {
                        info!("Received the following error trying to send the message: {:?}", err);
                    
                        info!("The client is trying to close the connection for the following reason: {:?}", reason);
        
                    let envelope = Envelope::new(
                        EntityDetails::Client(this_client.user_id.clone()),
                        EntityDetails::Server,
                        None,
                        Command::ClosedConnection(this_client.user_id.clone())
                    );
        
                    match tx_server_state_manager
                        .send((envelope,None))
                        .await {
                            Ok(_) => {info!("successfully closed/removed the connection!"); },
                            Err(err) => {info!("Had the following error while trying to send a ClosedConnection command to the tx_server_state_manager:\n {:?}", err);}
                        }

                    }

                    return // FUCK THAT CLIENT ANYWAYS
                }



            },
        }
    }
}

#[instrument]
async fn send_command_to_client_by_uuid(
    client: uuid::Uuid,
    command: Command,
    online_connections: &mut HashMap<uuid::Uuid, (Client, mpsc::Sender<Envelope>)>,
) {
    //let mut online_connections = HashMap::<uuid::Uuid, (Client, mpsc::Sender<Envelope>)>::new();
    let envelope = Envelope::new(
        EntityDetails::Server,
        EntityDetails::Client(client.clone()),
        None,
        command,
    );

    let (_client, connection_channel) = online_connections.get_mut(&client).expect("couldn't find client in online connections");
    
    
    match connection_channel.clone().send(envelope).await
    {
        Ok(_) => {},
        Err(err) => {info!("Received the following error: {:?}", err)}

    }
}

#[instrument]
async fn game_loop(
     status_processer_notifier: tokio::sync::mpsc::Sender<u64>,
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

async fn send_command_to_clients(client_list : HashMap::<uuid::Uuid, (Client, mpsc::Sender<Envelope>)> , command : Command) {

} 

#[instrument]
async fn server_global_state_manager(
    mut global_state_update_transceiver: Receiver<(Envelope, Option<mpsc::Sender<Envelope>>)>,
) {
    let mut online_connections = HashMap::<uuid::Uuid, (Client, mpsc::Sender<Envelope>)>::new();

    let (status_processer_notifier_tx, mut status_processer_notifier_rx) = mpsc::channel::<u64>(10);

    tokio::spawn(async move { game_loop(status_processer_notifier_tx, 60).await });

    let current_round = 0;

    loop {
        tokio::select! {

                    // This is the game time tracker... keeps track of the current round. Can be used for performing system-wide periodic behavior
                    game_notifier = status_processer_notifier_rx.recv() => {
                        
                        match game_notifier {
                            Some(current_round) => {
                                info!("The new round is starting!! {} ", current_round);
                            }
                            None => {info!("none...");}
                        }

                    },

                    some_connection = global_state_update_transceiver.recv() => {
                        if let Some((control_message, client_controller_channel) ) = some_connection {

                    info!("Received connection in status_manager");
                    let first_clone = control_message.clone();

                        if control_message.receiver.entity_type == EntityTypes::Server {
                            match control_message.command {


                        Command::Error(error) => {
                            info!("Received the following error: {:?}", error);
                        }

                        Command::SdpRequest(sdp) => {
                            info!("Received SdpRequest message with sdp: {:?}",sdp);
                        }
                        Command::SdpResponse(sdp) => {
                            info!("Received SdpRequest message with sdp: {:?}",sdp);
                        }
                        Command::ServerInitiated(client) => {

                            if let Some( client_connection) = client_controller_channel {

                            let envelope = Envelope::new(
                                EntityDetails::Server,
                                EntityDetails::Client(client.user_id.clone()),
                                None,
                                Command::ClientInfo(client.clone())
                            );

                            match client_connection.send(envelope).await
                            {
                                Ok(_) => {},
                                Err(err) => {info!("Received the following error: {:?}", err)}
                        
                            }
                            let client_id = client.user_id;

                            online_connections.insert(client_id, (client, client_connection));

                            let ( clients,  _client_connections) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();

                            let clients = clients.clone();

                            let keys : HashSet<uuid::Uuid> =  online_connections.keys().cloned().collect();

                            for uuid in keys  {
                                let clients = clients.clone();
                                send_command_to_client_by_uuid(uuid.clone(), Command::OnlineClients(clients, current_round), &mut online_connections).await
                            }
                            }

                        }
                        Command::ClientInfo(client) => {

                                    info!("received the following updated client info: {:?}", client);
                                    match online_connections.get_mut(&client.user_id) {
                                        Some((old_client, _client_connection)) => {
                                            match old_client.replace_with_newer_values(client)
                                            {
                                                Ok(_) => {},
                                                Err(err) => {info!("Couldn't replace the old_client: {:?}", err)}
                                        
                                            }
                                        },
                                        None => {
                                            // nooo
                                        }
                                    }

                                    let ( clients,  _client_connections) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();

                                    let clients = clients.clone();

                                    let keys : HashSet<uuid::Uuid>= online_connections.keys().cloned().collect();


                            for uuid in keys {
                                let clients = clients.clone();

                                send_command_to_client_by_uuid(uuid.clone(), Command::OnlineClients(clients, current_round), &mut online_connections).await
                            }
                            }
                            // ? Needs to be more specific what this should do on the server... maybe this problem will be taken care of when the Envelope are segmented based on where the message should be interpretted

                        Command::OnlineClients(_clients, _round_number) => {
                            // oof just not useful.
                        }
                        Command::AckClosedConnection(_) => {
                            // This is only useful on the clientside
                        }
                        Command::ReadyForPartner(client) => {
                            info!("{:?} would like to get partner please", client.user_id);
                            let ( clients, _) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();


        send_command_to_client_by_uuid(client.user_id, Command::OnlineClients(clients, current_round), &mut online_connections).await

                        }
                        Command::ClosedConnection(client) => {
                            info!("Before closing the connection the online connections are: {:?}", online_connections.clone());
                            {
                            match online_connections.remove_entry(&client){
                                Some((uuid,(_client, channel))) => {
                                    channel.send(Envelope::new(
                                        EntityDetails::Server,
                                        EntityDetails::Client(uuid),
                                        None,
                                        Command::AckClosedConnection(uuid)
                                    )).await.expect("Ehhh hope this doesn't break");
                                },
                                None => {

                                }
                            }
        }
                            info!("After closing the connection the online_connections are: {:?}", online_connections.clone());

                            let ( clients,  _client_connections) : (HashSet<Client>, Vec<mpsc::Sender<Envelope>>) = online_connections.values().cloned().unzip();

                            let clients = clients.clone();

                            let keys : HashSet<uuid::Uuid>= online_connections.keys().cloned().collect();
                            for uuid in keys {

                            let clients = clients.clone();
                                send_command_to_client_by_uuid(uuid.clone(), Command::OnlineClients(clients, current_round), &mut online_connections).await
                            }
                            }

                        }

                    }
                        else if control_message.intermediary.is_some() {
                            info!("the server is acting as an intermediary for the following message: {:?}", first_clone.clone());
                            let intermediary = control_message.intermediary.unwrap();
                            if intermediary.entity_type == EntityTypes::Server {

                                //pass the message to the receiver
                                if let EntityDetails::Client(receiver_uuid) = control_message.receiver.entity_detail {
                                    match online_connections.get_mut(&receiver_uuid){
                                        Some((_client, client_channel)) => {
                                            info!("Trying to re-route the message to the appropriate client.");
                                            match client_channel.send(first_clone.clone()).await
                                            {
                                                Ok(_) => info!("sent message!"),
                                                Err(err) => info!("Error: {:?}",  err)
                                            }
                                        }
                                        None => {
                                            info!("This is bad...Need to make custom error messages. I think messages could have statuses too...");

                                            panic!("At the disco");
                                        }
                                    }
                                }



                            }
                        }
                        else {
                            info!("When passing messages for the server, be sure to make sure that the server is either the receiver or the intermediary...");
                            info!("The following was received by the server but not addressed to the server: {:?}", first_clone);
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

    let  listener = TcpListener::bind("0.0.0.0:2096").await.expect("Couldn't bind to server address!");

    let (global_state_updater_tx, global_state_updater_rx) =
        mpsc::channel::<(Envelope, Option<mpsc::Sender<Envelope>>)>(10);

    tokio::spawn(async {
        info!("setting up a status manager");
        server_global_state_manager(global_state_updater_rx).await
    });

    let der = include_bytes!("../certificate.p12");
    let cert = Identity::from_pkcs12(der, "elliot").expect("identity to work..");
    let tls_acceptor =
        tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build().unwrap());


    loop {
        let tls_acceptor = tls_acceptor.clone();
        let (stream, remote_addr) = listener.accept().await.unwrap();

        info!("Accepted connection from: {}", remote_addr.clone());

        let global_state_updater_tx_clone = global_state_updater_tx.clone();

        // let stream = stream.into_std().unwrap();

        info!("Accepted connection from {}", remote_addr);

        let  tls_stream = tls_acceptor.accept(stream).await.expect("accept error");

        tokio::spawn(async move{
            establish_and_maintain_each_client_ws_connection(global_state_updater_tx_clone, tls_stream, remote_addr)
                .await
        });

    }

}
