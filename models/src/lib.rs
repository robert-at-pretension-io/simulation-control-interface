use bincode;
use serde::{Serialize, Deserialize};
use tungstenite::Message;

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

    pub fn to_binary_message(&self) -> Message {
        // let message = tungstenite::Message::binary(
        //     ControlMessages::serialize(&ControlMessages::Message(String::from("Hello from the server"))));
            
        tungstenite::Message::binary(self.serialize())
        
    }
}