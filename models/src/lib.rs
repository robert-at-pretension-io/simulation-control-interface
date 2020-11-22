use bincode;
use serde::{Serialize, Deserialize};

use std::collections::HashSet;
use std::net::SocketAddr;



#[derive(Debug, Serialize,Deserialize, Eq, Hash)]
pub struct Client {
    pub username: String,
    pub user_id: String,
    // This will only be set to None if the websocket connection is not yet initialized... Not sure this ever actually happens?
    pub current_socket_addr : Option<SocketAddr>
}

impl PartialEq for Client {
    fn eq(&self, other : &Self) -> bool {
        self.user_id == other.user_id
    }
}

#[derive(Debug, Serialize,Deserialize)]
pub enum MessageDirection {
    ClientToServer(Client),
    ServerToClient(Client),
    /// The first client will be the sender and the second will be the receiver 
    ClientToClient(Client,Client)
}

#[derive(Debug, Serialize,Deserialize)]
pub enum ControlMessages {
    /// When the server is initiated, the server sends this to the client and the client responds in turn (of course, changing the MessageDirection).
    ServerInitiated(Client),
    /// This is sent from the server to the client in order to uniquely identify the client... Will need to store this in a database
    Client(Client),
    // The Message Direction contains the client of interest
    Message(String, MessageDirection),
    OnlineClients(HashSet<Client>),
}

impl ControlMessages {
    pub fn serialize(&self) -> Vec<u8>{
        match bincode::serialize(self) {
            Ok(vec) => {vec},
            Err(oh_no) => {panic!(oh_no)}
        }
    }

    pub fn deserialize(bytes : &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(bytes)
    }


}