#![recursion_limit = "1024"]
use uuid::Uuid;
use wasm_bindgen::prelude::*;

//Each of the javascript api features must be added in both the YAML file and used here
use web_sys::{MessageEvent, WebSocket};

// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes.
use yew::prelude::*;

// This local trait is for shared objects between the frontend and the backend
use models::{Client, Command, Entity, EntityDetails, Envelope};

use std::net::SocketAddr;

use std::collections::HashSet;

// all of these are for webrtc
use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    RtcDataChannelEvent, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit,
};



#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum State {
    ConnectedToWebsocketServer,
    ConnectedToRtcPeer,
}

static WEBSOCKET_URL: &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
    connection_socket_address: Option<SocketAddr>,
    user_id: Option<uuid::Uuid>,
    username: Option<String>,
    partner: Option<Client>,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
    peers: HashSet<Client>,
    states: HashSet<State>,
    local_ice_candidate : Vec<String>,
    local_web_rtc_connection : Option<RtcPeerConnection>,
}

impl Model {
    fn reset_state(&mut self) {
        self.username = None;
        self.user_id = None;
        self.connection_socket_address = None;
        self.partner = None;
        self.websocket = None;
        self.peers = HashSet::new();
        self.states = HashSet::new();
    }
}
#[derive(Debug, Clone)]
enum Msg {
    SetupWebRtc(),
    RtcClientReady(RtcPeerConnection),
    UpdateUsername(String),
    SetClient(Client),
    ClearLog,
    ResetPage,
    
    ReceivedIceCandidate(String),
    SendIceCandidate(String),
    AddLocalIceCandidate(String),
    SetLocalWebRtcOffer(String),
    
    MakeSdpResponse(String, uuid::Uuid),
    SendSdpRequestToClient(Uuid, String),
    MakeSdpRequestToClient(Uuid),
    SdpResponse(String),


    InitiateWebsocketConnectionProcess,
    LogEvent(String),
    ServerSentWsMessage(String),
    UpdateOnlineUsers(HashSet<Client>),
    AddState(State),
    RemoveState(State),
    CloseWebsocketConnection,
    RequestUsersOnline(Client),
    SendWsMessage(Envelope),
}

extern crate web_sys;

impl Model {
    fn full_client_info_from_user_id(&self, user_id: uuid::Uuid) -> Result<Client, String> {
        let client = self
            .peers
            .get(&Client::from_user_id(user_id))
            .clone()
            .to_owned();
        match client {
            Some(client) => Ok(client.to_owned()),
            None => Err(format!("The client wasn't found in the online peer set.")),
        }
    }

    fn client_to_model(&mut self, client: Client) {
        self.connection_socket_address = client.current_socket_addr;
        self.username = client.username;
        self.user_id = Some(client.user_id);
    }

    fn show_welcome_message(&self) -> Html {
        {
            if self.username.is_none() {
                {
                    if self.states.contains(&State::ConnectedToWebsocketServer) {
                        html!(
                            <>

                            <button onclick=self.link.callback( |_| { Msg::UpdateUsername(format!("Alice"))

                        }   )> {"Set username Alice"} </button>

                        <button onclick=self.link.callback( |_| { Msg::UpdateUsername(format!("Bob"))

                        }   )> {"Set username Bob"} </button>
                            </>

                            )
                    } else {
                        html!(<></>)
                    }
                }
            } else {
                //Username IS set!
                html!(
                    <div>

                    {format!("Welcome {}", self.username.as_ref().unwrap())}

                    </div>
                )
            }
        }
    }

    fn send_ws_message(&mut self, data: Envelope) {
        let ws = self.websocket.take().unwrap();

        let message = data.clone();

        let data = &data.serialize();

        match ws.send_with_u8_array(data) {
            Ok(success) => self.link.send_message(Msg::LogEvent(format!(
                "Successfully sent the ws message: {:?}",
                message
            ))),
            Err(err) => self.link.send_message(Msg::LogEvent(format!(
                "There was an error sending the ws message: {:?}",
                err
            ))),
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

                match Envelope::deserialize(&array.to_vec()) {
                    Ok(result) => {
                        let sender = result.sender;
                        let receiver = result.receiver;
                        let intermediary = result.intermediary;
                        match result.command {
                            Command::Error(error) => {
                                cloned.send_message(Msg::LogEvent(
                                    format!(
                                        "Received the following error: {}",
                                        error,)
                                ));
                            }
                            Command::ServerInitiated(client) => {
                                let messages = vec![
                                    Msg::LogEvent(format!(
                                        "{:?} Connected To Websocket Server!",
                                        client
                                    )),
                                    Msg::SetClient(client.clone()),
                                    Msg::AddState(State::ConnectedToWebsocketServer),
                                    Msg::SetupWebRtc(),
                                    Msg::RequestUsersOnline(client),
                                ];
                                cloned.send_message_batch(messages);
                            }
                            Command::OnlineClients(clients, _round_number) => {
                                cloned.send_message(Msg::UpdateOnlineUsers(clients))
                            }
                            Command::ClientInfo(client) => {
                                cloned.send_message(Msg::SetClient(client));
                            }

                            Command::ReadyForPartner(client) => {
                                cloned.send_message(Msg::ServerSentWsMessage(format!(
                                    "Ready for partner acknowledged by server"
                                )));
                            }

                            Command::SdpRequest(request) => {
                                cloned.send_message(Msg::MakeSdpResponse(
                                    request,
                                    sender.get_uuid().unwrap(),
                                ));
                            }
                            Command::SdpResponse(response) => {
                                cloned.send_message(Msg::SdpResponse(response));
                            }
                            Command::ClosedConnection(client) => {
                                cloned.send_message(Msg::ResetPage)
                            }
                        }
                    }
                    Err(uhh) => {
                        cloned.send_message(Msg::LogEvent(format!(
                            "found an error while receiving a message from the ws server: {}",
                            uhh
                        )));
                    }
                };
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        // set message event handler on WebSocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        ws
    }


    fn create_local_rtc_peer(&mut self)  {
        let cloned_link = self.link.clone();

        let onicecandidate_callback =
        Closure::wrap(
            Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                Some(candidate) => {
                    cloned_link.send_message(Msg::LogEvent(format!("This client made an ice candidate: {:#?}", candidate.candidate())));
                    cloned_link.send_message(Msg::AddLocalIceCandidate(candidate.candidate()))
                    
                }
                None => {}
            }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,
        );

        let client = RtcPeerConnection::new().unwrap();
        client.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));

        onicecandidate_callback.forget();

        self.link.send_message(Msg::RtcClientReady(client));
    }



}


async fn create_webrtc_offer(link: ComponentLink<Model>, receiver : uuid::Uuid,  local : RtcPeerConnection) {
    let offer = JsFuture::from(local.create_offer()).await.unwrap();
    
unsafe {    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp")).unwrap()
        .as_string()
        .unwrap();
    
        link.send_message(Msg::SendSdpRequestToClient(receiver, offer_sdp));
    
    }
    
}

async fn set_local_webrtc_offer(offer_sdp: String, link: ComponentLink<Model>,  local : RtcPeerConnection) {

    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&offer_sdp);
    let sld_promise = local.set_local_description(&offer_obj);
    JsFuture::from(sld_promise).await.unwrap();

}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            local_web_rtc_connection: None,
            link,
            websocket: None,
            local_ice_candidate: Vec::<String>::new(),
            event_log: Vec::<String>::new(),
            connection_socket_address: None,
            user_id: None,
            username: None,
            partner: None,
            peers: HashSet::<Client>::new(),
            states: HashSet::<State>::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetupWebRtc() => {
                self.link.send_message(Msg::LogEvent(format!("Setting up webRTC locally...")));
                self.create_local_rtc_peer();
            true
            },
            Msg::AddLocalIceCandidate(candidate) => {
                self.local_ice_candidate.push(candidate.clone());
                self.link.send_message(Msg::LogEvent(format!("need to send ice candidate {} to partner...", candidate)));
                true
            }
            Msg::ClearLog => {
                self.event_log = Vec::<String>::new();
                true
            }
            Msg::RtcClientReady(client) => {
                self.local_web_rtc_connection = Some(client);
                self.link.send_message(Msg::LogEvent(format!("local WebRTC successfully set! Able to send sdp objects to clients now!")));
                false
            }
            // Msg::MakeSdpRequest(sdp, receiver) => {
                
            //     let envelope = Envelope::new(
            //         EntityDetails::Client(self.user_id.unwrap()),
            //         EntityDetails::Client(receiver),
            //         Some(EntityDetails::Server),
            //         Command::SdpRequest(sdp.clone()),
            //     );
            //     self.link.send_message(Msg::SendWsMessage(envelope));
            //     true
            // }
            Msg::ResetPage => {
                self.reset_state();
                true
            }
            Msg::RequestUsersOnline(client) => {
                let envelope = Envelope::new(
                    EntityDetails::Client(client.user_id.clone()),
                    EntityDetails::Server,
                    None,
                    Command::ReadyForPartner(client.clone()),
                );
                self.send_ws_message(envelope);

                true
            }
            Msg::SendWsMessage(control_message) => {
                self.link.send_message(Msg::LogEvent(format!(
                    "Sending Message to server: {:?}",
                    &control_message
                )));
                self.send_ws_message(control_message);

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
                    true
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
                    let messages: Vec<Msg> =
                        vec![Msg::LogEvent("attempting ws connection ...".to_string())];
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
                let user_id = self.user_id.clone().unwrap();

                let envelope = Envelope::new(
                    EntityDetails::Client(user_id.clone()),
                    EntityDetails::Server,
                    None,
                    Command::ClientInfo(Client {
                        email: None,
                        user_id: user_id.clone(),
                        username: Some(username),
                        current_socket_addr: None,
                    }),
                );

                self.send_ws_message(envelope);
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
            Msg::ReceivedIceCandidate(_) => false,
            Msg::SendIceCandidate(_) => false,
            Msg::SetLocalWebRtcOffer(offer) => {
                let local = self.local_web_rtc_connection.clone().unwrap();

                let link = self.link.clone();

                spawn_local(async move {set_local_webrtc_offer(offer, link,  local).await});
                
            }
            Msg::MakeSdpRequestToClient(receiver) => {


                let local = self.local_web_rtc_connection.clone().unwrap();

                let link = self.link.clone();


                spawn_local(async move {create_webrtc_offer(link, receiver.clone(), local).await});


                true
            }
            Msg::SendSdpRequestToClient(receiver, sdp) => {

                let sender = self.user_id.clone().unwrap();


                let envelope = Envelope::new(
                    EntityDetails::Client(sender),
                    EntityDetails::Client(receiver),
                    Some(EntityDetails::Server),
                    Command::SdpRequest(sdp),
                );

                self.send_ws_message(envelope);
                true
            }

            Msg::SdpResponse(_) => false,
            Msg::MakeSdpResponse(sdp, client) => {
                self.link.send_message(Msg::LogEvent(format!("Received the following sdp request: {:?} from client {:?}", sdp, client)));
                true},
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


                {
                    if (!self.states.contains(&State::ConnectedToWebsocketServer)){
                    html!(<button onclick=self.link.callback(|_| {
                        Msg::InitiateWebsocketConnectionProcess
                    })>
                        {"Click here to connect to the server."}
                    </button>)}
                    else {
                        html!(<div>

                            {if self.peers.len() > 1 {
                                html!(
                            <div>
                            <h1> {"Peers online:"} </h1>
                            {
                                for self.peers.iter().map(|client| {
                                    if client != &Client::from_user_id(self.user_id.unwrap()){
                                self.show_peers_online(client)} else {html!(<></>)}

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
