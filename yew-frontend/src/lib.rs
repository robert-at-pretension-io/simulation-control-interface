#![recursion_limit = "1024"]
use uuid::Uuid;
use wasm_bindgen::prelude::*;


// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes.
// ^ Thanks I needed that laugh..
use yew::prelude::*;

// This local trait is for shared objects between the frontend and the backend
use models::{Client, Command, EntityDetails, Envelope};

use std::net::SocketAddr;

use std::collections::HashSet;

// all of these are for webrtc
use js_sys::{Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{MediaDevices, MediaTrackSupportedConstraints, MessageEvent, Navigator, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, WebSocket, Window, MediaStreamConstraints, RtcConfiguration,RtcIceServer, RtcIceConnectionState, 
    RtcIceGatheringState,
    RtcSignalingState,
    MediaStream};


    use console_error_panic_hook;
use std::panic;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum State {
    ConnectedToWebsocketServer,
    ConnectedToRtcPeer,
}

static WEBSOCKET_URL: &str = "wss://liminalnook.com:2096";

struct Model {
    stream : Option<MediaStream>,
    event_log: Vec<String>,
    event_log_length: usize,
    connection_socket_address: Option<SocketAddr>,
    user_id: Option<uuid::Uuid>,
    username: Option<String>,
    partner: Option<Client>,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
    peers: HashSet<Client>,
    states: HashSet<State>,
    local_ice_candidate: Vec<String>,
    local_web_rtc_connection: Option<RtcPeerConnection>,
}

impl Model {
    fn reset_state(&mut self) {
        self.event_log_length = 5;
        self.stream = None;
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
    UpdateUsername(String),
    SetClient(Client),
    ClearLog,
    ResetPage,
    LogEvent(String),
    IncreaseLogSize,
    DecreaseLogSize,

    SetupWebRtc(),
    RtcClientReady(RtcPeerConnection),
    StoreMediaStream(MediaStream),
    ReceivedIceCandidate(String),
    SendIceCandidate(String),
    AddLocalIceCandidate(String),
    SetLocalWebRtcOffer(String),

    SetLocalMedia,

    MakeSdpResponse(String, uuid::Uuid),
    SendSdpRequestToClient(Uuid, String),
    MakeSdpRequestToClient(Uuid),
    SendSdpResponse(uuid::Uuid, String),
    ReceiveSdpResponse(uuid::Uuid, String),
    OverrideRtcPeer(RtcPeerConnection),
    ReportRtcDiagnostics(),

    InitiateWebsocketConnectionProcess,

    ServerSentWsMessage(String),
    UpdateOnlineUsers(HashSet<Client>),
    AddState(State),
    RemoveState(State),
    CloseWebsocketConnection,
    EndWebsocketConnection,
    RequestUsersOnline(Client),
    SendWsMessage(Envelope),
    
}

extern crate web_sys;

impl Model {


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
        let ws = self.websocket.take().expect("error getting the websocket :] in send_ws_message function");

        let message = data.clone();

        let data = &data.serialize();

        match ws.send_with_u8_array(data) {
            Ok(_) => self.link.send_message(Msg::LogEvent(format!(
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
            {for self.event_log.iter().rev().take(self.event_log_length).rev().map(|event| {
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
                        cloned.send_message(Msg::LogEvent(format!("Received the following command message: {:?}", result.command.clone())));
                        let sender = result.sender;
                        let receiver = result.receiver;
                        match result.command {
                            Command::Error(error) => {
                                cloned.send_message(Msg::LogEvent(format!(
                                    "Received the following error: {}",
                                    error,
                                )));
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
                                cloned.send_message(Msg::LogEvent(format!("Updating the clients online")));
                                cloned.send_message(Msg::UpdateOnlineUsers(clients))
                            }
                            Command::ClientInfo(client) => {
                                cloned.send_message(Msg::SetClient(client));
                            }

                            Command::ReadyForPartner(_) => {
                                cloned.send_message(Msg::ServerSentWsMessage(format!(
                                    "Ready for partner acknowledged by server"
                                )));
                            }

                            Command::SdpRequest(request) => {
                                cloned.send_message(Msg::MakeSdpResponse(
                                    request,
                                    sender.get_uuid().expect("error in Command::SdpRequest"),
                                ));
                            }
                            Command::SdpResponse(response) => {
                                cloned.send_message(Msg::ReceiveSdpResponse(
                                    receiver.get_uuid().expect("error in Command::SdpReponse"),
                                    response,
                                ));
                            }
                            Command::ClosedConnection(_) => {
                                cloned.send_message(Msg::ResetPage)
                            }
                            Command::AckClosedConnection(_) => {
                                cloned.send_message(Msg::EndWebsocketConnection)
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

    fn create_local_rtc_peer(&mut self) {
        let cloned_link = self.link.clone();

        let onicecandidate_callback = Closure::wrap(Box::new(
            move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                Some(candidate) => {
                    cloned_link.send_message(Msg::LogEvent(format!(
                        "This client made an ice candidate: {:#?}",
                        candidate.candidate()
                    )));
                    cloned_link.send_message(Msg::AddLocalIceCandidate(candidate.candidate()))
                }
                None => {}
            },
        )
            as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);

        let mut config = RtcConfiguration::new();
        // let mut ice_server = RtcIceServer::new();

        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        struct Test {
            urls : String
        };


        let val = JsValue::from_serde(&[Test{urls: String::from("stun:stun.services.mozilla.com")}]).expect("error converting IceServer to JsValue with serde");



        let config = config.ice_servers(&val);
        let client = RtcPeerConnection::new_with_configuration(&config);

        
        match client.clone() {
            Ok(client) => {          
                self.link.send_message(Msg::LogEvent(format!("successfully setup stun server!")));
                client.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));

                onicecandidate_callback.forget();
        
                self.link.send_message(Msg::RtcClientReady(client));}
                Err(err) => {self.link.send_message(Msg::LogEvent(format!("Received the following error: {:?}", err)))}

        }
 
    }
}

async fn get_local_user_media(
    link: ComponentLink<Model>,
    local: RtcPeerConnection,
) {
    let window = web_sys::window().expect("couldn't get window");
    
    let media_devices : MediaDevices = window.navigator().media_devices().expect("Couldn't get the navigator");

    let mut constraints = MediaStreamConstraints::new();
    let constraints = constraints.audio(&JsValue::from_bool(true)).video(&JsValue::from_bool(true));
   

    match media_devices.get_user_media_with_constraints(&constraints) {
        Ok(media ) => {
            match JsFuture::from(media).await {
                Ok(stream ) => {
                    match   stream.dyn_into::<MediaStream>() {
                        Ok(stream) => {
                            link.send_message(Msg::LogEvent(format!("Alright, able to get user media... should probably do something with it now!")));
                    link.send_message(Msg::StoreMediaStream(stream));
                        }
                        Err(err) => {link.send_message(Msg::LogEvent(format!("Error: {:?}", err)))}
                    }
                    
                },
                Err(err) => {link.send_message(Msg::LogEvent(format!("Error with getting user media: {:?}", err)))}
            }
        },
        Err(err) => link.send_message(Msg::LogEvent(format!("Recevied the following error while trying to get the media devices...: {:?}", err)))
    }




}

async fn create_webrtc_offer(
    link: ComponentLink<Model>,
    receiver: uuid::Uuid,
    local: RtcPeerConnection,
) {
    let offer = JsFuture::from(local.create_offer()).await.expect("error creating offer.");

    
        let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
            .expect("error getting offer sdp")
            .as_string()
            .expect("error converting offer sdp to string");

        link.send_message(Msg::SendSdpRequestToClient(receiver, offer_sdp));
    
}

async fn set_remote_webrtc_offer(
    sdp: String,
    receiver: uuid::Uuid,
    local: RtcPeerConnection,
    link: ComponentLink<Model>,
) {
    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&sdp);
    let srd_promise = local.set_remote_description(&offer_obj);
    match JsFuture::from(srd_promise).await {
        Ok(_) => {
            link.send_message(Msg::OverrideRtcPeer(local.clone()));
            link.send_message(Msg::LogEvent(format!(
                "Successfully set the remote webrtc offer!"
            )));
            create_and_set_answer_locally( receiver, local, link).await
        }
        Err(err) => link.send_message(Msg::LogEvent(format!("Error: {:?}", err))),
    }
}

async fn set_remote_webrtc_answer(
    answer_sdp: String,
    local: RtcPeerConnection,
    link: ComponentLink<Model>,
) {
    let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_obj.sdp(&answer_sdp);
    let srd_promise = local.set_remote_description(&answer_obj);
    match JsFuture::from(srd_promise).await {
        Ok(_) => {
            link.send_message(Msg::OverrideRtcPeer(local));
            link.send_message(Msg::LogEvent(format!(
                "Successfully set the remote webrtc answer!"
            )));
        }
        Err(err) => link.send_message(Msg::LogEvent(format!("Error: {:?}", err))),
    }
}

async fn create_and_set_answer_locally(
    receiver: uuid::Uuid,
    local: RtcPeerConnection,
    link: ComponentLink<Model>,
) {
    link.send_message(Msg::LogEvent(format!(
        "attempting to create answer to offer..."
    )));

    let answer = JsFuture::from(local.create_answer()).await.expect("couldn't create answer :[");

    unsafe {
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))
            .expect("error making sdp answer...")
            .as_string()
            .expect("error converting to string");
        //console_log!("pc2: answer {:?}", answer_sdp);

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let sld_promise = local.set_local_description(&answer_obj);
        match JsFuture::from(sld_promise).await {
            Ok(_) => {
                link.send_message(Msg::SendSdpResponse(receiver, answer_sdp.clone()));
                link.send_message(Msg::OverrideRtcPeer(local));
            }
            Err(err) => link.send_message(Msg::LogEvent(format!(
                "Error while trying to submit the sdp answer: {:?}",
                err
            ))),
        }
    }
}

async fn set_local_webrtc_offer(
    offer_sdp: String,
    link: ComponentLink<Model>,
    local: RtcPeerConnection,
) {
    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&offer_sdp);
    let sld_promise = local.set_local_description(&offer_obj);
    match JsFuture::from(sld_promise).await {
        Ok(_) => link.send_message(Msg::OverrideRtcPeer(local)),
        Err(err) => link.send_message(Msg::LogEvent(format!("Got the following Error: {:?}", err))),
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            local_web_rtc_connection: None,
            link,
            stream: None,
            websocket: None,
            local_ice_candidate: Vec::<String>::new(),
            event_log: Vec::<String>::new(),
            event_log_length: 5,
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
            Msg::ReportRtcDiagnostics() => {
                if self.local_web_rtc_connection.is_some(){
                    let local = self.local_web_rtc_connection.clone().expect("error unwraping the local_web_rtc_connection");
                    self.link.send_message(Msg::LogEvent(format!("RTC Connection Status:\nIce Connection State: {:?}\nSignaling State: {:?}\nIce Gathering State: {:?}", local.ice_connection_state(), local.signaling_state(), local.ice_gathering_state())));
                    true
                } else {
                    false
                }
            }
            Msg::StoreMediaStream(stream) => {
                self.stream = Some(stream);
                self.link.send_message(Msg::LogEvent(format!("Need to update the local video to include the new stream.")));
                true
            }
            Msg::OverrideRtcPeer(local) => {
                self.link
                    .send_message(Msg::LogEvent(format!("override the old WebRtcConnection")));
                    self.link.send_message(Msg::ReportRtcDiagnostics());
                self.local_web_rtc_connection = Some(local);
                true
            }
            Msg::SetupWebRtc() => {
                self.link
                    .send_message(Msg::LogEvent(format!("Setting up webRTC locally...")));
                self.create_local_rtc_peer();
                true
            }
            Msg::AddLocalIceCandidate(candidate) => {
                self.local_ice_candidate.push(candidate.clone());
                self.link.send_message(Msg::LogEvent(format!(
                    "need to send ice candidate {} to partner...",
                    candidate
                )));
                true
            }
            Msg::ClearLog => {
                self.event_log = Vec::<String>::new();
                true
            }
            Msg::IncreaseLogSize => {
                self.event_log_length = self.event_log_length.clone() + 1;
                true
            }
            Msg::DecreaseLogSize => {
                if self.event_log_length.clone() > 2 {
                    self.event_log_length = self.event_log_length.clone() - 1;
                }
                else {
                    self.link.send_message(Msg::LogEvent(format!("Can't decrease the event log size even more...")));
                }
                true
            }
            Msg::RtcClientReady(client) => {
                self.local_web_rtc_connection = Some(client);
                self.link.send_message(Msg::LogEvent(format!(
                    "local WebRTC successfully set! Able to send sdp objects to clients now!"
                )));
                self.link.send_message(Msg::ReportRtcDiagnostics());
                self.link.send_message(Msg::SetLocalMedia);
                false
            }
            Msg::SetLocalMedia => {
                let link = self.link.clone();
                let local = self.local_web_rtc_connection.clone().expect("error with unwrapping the local web rtc in setlocalmedia msg ");
                spawn_local(async move {
                    get_local_user_media(link, local).await;
                });
                true
            }
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

                let envelope = Envelope::new(
                    EntityDetails::Client(self.user_id.clone().unwrap()),
                    EntityDetails::Server,
                    None,
                    Command::ClosedConnection(self.user_id.clone().unwrap())
                );
                 self.link.send_message(Msg::LogEvent(format!("Informing the server about the desire to end the connection")));
                 self.link.send_message(Msg::SendWsMessage(envelope));

                
                true

            }
            Msg::EndWebsocketConnection => {
                let ws = self.websocket.take();

                match ws {
                    Some(ws) => {
                        ws.close().expect("Error with closing the ws connection");
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
            Msg::InitiateWebsocketConnectionProcess => 
                {

                    panic::set_hook(Box::new(console_error_panic_hook::hook));
                match WebSocket::new(WEBSOCKET_URL) {
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

                }
            }
            Msg::SetClient(client) => {
                self.client_to_model(client);

                true
            }
            Msg::UpdateUsername(username) => {
                self.username = Some(username.clone());
                let user_id = self.user_id.clone().expect("error unwrapping the user id");

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
                let local = self.local_web_rtc_connection.clone().expect("error getting the web_rtc in set local web rtc offer message");

                self.link
                    .send_message(Msg::LogEvent(format!("Set the local webRTC offer.")));
                let link = self.link.clone();

                spawn_local(async move { set_local_webrtc_offer(offer, link, local).await });

                true
            }
            Msg::MakeSdpRequestToClient(receiver) => {
                let local = self.local_web_rtc_connection.clone().expect("unable to get the local web rtc in making sdp request to client");

                let link = self.link.clone();

                spawn_local(
                    async move { create_webrtc_offer(link, receiver.clone(), local).await },
                );

                true
            }
            Msg::SendSdpRequestToClient(receiver, sdp) => {
                let sender = self.user_id.clone().expect("error getting the sender's user id in SendSdpRequestToClient msg");

                self.link
                    .send_message(Msg::SetLocalWebRtcOffer(sdp.clone()));

                let envelope = Envelope::new(
                    EntityDetails::Client(sender),
                    EntityDetails::Client(receiver),
                    Some(EntityDetails::Server),
                    Command::SdpRequest(sdp),
                );

                self.send_ws_message(envelope);
                true
            }

            Msg::SendSdpResponse(client, sdp_response) => {
                self.link.send_message(Msg::LogEvent(format!(
                    "Sending sdp response {:?} back to other client",
                    sdp_response.clone()
                )));

                let sender = self.user_id.clone().expect("error getting the user_id in SendSdpResponse");
                let receiver = client.clone();

                let envelope = Envelope::new(
                    EntityDetails::Client(sender),
                    EntityDetails::Client(receiver),
                    Some(EntityDetails::Server),
                    Command::SdpResponse(sdp_response),
                );

                self.send_ws_message(envelope);

                false
            }

            Msg::ReceiveSdpResponse(sender, sdp) => {
                self.link.send_message(Msg::LogEvent(format!(
                    "Received the following sdp reponse: {:?} from client {:?}",
                    sdp.clone(),
                    sender.clone()
                )));

                let local = self.local_web_rtc_connection.clone().expect("error unwrapping the local_web_rtc_connection in ReceiveSdpResponse");
                let link = self.link.clone();

                spawn_local(async move {set_remote_webrtc_answer(sdp,
                local,
                    link
                ).await});

                true
            }
            Msg::MakeSdpResponse(sdp, client) => {
                self.link.send_message(Msg::LogEvent(format!(
                    "Received the following sdp request: {:?} from client {:?}",
                    sdp, client
                )));

                let sdp = sdp.clone();
                let client = client.clone();

                let local = self.local_web_rtc_connection.clone().expect("error in MakeSdpReponse msg");
                let link = self.link.clone();

                spawn_local(async move { set_remote_webrtc_offer(sdp, client, local, link).await });

                true
            }
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

                    <button onclick=self.link.callback(|_| {Msg::DecreaseLogSize})> {"Decrease Log Size"} </button>
                    <button onclick=self.link.callback(|_| {Msg::IncreaseLogSize})> {"Increase Log Size"} </button>

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
