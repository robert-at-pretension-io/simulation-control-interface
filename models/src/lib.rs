use bincode;
use serde::{Serialize, Deserialize};
//use tungstenite::Message;

#[derive(Debug, Serialize,Deserialize)]
pub enum ControlMessages {
    ServerInitiated,
    Id(String),
    Message(String)
}

impl ControlMessages {
    pub fn serialize(&self) -> Vec<u8>{
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(bytes : &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }

    // I don't think the frontend can use this? Yep.. Supposedly tungstenite relies on the openssl crate :X
    // pub fn to_binary_message(&self) -> Message {
    //     // let message = tungstenite::Message::binary(
    //     //     ControlMessages::serialize(&ControlMessages::Message(String::from("Hello from the server"))));
            
    //     tungstenite::Message::binary(self.serialize())
        
    // }
}