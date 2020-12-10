use bincode;
use serde::{Serialize, Deserialize};

use std::collections::HashSet;
use std::net::SocketAddr;




#[derive(Debug, Serialize,Deserialize, Eq, Hash, Clone)]
pub struct Client {
    pub username: Option<String>,
    pub email: Option<String>,
    pub user_id: uuid::Uuid,
    // This will only be set to None if the websocket connection is not yet initialized... Not sure this ever actually happens?
    pub current_socket_addr : Option<SocketAddr>
}

impl PartialEq for Client {
    fn eq(&self, other : &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl Client {
    /// This function is mainly used for comparison...
    pub fn from_user_id(user_id : uuid::Uuid) -> Client {
        Client{username: None, user_id, email: None, current_socket_addr : None}
    }

    pub fn replace_with_newer_values(&mut self, new : Client) -> Result<(), String>{
        if self.user_id == new.user_id {
            self.username = new.username;
            self.email = new.email;
            self.current_socket_addr = new.current_socket_addr;
            Ok(())
        }
        else {
            Err(format!("The new client did not equal the old one!"))
        }
    }
}


#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq)]
pub enum Entity {
     Client,
     Server
}
#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq)]
pub enum DetailedEntity {
    Client(uuid::Uuid),
    Server
}

#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq)]
pub struct InformationFlow {
    pub sender: DetailedEntity,
    pub receiver: DetailedEntity
}

impl InformationFlow{
    fn switch_direction(&self) -> Self {
        Self{
            sender: self.receiver.clone(),
            receiver: self.sender.clone(),
        }
    }
}

#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq)]
pub enum MessageDirection {
    BetweenClientAndServer(InformationFlow),
    /// The first client will be the sender and the second will be the receiver 
    BetweenClientAndClient(InformationFlow)
}

impl MessageDirection {
    pub fn switch_direction(&self) -> Self {
        match self {
            MessageDirection::BetweenClientAndClient(information_flow) => {
                MessageDirection::BetweenClientAndClient(information_flow.switch_direction())
            }
            MessageDirection::BetweenClientAndServer(information_flow) => {
                MessageDirection::BetweenClientAndServer(information_flow.switch_direction())
            }
        }
    }
}

type RoundNumber = u64;


#[derive(Debug, Serialize,Deserialize, Clone)]
/// Server Commands are received on the server, created by the client. They need to be processed on the server ONLY.
pub enum ServerCommand {
    /// This indicates that this client is ready to be paired at whatever future round, the server will respond with a Self::OnlineClients variant
    ReadyForPartner(Client),

}


#[derive(Debug, Serialize,Deserialize, Clone)]
///These commands are meant to be processed by the clients
pub enum ClientCommand {
    /// When the server is initiated, the server sends this to the client and the client responds in turn (of course, changing the MessageDirection).
    ServerInitiated(Client),
    /// This will show the client the available users on any particular round
    OnlineClients(HashSet<Client>, RoundNumber),
}

#[derive(Debug, Serialize,Deserialize, Clone)]
///These commands are meant to be processed by the client and the server. These are usually for the purpose of upgrading connections to webRTC or for passing messages between clients 
pub enum ServerAndClientCommand{
   /// This is sent from the server to the client in order to uniquely identify the client... Will need to store this in a database
   ClientInfo(Client),
   /// The Message Direction contains the sender and intended receiver
   Message(String, MessageDirection),
   /// The string contains the content of the sdp message
   SdpRequest(String, MessageDirection),
   /// The receiver in the message direction is the client that initially sent the SDP Request 
   SdpResponse(String, MessageDirection),
    /// This is used for ending the websocket connection between the client and the server. The message direction indicates who has initiated the closure.
    ClosedConnection(Client),
}


#[derive(Debug, Serialize,Deserialize, Clone)]
pub enum Command {
ServerCommand(ServerCommand),
ClientCommand(ClientCommand),
ServerAndClientCommand(ServerAndClientCommand)
}

#[derive(Debug, Serialize,Deserialize, Clone)]
pub struct Envelope{
    sender: Entity,
    receiver: Entity,
    command: Command
}

impl Envelope {
    

    pub fn serialize(&self) -> Vec<u8>{
        match bincode::serialize(self) {
            Ok(vec) => {vec},
            Err(oh_no) => {panic!(oh_no)}
        }
    }

    pub fn deserialize(bytes : &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(bytes)
    }

    pub fn switch_direction(&self) -> Self {
        Self {
            sender : self.receiver.clone(),
            receiver : self.sender.clone(),
            command : self.command.switch_direction(),
        }
    }
}

impl Command {
    pub fn switch_direction(&self) -> Self {
        match self {
            Command::ClientCommand(_) | Command::ServerCommand(_) => {self.clone()}
            Command::ServerAndClientCommand(server_and_client_command) => {
                match server_and_client_command {
                    ServerAndClientCommand::ClientInfo(_client) => self.clone(),
                    ServerAndClientCommand::Message(message, message_direction) => { Command::ServerAndClientCommand(ServerAndClientCommand::Message(message.clone(), message_direction.switch_direction()))},
                    ServerAndClientCommand::SdpRequest(message, message_direction) => { Command::ServerAndClientCommand(ServerAndClientCommand::SdpRequest(message.clone(), message_direction.switch_direction()))},
                    ServerAndClientCommand::SdpResponse(message, message_direction) => { Command::ServerAndClientCommand(ServerAndClientCommand::SdpResponse(message.clone(), message_direction.switch_direction()))},
                    ServerAndClientCommand::ClosedConnection(_client) => {self.clone()}
                }
            }

        }
    }

}