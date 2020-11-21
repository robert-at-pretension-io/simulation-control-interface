use bincode;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize,Deserialize, Eq)]
pub struct Client {
    pub username: String,
    pub user_id: String
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
    ServerInitiated(MessageDirection),
    /// This is sent from the server to the client in order to uniquely identify the client... Will need to store this in a database
    Client(Client),
    // The Message Direction contains the client of interest
    Message(String, MessageDirection),
    OnlineClients(Vec<Client>),
}

impl ControlMessages {
    pub fn serialize(&self) -> Vec<u8>{
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(bytes : &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }


}