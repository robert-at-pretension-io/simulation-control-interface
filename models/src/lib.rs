use bincode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use chrono;
use std::net::SocketAddr;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    hash::Hasher,
};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct Client {
    pub username: Option<String>,
    pub email: Option<String>,
    pub user_id: uuid::Uuid,
    // This will only be set to None if the websocket connection is not yet initialized... Not sure this ever actually happens?
    pub current_socket_addr: Option<SocketAddr>,
    pub status: Option<Status>,
    pub ping_status: PingStatus,
}

#[derive(Debug, Serialize, Deserialize, Hash, Clone, Eq, PartialEq)]
pub enum Status {
    InCall(Uuid, Uuid),
    WaitingForPartner,
    AnsweringQuestionAboutLastPartner,
}

// impl PartialEq for Status {
//     fn eq(&self, other: &Self) -> bool {
//         match self {
//             Status::InCall(_a, _b) => match other {
//                 Status::InCall(_c, _d) => true,
//                 Status::WaitingForPartner => false,
//                 Status::AnsweringQuestionAboutLastPartner => false,
//             },
//             Status::WaitingForPartner => match other {
//                 Status::InCall(_c, _d) => false,
//                 Status::WaitingForPartner => true,
//                 Status::AnsweringQuestionAboutLastPartner => false,
//             },
//             Status::AnsweringQuestionAboutLastPartner => match other {
//                 Status::InCall(_c, _d) => false,
//                 Status::WaitingForPartner => false,
//                 Status::AnsweringQuestionAboutLastPartner => true,
//             },
//         }
//     }
// }

impl Client {
    pub fn update(&mut self, client: Client) {
        self.username = client.username;
        self.ping_status = client.ping_status;
        self.status = client.status;
        self.user_id = client.user_id;
        self.current_socket_addr = client.current_socket_addr;
        self.email = client.email;
    }
    /// This function is mainly used for comparison...
    pub fn from_user_id(user_id: uuid::Uuid) -> Client {
        Client {
            username: None,
            user_id,
            email: None,
            current_socket_addr: None,
            status: None,
            ping_status: PingStatus::NeverPinged,
        }
    }

    pub fn replace_with_newer_values(&mut self, new: Client) -> Result<(), String> {
        if self.user_id == new.user_id {
            self.username = new.username;
            self.email = new.email;
            self.current_socket_addr = new.current_socket_addr;
            Ok(())
        } else {
            Err(format!("The new client did not equal the old one!"))
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Eq, Hash, Clone)]
///This enum will be used for keeping the connections alive and informing the clients of the round number
pub enum PingStatus {
    /// This is when the client has last been communicated with, the u64 value refers to the round number
    Pinged(u64),
    /// This is the state that all new clients to the system will be put into
    NeverPinged,
    /// This will be the response that a client gives
    Ponged(u64),
}

impl PartialEq for PingStatus {
    fn eq(&self, other: &PingStatus) -> bool {
        match self {
            PingStatus::Pinged(a) => match other {
                PingStatus::Pinged(c) => {
                    if a == c {
                        true
                    } else {
                        false
                    }
                }
                PingStatus::NeverPinged => false,
                PingStatus::Ponged(_d) => false,
            },
            PingStatus::NeverPinged => match other {
                PingStatus::Pinged(_c) => false,
                PingStatus::NeverPinged => true,
                PingStatus::Ponged(_d) => false,
            },
            PingStatus::Ponged(a) => match other {
                PingStatus::Pinged(_c) => false,
                PingStatus::NeverPinged => false,
                PingStatus::Ponged(d) => {
                    if a == d {
                        true
                    } else {
                        false
                    }
                }
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum EntityTypes {
    Client,
    Server,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct Entity {
    pub entity_type: EntityTypes,
    pub entity_detail: EntityDetails,
}

impl Entity {
    pub fn new(entity_detail: EntityDetails) -> Entity {
        let entity_type = match entity_detail {
            EntityDetails::Client(_uuid) => EntityTypes::Client,
            EntityDetails::Server => EntityTypes::Server,
        };

        Entity {
            entity_type,
            entity_detail,
        }
    }
    pub fn get_uuid(&self) -> Option<uuid::Uuid> {
        match self.entity_detail {
            EntityDetails::Client(uuid) => Some(uuid),
            EntityDetails::Server => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum EntityDetails {
    Client(uuid::Uuid),
    Server,
}

type RoundNumber = u64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    /// This will send out the most up-to-date state of the online clients
    BroadcastUpdate,
    /// The first uuid is the initiator of the call, the second uuid is the receiver
    InCall(Uuid, Uuid),
    /// This command will be send from either client indicating to the system the desire for their call to be ended... The system will then mark them as available to chat with future partners
    EndCall(Uuid, Uuid),
    ///This will inform all clients of the current state of the system... In the future this will not need to be sent to all clients, instead it can be sent to a strongly-connected client which then propagates the updates to all other clients
    UpdateClient(Client),
    /// For the time being, the error will be a string. In the future, it will be a struct/enum containing all possible errors that could occur
    Error(String),
    // This indicates that this client is ready to be paired at whatever future round, the server will respond with a Self::OnlineClients variant
    // ReadyForPartner(Client),
    ///  When the server is initiated, the server sends this to the client and the client responds in turn (of course, changing the MessageDirection).
    ServerInitiated(Client),
    /// This will show the client the available users on any particular round
    OnlineClients(HashMap<Uuid, Client>, RoundNumber),
    /// Used to uniquely identify the client
    // InitiazeClient(Client),
    /// The string contains the content of the sdp message
    SdpRequest(String),
    /// The receiver in the message direction is the client that initially sent the SDP Request
    SdpResponse(String),
    /// This is used for ending the websocket connection between the client and the server. The message direction indicates who has initiated the closure.
    ClosedConnection(uuid::Uuid),
    /// This is needed to confirm that the server is ready to close the connection
    AckClosedConnection(uuid::Uuid),
    /// Ice Candidate used for supporting a webrtc connection channel
    IceCandidate(String),
    /// Websocket Ping
    Ping(Uuid, u64),
    /// Websocket Pong
    Pong(Uuid, u64),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Envelope {
    pub sender: Entity,
    pub intermediary: Option<Entity>,
    pub receiver: Entity,
    pub command: Command,
}

impl Envelope {
    pub fn new(
        sender: EntityDetails,
        receiver: EntityDetails,
        intermediary: Option<EntityDetails>,
        command: Command,
    ) -> Envelope {
        Envelope {
            sender: Entity::new(sender),
            receiver: Entity::new(receiver),
            intermediary: intermediary.map_or(None, |ent| Some(Entity::new(ent))),
            command,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        match bincode::serialize(self) {
            Ok(vec) => vec,
            Err(oh_no) => panic!(oh_no),
        }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(bytes)
    }

    pub fn switch_direction(&self) -> Self {
        Self {
            sender: self.receiver.clone(),
            intermediary: self.intermediary.clone(),
            receiver: self.sender.clone(),
            command: self.command.clone(),
        }
    }
}
