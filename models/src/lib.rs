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
}

#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq)]
pub struct InformationFlow {
    pub sender: Client,
    pub receiver: Client
}

#[derive(Debug, Serialize,Deserialize, Clone, Eq, PartialEq)]
pub enum MessageDirection {
    ClientToServer(Client),
    ServerToClient(Client),
    /// The first client will be the sender and the second will be the receiver 
    ClientToClient(InformationFlow)
}

type RoundNumber = u64;

#[derive(Debug, Serialize,Deserialize, Clone)]
pub enum ControlMessages {
    /// When the server is initiated, the server sends this to the client and the client responds in turn (of course, changing the MessageDirection).
    ServerInitiated(Client),
    /// This is sent from the server to the client in order to uniquely identify the client... Will need to store this in a database
    ClientInfo(MessageDirection),
    /// The Message Direction contains the sender and intended receiver
    Message(String, MessageDirection),
    /// This will show the client the available users on any particular round
    OnlineClients(HashSet<Client>, RoundNumber),
    /// This indicates that this client is ready to be paired at whatever future round, the server will respond with a Self::OnlineClients variant
    ReadyForPartner(Client),
    /// This is used for ending the websocket connection between the client and the server. The message direction indicates who has initiated the closure.
    ClosedConnection(Client),
    /// The string contains the content of the sdp message
    SdpRequest(String, MessageDirection),
    /// The receiver in the message direction is the client that initially sent the SDP Request 
    SdpResponse(String, MessageDirection)
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