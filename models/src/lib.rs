use bincode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::{time::{Duration, Instant}, u128};

// use anyhow::Error;


/*
HtmlMediaElement, MediaDevices, MediaStream, MediaStreamConstraints, MediaStreamTrack,
    MessageEvent, RtcConfiguration, RtcIceCandidate, RtcIceCandidateInit, RtcIceConnectionState,
    RtcOfferOptions, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcRtpReceiver,
    RtcRtpTransceiver, RtcRtpTransceiverDirection, RtcSdpType, RtcSessionDescriptionInit,
    RtcTrackEvent,
*/
use web_sys::{
     WebSocket as WS,
};


use std::net::SocketAddr;
use std::{
    collections::{HashMap},
    hash::Hash,
};

use async_trait::async_trait;

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

#[derive(Debug, Serialize, Deserialize, Hash, Clone, Eq, PartialEq)]
pub enum Role {
    Admin,
    Moderator,
    User,
    Server
}





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


// #[derive(Debug)]
pub struct Environment {
    /// This is used for identification within the message routing system
    pub identity: Entity,
    /// These will define what behavior is allowed
    pub roles: Vec<Role>,
    /// The environment will orchestrate the running of the processes depending on if they are blocking. There must be at least one infinitely looping process in the processes Vec, otherwise the environment is not long-lived... That would be silly.
    pub processes: Vec<Process>,
    /// If the version is out of date with the network of clients or the server, a process of updating will occur
    pub version: u32,
    /// This is the first function that is run before any others. It will be used to initialize the environment. Most commonly this could include tasks such as opening channels that will be open through the lifetime of the environment
    pub initialization: Process,
    /// Each environment will contain an abstraction of a communication manager. This struct will negotiate messages sent between the Entity and all other Entities. 
    pub communication_manager: CommunicationManager
}


impl Environment {
    fn initialize() -> Environment {
        // First we need to initialize the communication channels involved in this environment. But only the signaling server. The other types of channels will be spun up on demand.

        // let identity = (get this value from the server)
    }

}


pub struct CommunicationManager {
    pub signaling_channel : (Entity, Box<dyn CommunicationChannel>),
    pub open_channels: Vec<(Box<dyn CommunicationChannel>, Entity, Entity)>,
    pub all_entities: Vec<Entity>,
}

pub enum ChannelTypes {
    Websocket,
    WebRtcData,
    WebRtcVideo,
}

#[async_trait]
pub trait CommunicationChannel {
    /// The second element in the truple will be the identity of the client who sent the initialize process
    /// The third item represents the role that the client is currently allowed to take on
    async fn initialize  (&self, process : Process) -> (&Self, Entity, Vec<Role>) where Self : Sized; 
    async fn send(&self, sender: Entity, receiver : Entity, message: ContextualizedCommand) where Self : Sized;
    async fn receive(&self, sender: EntityDetails, message: ContextualizedCommand) where Self : Sized;
}

struct WebSocket {

}

#[async_trait]
impl CommunicationChannel for WebSocket {
    async fn initialize  (&self, process : Process) -> (&Self, Entity, Vec<Role>) where Self : Sized {
        todo!()
    }

    async fn send(&self, sender: Entity, receiver : Entity, message: ContextualizedCommand) where Self : Sized {
        todo!()
    }

    async fn receive(&self, sender: EntityDetails, message: ContextualizedCommand) where Self : Sized {
        todo!()
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Entity {
    pub entity_type: EntityTypes,
    pub entity_detail: EntityDetails,
}

impl Entity {
    pub fn new(entity_detail: EntityDetails) -> Entity {
        let entity_type = match entity_detail {
            EntityDetails::Client(_uuid, _address) => EntityTypes::Client,
            EntityDetails::Server(_uuid, _address) => EntityTypes::Server,
        };

        Entity {
            entity_type,
            entity_detail,
        }
    }
    pub fn get_uuid(&self) -> Option<uuid::Uuid> {
        match self.entity_detail {
            EntityDetails::Client(uuid, _address) => Some(uuid),
            EntityDetails::Server(uuid, address) => Some(uuid),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum EntityDetails {
    Client(uuid::Uuid, SocketAddr),
    Server(uuid::Uuid, SocketAddr),
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
    /// Ice Candidate used for supporting a webrtc connection channel
    IceCandidate(String),
    /// Websocket Ping
    Ping(Uuid, u64),
    /// Websocket Pong
    Pong(Uuid, u64),
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextualizedCommand {
    /// When a command is sent out, this identifier will be used by the environment to track responses.
    pub uuid : Uuid,
    pub sender: Entity,
    pub intermediary: Option<Entity>,
    pub receiver: Entity,
    /// This will be a trait object 
    pub command: Command,
    pub response: Option<Command>
    
}

impl ContextualizedCommand {
    pub fn new(
        uuid: Uuid,
        sender: EntityDetails,
        receiver: EntityDetails,
        intermediary: Option<EntityDetails>,
        command: Command,
        response: Option<Command>
    ) -> ContextualizedCommand {
        ContextualizedCommand {
            uuid,
            sender: Entity::new(sender),
            receiver: Entity::new(receiver),
            intermediary: intermediary.map_or(None, |ent| Some(Entity::new(ent))),
            command,
            response
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
            uuid: self.uuid.clone(),
            response: self.response.clone(),

        }
    }
}
// #[async_trait]
// trait Runtime {
//     type ReturnType;
//     type ErrorType;  
    
//     fn setup() -> Box<Self>;
//     async fn run(&self)  -> Result<Option<Self::ReturnType>, Option<Self::ErrorType>> where Self : Send;
//     async fn main() -> Result<(u128, Option<Self::ReturnType>), (u128, Option<Self::ErrorType>)> where Self : Send{
//         let start = Instant::now();
//         let s = Self::setup();
//         let result = s.run().await;
//         match result {
//             Ok(val) => {
//                 let time = start.elapsed().as_micros();
//                 Ok((time, val))
//             }
//             Err(err) => {
//                 let time = start.elapsed().as_micros();
//                 Err((time,err))
//             }
//         }
        
//     }
// }




#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Process {
    /// This will be stored within the environment so that multiple asynchronous processes can occur without message collision.
    pub uuid: Uuid,
    /// This will be visible within the user/admin interface to identify which process is occurring. 
    pub name: String,
    /// The order the commands are put into the vector is the order in which they will be executed.
    pub ordered_commands: Vec<ContextualizedCommand>,
    /// This field explains to any programer/informed user what the purpose of the process is
    pub explanation: String,
    /// If this process blocks then the next process will not be able to start execution until this process finishes
    pub blocking: bool,
    /// This will determine if a process should repeat from the beginning after its completion (for instance if the behavior within the process is the main functionality of the system!)
    pub looping: bool,

    /// The environment will use this to negotiate the lifetime of the process
    pub keep_alive : bool

}

impl Process {

}
