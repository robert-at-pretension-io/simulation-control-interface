use bincode;
use serde::{Serialize, Deserialize};

use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum EntityTypes {
    Client,
    Server
}

#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct Entity {
     entity_type : EntityTypes,
     entity_detail: EntityDetails
}

impl Entity {
    pub fn new(entity_detail : EntityDetails) -> Entity {
        let entity_type = match entity_detail {
            EntityDetails::Client(uuid) => EntityTypes::Client,
            EntityDetails::Server => EntityTypes::Server
        };

        Entity {
            entity_type,
            entity_detail
        }
    }
    pub fn get_uuid(&self) -> Option<uuid::Uuid> {
        match self.entity_detail {
            EntityDetails::Client(uuid) => {Some(uuid)},
            EntityDetails::Server => {None}
        }
    }
}


#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum EntityDetails {
    Client(uuid::Uuid),
    Server
}



type RoundNumber = u64;




#[derive(Debug, Serialize,Deserialize, Clone)]
pub enum Command {
    /// This indicates that this client is ready to be paired at whatever future round, the server will respond with a Self::OnlineClients variant
    ReadyForPartner(Client),    
    /// When the server is initiated, the server sends this to the client and the client responds in turn (of course, changing the MessageDirection).
    ServerInitiated(Client),
    /// This will show the client the available users on any particular round
    OnlineClients(HashSet<Client>, RoundNumber),
   /// This is sent from the server to the client in order to uniquely identify the client... Will need to store this in a database
   ClientInfo(Client),
   /// The string contains the content of the sdp message
   SdpRequest(String),
   /// The receiver in the message direction is the client that initially sent the SDP Request 
   SdpResponse(String),
    /// This is used for ending the websocket connection between the client and the server. The message direction indicates who has initiated the closure.
    ClosedConnection(Client),
}

#[derive(Debug, Serialize,Deserialize, Clone)]
pub struct Envelope{
    pub sender: Entity,
    pub intermediary: Option<Entity>,
    pub receiver: Entity,
    pub command: Command
}

impl Envelope {
    pub fn new(sender: EntityDetails, receiver: EntityDetails, intermediary: Option<EntityDetails>, command : Command) -> Envelope {
        Envelope {
            sender : Entity::new(sender),
            receiver : Entity::new(receiver),
            intermediary : intermediary.map_or(None, |ent| Some(Entity::new(ent))),
            command
        }
    }

    

    

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
            intermediary : self.intermediary.clone(),
            receiver : self.sender.clone(),
            command : self.command.clone()
        }
    }
}
