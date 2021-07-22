

use petgraph::EdgeType;
use serde::{Deserialize, Serialize};
use erased_serde::Serialize as ErasedSerialize;
use uuid::Uuid;


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

use petgraph::graphmap::{GraphMap, UnGraphMap};


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
}

#[derive(Debug, Serialize, Deserialize, Hash, Clone, Eq, PartialEq)]
pub enum Status {
    InCall(Uuid, Uuid),
    WaitingForPartner,
    AnsweringQuestionAboutLastPartner,
}

// #[derive(Debug, Serialize, Deserialize, Hash, Clone, Eq, PartialEq)]
// pub enum Role {
//     Admin,
//     Moderator,
//     User,
//     Server
// }

impl Client {
    pub fn update(&mut self, client: Client) {
        self.username = client.username;
        self.user_id = client.user_id;
        self.email = client.email;
    }
    /// This function is mainly used for comparison...
    pub fn from_user_id(user_id: uuid::Uuid) -> Client {
        Client {
            username: None,
            user_id,
            email: None,
        }
    }

    pub fn replace_with_newer_values(&mut self, new: Client) -> Result<(), String> {
        if self.user_id == new.user_id {
            self.username = new.username;
            self.email = new.email;
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

type NetworkTopology = GraphMap<Entity, (Entity,Entity), Undirected>;

#[async_trait]
pub trait Environment : InternalSystemComponents{
    async fn initialize(&mut self, communication_manager : impl CommunicationManager,  environment_processes : Vec<impl Process>) -> Result<(NetworkTopology, Entity), EnvironmentErrors>;
    async fn run(&mut self);
    fn version(&self) -> u32;
    fn identity(&self) -> Entity; 

}

pub struct Undirected  {
}

impl EdgeType for Undirected {
    fn is_directed() -> bool{
        false
    }
}

pub enum EnvironmentErrors {
    FailureToInitialize,
}


#[async_trait]
pub trait CommunicationManager : InternalSystemComponents {
    /// The return Entity refers to the identity of the environment. In this system, the identity refers to the environment instead of the user. This is mainly because Entities are used to connect the network in a certain topology...
async fn new(&mut self, allowed_communication_types : Vec<(EntityTypes,EntityTypes)>, signaling_server : (Entity, impl CommunicationChannel), signal_channel_initialization : impl Process) -> (Self, Entity) where Self : Sized;
async fn add_channel(&mut self, channel: impl CommunicationChannel, participant :Entity) -> Result<(), CommunicationErrors>;
async fn open_channels(&self) -> &Vec<(&dyn CommunicationChannel, Entity)>;
async fn register_process_message(&self, waiting_process : impl Process, wait_for_message : impl Message);
async fn pop_queue(&mut self) -> dyn Message;
async fn add_to_queue(&mut self, message: dyn Message);  


}

#[async_trait]
pub trait InternalSystemComponents {
    async fn send_receive_messages(&self, message : InternalMessage);
    fn get_uuid(&self) -> Uuid;
}

/// The process manager hooks up the process runtime with the network topology. It keeps track of receving internal control messages from the communication manager.
#[async_trait]
pub trait ProcessManager : InternalSystemComponents{
    async fn initialize(identity : Entity) -> Self where Self : Sized;
    fn register_functionality(process : impl Process);
}

pub struct StateManager {
    state: HashMap<String, Box<dyn ErasedSerialize>>
}


pub enum CommunicationErrors {
    DisallowedEntityType,
    ChannelInitializationFailed,
} 

pub trait Message {
    fn identity(&self) -> Uuid;
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn data(&self, send_data : Option<Box<dyn ErasedSerialize>>) -> Option<Vec<u8>>;
}
/// This enum will be used for communicating between the major independent components of the system. As of writing, these include: 1) The process manager, 2) The state manager, 3) The Network manager. All communication between entities will be specified by sending processes between network entities. The process manager and communcation(network) manager will then orchestrate amongst themselves how processes are carried out. In order for the system to be very flexible and extensible, the state manager and process manager are going to be defined in terms of traits (abstract interfaces). Also, the data-structures used for distributed state management is based on p2p consensus, allowing for the applications to be more akin to configurations of a dynamic/powerful system built with the rust language. The internal messages are separated from process messages for another good reason: Any internal message interpreted within an environment or system can easily considered to have fully-authorized permission and allows for rapid velocity of development.
pub enum InternalMessage{
    StartProcess(Uuid),
    PolluteProcess(Uuid),
    AdvanceProcess(Uuid),
    SendProcess(Uuid),
    CloseChannel(Uuid),
    OpenChannel(Uuid)
}

#[async_trait]
pub trait CommunicationChannel :  ErasedSerialize{
    /// The first element in the returned tuple will be the identity of the client who sent the initialize process
    /// The second item represents the role that the client is currently allowed to take on
    async fn initialize  (&self, setup_channel : impl Process, participant: Entity, keep_alive : PingTime, channel_type : ChannelType) -> Result<(Entity, Self), CommunicationErrors> where Self : Sized; 
    async fn send(&self, sender: Entity, receiver : Entity, message: impl Message) where Self : Sized;
    fn channel(&self) -> (tokio::sync::mpsc::Sender<Box<dyn Message>>,tokio::sync::mpsc::Receiver<Box<dyn Message>>); 
    async fn receive(&self, sender: EntityDetails, message: impl Message) where Self : Sized;
    fn identity(&self) -> Uuid;
}

pub enum ChannelType {
     Signaling,
     Communication
}




//     /// The environment will use this to negotiate the lifetime of the process
//     pub keep_alive : PingTime

struct WebSocket {
    websocket : WS,
    channel: (tokio::sync::mpsc::Sender<Box<dyn Message>>,tokio::sync::mpsc::Receiver<Box<dyn Message>>),


}

// #[async_trait]
// impl CommunicationChannel for WebSocket {
//     async fn initialize  (&self, process : Process) -> (&Self, Entity, Vec<Role>) where Self : Sized {
//         todo!()
//     }

//     async fn send(&self, sender: Entity, receiver : Entity, message: ContextualizedCommand) where Self : Sized {
//         todo!()
//     }

//     fn channel(&self) -> (tokio::sync::mpsc::Sender<ContextualizedCommand>,tokio::sync::mpsc::Receiver<ContextualizedCommand>) {
//         todo!()
//     }
// }


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

}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum EntityDetails {
    Client(uuid::Uuid, Option<SocketAddr>),
    Server(uuid::Uuid, SocketAddr),
}



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
    OnlineClients(HashMap<Uuid, Client>, u32),
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
pub enum Entities {
    One(EntityTypes),
    Exactly(u32, EntityTypes),
    UpTo(u32, EntityTypes),
    AtLeast(u32, EntityTypes)
}




#[async_trait]
pub trait Process : InternalSystemComponents{
    fn new(involved_parties : Vec<Entities>,
        ordered_messages : Vec<(EntityTypes, Box<dyn Message> )> ,name: String, explanation: String, blocking : bool, looping: bool) -> Self where Self: Sized;

    async fn log_step(&mut self, step : impl Message, status: ProcessStatus, posted_by : Entity);

    fn waiting_for_message_type(&self) -> dyn Message;
    
    async fn receive_message(&self, message : dyn Message);
    /// Sends the message and pushes the 'focus token' onto the next message in the  ```rust ordered_message_pairs ```
    async fn send_message(&mut self, message: dyn Message);
    async fn start(&mut self);
    async fn start_timed(&mut self);
}



pub enum ProcessStatus {
    Received,
    Sent,
    Waiting,
    Running
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PingTime {
    /// The process doesn't need to ping the participants
    Never,
    /// The process will ping the participants every u32 seconds
    Every(u32),
    /// ping the participants in the process randomly between every u32 and u32 seconds.
    RandomBetween(u32, u32)
}

