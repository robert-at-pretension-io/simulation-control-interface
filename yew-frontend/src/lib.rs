#![recursion_limit = "512"]

use uuid::Uuid;
use wasm_bindgen::prelude::*;

//Each of the javascript api features must be added in both the YAML file and used here
use web_sys::{MessageEvent, WebSocket};

// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes.
use yew::prelude::*;

// This local trait is for shared objects between the frontend and the backend
use models::{Client, ControlMessages, MessageDirection, InformationFlow};

use std::net::SocketAddr;


use std::collections::HashSet;



// all of these are for webrtc
use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
     RtcDataChannelEvent, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit,
};


#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum State {
    UnsetUsername,
    ConnectedToWebsocketServer,
    ConnectedToRtcPeer,
}

static WEBSOCKET_URL: &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
connection_socket_address : Option<SocketAddr>,
    user_id : Option<uuid::Uuid>,
    username : Option<String>,
    partner: Option<Client>,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
    peers: HashSet<Client>,
    states: HashSet<State>,
}

impl Model {
    fn reset_state(&mut self) {
            self.username = None;
            self.user_id = None;
            self.connection_socket_address = None;
            self.partner =  None;
            self.websocket = None;
            self.peers = HashSet::new();
            self.states = HashSet::new();

    }
}
#[derive(Debug,Clone)]
enum Msg {
    UpdateUsername(String),
    SetClient(Client),
    ClearLog,
    ReceivedIceCandidate(String, MessageDirection),
    SendIceCandidate(String, MessageDirection),
    SdpRequest(String, MessageDirection),
    /// The string will indicate the client user_id field
    MakeSdpRequestToClient(Uuid),
    SdpResponse(String, MessageDirection),
    ResetPage,
    InitiateWebsocketConnectionProcess,
    LogEvent(String),
    ServerSentWsMessage(String),
    UpdateOnlineUsers(HashSet<Client>),
    AddState(State),
    RemoveState(State),
    CloseWebsocketConnection,
    RequestUsersOnline(Client),
    SendWsMessage(ControlMessages),
}

extern crate web_sys;

impl Model {

    fn client_to_model(&mut self, client : Client) {
self.connection_socket_address = client.current_socket_addr;
self.username = client.username;
self.user_id = Some(client.user_id);
    }

    fn show_welcome_message(&self) -> Html {

        {if self.username.is_none() {
            
                {if self.states.contains(&State::ConnectedToWebsocketServer) {

                    html!(
                    <>

                    <button onclick=self.link.callback( |_| { Msg::UpdateUsername(format!("Alice"))

                }   )> {"Set username Alice"} </button>

                <button onclick=self.link.callback( |_| { Msg::UpdateUsername(format!("Bob"))

                }   )> {"Set username Bob"} </button>
                    </>
                    
                    )
              

            } else { html!(<></>)}
        }
        
        }
        else{ //Username IS set!
            html!(
                <div>
                //, self.client.clone().unwrap().username.unwrap()
                {format!("Welcome {}", self.username.as_ref().unwrap())}

                </div>

            )
        }
        
        }
    }

    fn send_ws_message(&mut self, data: ControlMessages) {
        let ws = self.websocket.take().unwrap();

        let message = data.clone();

        let data = &data.serialize();

        

        match ws.send_with_u8_array(data) {
            Ok(success) => {
                self.link.send_message(Msg::LogEvent(format!("Successfully sent the ws message: {:?}", message)))
            },
            Err(err) => {
                self.link.send_message(Msg::LogEvent(format!("There was an error sending the ws message: {:?}", err)))
            }
        }

        self.websocket = Some(ws);
    }
    fn show_events_in_table(&self) -> Html {
        html!(
            <ul>
            {for self.event_log.iter().map(|event| {
                html!(<li> {event} </li>)
            })  }
            </ul>
        )
    }

    //still exhausted...
    fn show_peers_online(&self, client: &Client) -> Html {
        
            let client_clone = client.clone();
            let client_clone2 = client.clone();

                html!(<li> <button onclick=self.link.callback( move |_| {
                    Msg::MakeSdpRequestToClient(client_clone.user_id.clone())
                } ) > {format!("{:?} : {:?}", client_clone2.username.clone() , client_clone2.current_socket_addr.clone())} </button> </li>)
              
        
    }

    fn setup_websocket_object_callbacks(&mut self, ws: WebSocket) -> WebSocket {
        let cloned = self.link.clone();

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // The only type of message that will be officially recognized is the almighty ArrayBuffer Binary Data!
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);

                match ControlMessages::deserialize(&array.to_vec()) {
                    Ok(result) => match result {
                        ControlMessages::ClientInfo(message_direction) => {

                            match message_direction {
                                MessageDirection::ClientToClient(_) | MessageDirection::ClientToServer(_) => {
                                    // this shouldn't happen?
                                    cloned.send_message(Msg::LogEvent(format!("This shouldn't happen :/")));
                                }
                                MessageDirection::ServerToClient(client) => {
                                    cloned.send_message(Msg::SetClient(client));
                                }
                            }

                            
                        }
                        ControlMessages::SdpResponse(response, message_direction) => {
                            cloned.send_message(Msg::SdpResponse(response, message_direction));
                        }
                        ControlMessages::SdpRequest(request, message_direction) => {
                            cloned.send_message(Msg::SdpRequest(request, message_direction));
                        
                        }

                        ControlMessages::Message(message, _directionality) => {
                            cloned.send_message(Msg::ServerSentWsMessage(message));
                        }

                        ControlMessages::ServerInitiated(info) => cloned.send_message_batch(vec!(Msg::LogEvent(format!("{:?} Connected To Websocket Server!", info)),
                        Msg::SetClient(info.clone()),
                        Msg::AddState(State::ConnectedToWebsocketServer),
                        Msg::RequestUsersOnline(info)
                    )
                            
                        ),

                        ControlMessages::OnlineClients(clients, round_number) => {
                            cloned.send_message(Msg::UpdateOnlineUsers(clients))
                        }
                        ControlMessages::ReadyForPartner(_client) => {}
                        ControlMessages::ClosedConnection(_) => {
                            cloned.send_message(Msg::ResetPage)
                        }
                    },
                    Err(oh_no) => {
                        cloned.send_message(Msg::ServerSentWsMessage(oh_no.to_string()));
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        ws
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            link,
            websocket: None,
            event_log: Vec::<String>::new(),
            connection_socket_address : None,
            user_id : None,
            username : None,
            partner: None,
            peers: HashSet::<Client>::new(),
            states: HashSet::<State>::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ClearLog => {
                self.event_log = Vec::<String>::new();
                true
            }
            Msg::SdpRequest(sdp, message_direction) => {
                let message_direction_clone = message_direction.clone();
                let client = Client{username: None, user_id: self.user_id.clone().unwrap(), email: None, current_socket_addr: None};
                match message_direction {
                    MessageDirection::ClientToClient(flow) => {
                        
                        if &flow.sender == &client {
                            self.link.send_message(Msg::SendWsMessage(ControlMessages::SdpRequest(sdp, message_direction_clone)))
                        }
                        if &flow.receiver == &client{

                        }
                    } 
                    MessageDirection::ClientToServer(_) | MessageDirection::ServerToClient(_) => {
                        self.link.send_message(Msg::LogEvent(format!("SdpRequest is messing up...")));
                    }
                }
                true
            }
            Msg::ResetPage => {
                self.reset_state();
                true
            },
            Msg::RequestUsersOnline(client)=> {
                self.send_ws_message(ControlMessages::ReadyForPartner(client.clone()));
                
                true
            },
            Msg::SendWsMessage(control_message) => {
                self.link.send_message(Msg::LogEvent(format!(
                    "Sending Message to server: {:?}",
                    &control_message
                )));

                let cloned_message = control_message.clone();

                match cloned_message {
                    ControlMessages::ServerInitiated(_) => {
                        self.link.send_message(Msg::LogEvent(format!(
                            "ServerInitiated isn't implemented on the client side"
                        )))
                    }
                    ControlMessages::ClientInfo(message_direction) => {
                        
                        match message_direction {
                            MessageDirection::ServerToClient(client) => {
                                self.link.send_message(Msg::UpdateUsername(client.username.unwrap()))
                            }
                            MessageDirection::ClientToServer(_) | MessageDirection::ClientToClient(_) => {
                                self.link.send_message(Msg::LogEvent(
                                    format!("ClientInfo isn't implemented on the client side."),
                                ))
                            }
                        }
                        
                
                }
                    
                    ,
                    ControlMessages::Message(_, message_direction) => match message_direction
                    {
                        MessageDirection::ClientToClient(flow) => {
                            self.send_ws_message(control_message);
                        }
                        MessageDirection::ServerToClient(_) => {}
                        MessageDirection::ClientToServer(_t) => {
                            self.send_ws_message(control_message);
                        }
                    },
                    ControlMessages::OnlineClients(_, _) => self.link.send_message(Msg::LogEvent(
                        format!("OnlineClients isn't implemented on the client side."),
                    )),
                    ControlMessages::ReadyForPartner(client) => {
                        self.send_ws_message(ControlMessages::ReadyForPartner(client))
                    }
                    ControlMessages::ClosedConnection(_) => {}
                    ControlMessages::SdpRequest(_, _) => {}
                    ControlMessages::SdpResponse(_, _) => {}
                }

                true
            }
            Msg::CloseWebsocketConnection => {
                let ws = self.websocket.take();

                match ws {
                    Some(ws) => {
                        ws.close().unwrap();
                        self.link
                            .send_message(Msg::RemoveState(State::ConnectedToWebsocketServer));
                        true
                    }
                    None => self.states.remove(&State::ConnectedToWebsocketServer),
                }
            }
            Msg::AddState(state) => match self.states.insert(state) {
                true => true,
                false => false,
            },
            Msg::RemoveState(state) => match self.states.remove(&state) {
                true => {
                    self.link.send_message(Msg::LogEvent(format!(
                        "Removed the following state: {:?}",
                        state
                    )));
                    true
                }
                false => {
                    self.link.send_message(Msg::LogEvent(format!("Tried removing the following state: {:?}. But it wasn't in the current set of states.", state)));
                    false
                }
            },
            Msg::LogEvent(event) => {
                self.event_log.push(event);
                true
            }
            Msg::ServerSentWsMessage(message) => {
                self.link.send_message(Msg::LogEvent(message));
                true
            }
            Msg::InitiateWebsocketConnectionProcess => match WebSocket::new(WEBSOCKET_URL) {
                Ok(ws) => {
                    let ws = self.setup_websocket_object_callbacks(ws);

                    self.websocket = Some(ws);
                    let messages: Vec<Msg> = vec![
                        Msg::LogEvent("attempting ws connection ...".to_string()),
                        
                        
                    ];
                    self.link.send_message_batch(messages);
                    true
                }
                Err(err) => {
                    self.link
                        .send_message(Msg::LogEvent(format!("error: {:?}", err)));
                    true
                }
            },
            Msg::SetClient(client) => {
                self.client_to_model(client);

                true
            }
            Msg::UpdateUsername(username) => {
                self.username = Some(username.clone());
                let msg = Msg::SendWsMessage(ControlMessages::ClientInfo(
                    MessageDirection::ClientToServer(Client{
                        email: None, user_id : self.user_id.clone().unwrap(), username : Some(username), current_socket_addr : None
                    })

                ));
                self.link.send_message(msg);
                true
            }
            Msg::UpdateOnlineUsers(clients) => {
                let mut clients = clients.clone();
                
                match self.user_id {
                    Some(this_user) => {
                        clients.remove(&Client::from_user_id(this_user));
                    }
                    None => {
                        // how?!
                    }
                }
                self.peers = clients;

                true
            }
            Msg::ReceivedIceCandidate(_, _) => {false}
            Msg::SendIceCandidate(_, _) => {false}
            Msg::MakeSdpRequestToClient(user_id ) => {
                let sender = Client::from_user_id(user_id);
                let receiver = Client{user_id, username: None, email: None, current_socket_addr: None};
                let messages : Vec<Msg> = vec![Msg::LogEvent(format!("Need to make Sdp Request for {:?}", &receiver)),
                Msg::SdpRequest(format!("sdpRequest"), MessageDirection::ClientToClient(InformationFlow{sender, receiver}))
                ];

                self.link.send_message_batch(messages);
                true
            }
            Msg::SdpResponse(_, _) => {false}
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        
        html! {
            <div>


                {
                    
                    self.show_welcome_message()}
                
                
                {if (self.event_log.len() > 5 ){ html!(<button onclick=self.link.callback(|_| {Msg::ClearLog})> {"Clear the event log."} </button> )} else {html!(<></>)}  }
                <div>

                {if (self.event_log.len() > 1 ){ html!(<p> {format!("The following details the event log of the application:")} </p> )} else {html!(<></>)}  }

                
                {self.show_events_in_table() }
                </div>

                // <button onclick=self.link.callback(|_| {
                //     Msg::InitiateWebsocketConnectionProcess
                // })>
                //     {"Click here to connect to the server."}
                // </button>

                {
                    if (!self.states.contains(&State::ConnectedToWebsocketServer)){
                    html!(<button onclick=self.link.callback(|_| {
                        Msg::InitiateWebsocketConnectionProcess
                    })>
                        {"Click here to connect to the server."}
                    </button>)}
                    else {
                        html!(<div>

                            {if !self.peers.is_empty() {
                                html!(
                            <div>
                            <h1> {"Peers online:"} </h1>
                            {
                                for self.peers.iter().map(|client| {
                                self.show_peers_online(client)
                                })
                            
                            }
                            </div>
                                )
                            }  else {html!(<p> {"No peers online this round."} </p>)} }

                            <button onclick=self.link.callback(|_| {
                                Msg::CloseWebsocketConnection
                            })>
                                {"Disconnect"}
                            </button>

                            </div>)
                    }


            }


            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
