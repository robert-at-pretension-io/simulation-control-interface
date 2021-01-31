#![recursion_limit = "1024"]
use uuid::Uuid;
use wasm_bindgen::prelude::*;

// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes.
// ^ Thanks I needed that laugh..
use yew::prelude::*;
use yew::ComponentLink;

// This local trait is for shared objects between the frontend and the backend
use models::{Client, Command, EntityDetails, Envelope};

use core::borrow;
use std::{clone, net::SocketAddr};

use std::collections::HashSet;

// all of these are for webrtc
use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Element, HtmlMediaElement, MediaDevices, MediaStream, MediaStreamConstraints, MediaStreamTrack, MediaTrackSupportedConstraints, MessageEvent, Navigator, RtcConfiguration, RtcIceCandidate, RtcIceCandidateInit, RtcIceConnectionState, RtcIceGatheringState, RtcIceServer, RtcOfferOptions, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcRtpSender, RtcRtpTransceiver, RtcRtpTransceiverDirection, RtcSdpType, RtcSessionDescriptionInit, RtcSignalingState, RtcTrackEvent, WebSocket, Window};

use console_error_panic_hook;
use std::panic;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum State {
    ConnectedToWebsocketServer,
    ConnectedToRtcPeer,
}

static WEBSOCKET_URL: &str = "wss://liminalnook.com:2096";
static STUN_SERVER: &str = "stun:stun.services.mozilla.com";

struct Model {
    local_stream: Option<MediaStream>,
    remote_stream: Option<MediaStream>,
    local_video : NodeRef,
    remote_video : NodeRef,
    event_log: Vec<String>,
    event_log_length: usize,
    connection_socket_address: Option<SocketAddr>,
    user_id: Option<uuid::Uuid>,
    username: Option<String>,
    partner: Option<Uuid>,
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
    GetUserMediaPermission,
    UpdateUsername(String),
    SetClient(Client),
    ClearLog,
    ResetPage,
    LogEvent(String),
    IncreaseLogSize,
    DecreaseLogSize,
    SetLocalMediaStream,

    SetupWebRtc(),
    RtcClientReady(RtcPeerConnection),
    StoreMediaStream(MediaStream),
    ReceivedIceCandidate(String),
    SendIceCandidate(String),
    AddLocalIceCandidate(String),
    SetLocalWebRtcOffer(String),
    CloseWebRtcConnection,

    SetLocalMedia,
    AddRemoteMediaStream(MediaStreamTrack),

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
        let ws = self
            .websocket
            .take()
            .expect("error getting the websocket :] in send_ws_message function");

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
            {for self.event_log.iter().rev().take(self.event_log_length.clone()).rev().map(|event| {
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
                        cloned.send_message(Msg::LogEvent(format!(
                            "Received the following command message: {:?}",
                            result.command.clone()
                        )));
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
                                cloned.send_message(Msg::LogEvent(format!(
                                    "Updating the clients online"
                                )));
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
                            Command::IceCandidate(ice_candidate) => {
                                cloned.send_message(Msg::ReceivedIceCandidate(ice_candidate));
                            }
                            Command::ClosedConnection(_) => cloned.send_message(Msg::ResetPage),
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
        use serde::{Deserialize, Serialize};
                #[derive(Serialize, Deserialize)]
        struct Test {
            urls: String,
        };
        let val = JsValue::from_serde(&[Test {
            urls: String::from(STUN_SERVER),
        }])
        .expect("error converting IceServer to JsValue with serde");
        
        let mut config = RtcConfiguration::new();
        let config = config.ice_servers(&val);
        let client = RtcPeerConnection::new_with_configuration(&config);

        match client.clone() {
            Ok(client) => {
                // let mut ice_server = RtcIceServer::new();

                self.link
                    .send_message(Msg::LogEvent(format!("successfully setup stun server! ")));
                self.link
                        .send_message(Msg::LogEvent(format!("Now to attach the tracks to the stream!")));

                let onicecandidate_callback = return_ice_callback(cloned_link.clone());
                client.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
                onicecandidate_callback.forget();

                let return_track_callback = return_track_added_callback(cloned_link.clone());
                client.set_ontrack(Some(return_track_callback.as_ref().unchecked_ref()));
                return_track_callback.forget();

                if let Some(my_stream) = &self.local_stream {
                    for track in my_stream.clone().get_tracks().to_vec() {
                        let track = track.dyn_into::<MediaStreamTrack>().unwrap();
                        RtcPeerConnection::add_track_0(&client,&track, my_stream);
                    }
                    self.link.send_message(Msg::LogEvent(format!("Added the local tracks to the WebRtc Connection")));
                    
                    self.link.send_message(Msg::RtcClientReady(client));
                }
                else {
                    self.link.send_message(Msg::LogEvent(format!("Aparently there is no local_stream... I guess the webcam isn't working OR permission to use the webcam was not aquired :o. This halts the progression of the application :[")));
                }


            }
            Err(err) => self.link.send_message(Msg::LogEvent(format!(
                "Received the following error: {:?}",
                err
            ))),
        }
    }
}

async fn get_local_user_media(link: ComponentLink<Model>) {
    let window = web_sys::window().expect("couldn't get window");

    let media_devices: MediaDevices = window
        .navigator()
        .media_devices()
        .expect("Couldn't get the navigator");

    let mut constraints = MediaStreamConstraints::new();
    let constraints = constraints
        .audio(&JsValue::from_bool(true))
        .video(&JsValue::from_bool(true));

    match media_devices.get_user_media_with_constraints(&constraints) {
        Ok(media) => match JsFuture::from(media).await {
            Ok(stream) => match stream.dyn_into::<MediaStream>() {
                Ok(stream) => {
                    link.send_message(Msg::LogEvent(format!("Alright, able to get user media... should probably do something with it now!")));
                    link.send_message(Msg::StoreMediaStream(stream));
                    link.send_message(Msg::SetLocalMediaStream);
                    link.send_message(Msg::InitiateWebsocketConnectionProcess);
                }
                Err(err) => link.send_message(Msg::LogEvent(format!("Error: {:?}", err))),
            },
            Err(err) => link.send_message(Msg::LogEvent(format!(
                "Error with getting user media: {:?}",
                err
            ))),
        },
        Err(err) => link.send_message(Msg::LogEvent(format!(
            "Recevied the following error while trying to get the media devices...: {:?}",
            err
        ))),
    }
}

async fn create_webrtc_offer(
    link: ComponentLink<Model>,
    receiver: uuid::Uuid,
    local: RtcPeerConnection,
) {

    let mut offer_options =   RtcOfferOptions::new();
    
    offer_options.offer_to_receive_audio(true).offer_to_receive_video(true);


    let offer = JsFuture::from(local.create_offer_with_rtc_offer_options(&offer_options))
        .await
        .expect("error creating offer.");

    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
        .expect("error getting offer sdp")
        .as_string()
        .expect("error converting offer sdp to string");

    link.send_message(Msg::SendSdpRequestToClient(receiver, offer_sdp));
}

fn return_ice_callback(cloned_link : ComponentLink<Model>) -> Closure<dyn FnMut(RtcPeerConnectionIceEvent) >{

    Closure::wrap(
        Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
            Some(candidate) => {
                cloned_link.send_message(Msg::LogEvent(format!(
                    "This client made an ice candidate: {:#?}",
                    candidate.candidate()
                )));
                cloned_link.send_message(Msg::AddLocalIceCandidate(candidate.candidate()))
                
            }
            None => {
                cloned_link.send_message(Msg::LogEvent(format!(
                    "Done getting ice candidates"
                )));
                cloned_link.send_message(Msg::ReportRtcDiagnostics());
            }
        }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,)
}


fn return_track_added_callback(cloned_link : ComponentLink<Model>) -> Closure<dyn FnMut(RtcTrackEvent) >{

    Closure::wrap(
        Box::new(move |event : RtcTrackEvent| {

            let track = event.track();
                cloned_link.send_message(Msg::LogEvent(format!("The remote track: {:?} was added to the RtcPeerConnection", track)));

            cloned_link.send_message(Msg::AddRemoteMediaStream(track));
            

        }) as Box<dyn FnMut(RtcTrackEvent)>)
}


async fn set_remote_webrtc_offer(
    remote_sdp: String,
    receiver: uuid::Uuid,
    local_stream : Option<MediaStream>,
    link: ComponentLink<Model>,
) {
    

    let cloned_link = link.clone();

        

    let mut config = RtcConfiguration::new();
    // let mut ice_server = RtcIceServer::new();

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Test {
        urls: String,
    };

    let val = JsValue::from_serde(&[Test {
        urls: String::from(STUN_SERVER),
    }])
    .expect("error converting IceServer to JsValue with serde");

    let config = config.ice_servers(&val);
    let client = RtcPeerConnection::new_with_configuration(&config);

    match client.clone() {
        Ok(local) => {


            link.send_message(Msg::LogEvent(format!("successfully setup stun server!")));

            let onicecandidate_callback =return_ice_callback(link.clone());
            local.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
            onicecandidate_callback.forget();

            let return_track_callback = return_track_added_callback(link.clone());
            local.set_ontrack(Some(return_track_callback.as_ref().unchecked_ref()));
            return_track_callback.forget();

            // This is now where the code for sending a response goes...

            let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);

            offer_obj.sdp(&remote_sdp);
            let srd_promise = local.set_remote_description(&offer_obj);
            match JsFuture::from(srd_promise).await {
                Ok(_) => {
                    link.send_message(Msg::OverrideRtcPeer(local.clone()));
                    link.send_message(Msg::LogEvent(format!(
                        "Successfully set the remote webrtc offer!"
                    )));
                    create_and_set_answer_locally(receiver, local, local_stream.clone() ,link).await
                }
                Err(err) => link.send_message(Msg::LogEvent(format!("Error: {:?}", err))),
            }
        }

        // link.send_message(Msg::RtcClientReady(client));}
        Err(err) => link.send_message(Msg::LogEvent(format!(
            "Received the following error: {:?}",
            err
        ))),
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
    local_stream : Option<MediaStream>,
    link: ComponentLink<Model>,
) {
    link.send_message(Msg::LogEvent(format!(
        "attempting to create answer to offer...This is also where the local media stream/tracks will be connected to the RTCPeerConnection"
    )));

    

    if let Some(my_stream) = &local_stream {
        for track in my_stream.clone().get_tracks().to_vec() {
            let track = track.dyn_into::<MediaStreamTrack>().unwrap();
            

            let transceiver : RtcRtpTransceiver = local.add_transceiver_with_media_stream_track(&track);

            transceiver.set_direction(RtcRtpTransceiverDirection::Sendrecv);

                // RtcPeerConnection::add_track_0(&local,&track, my_stream);
        }
        link.send_message(Msg::LogEvent(format!("Added the local tracks")));
    }
    else {
        link.send_message(Msg::LogEvent(format!("Aparently there is no local_stream... I guess the webcam isn't working OR permission to use the webcam was not aquired :o. This halts the progression of the application :[")));
    }

    

    let return_track_callback = return_track_added_callback(link.clone());
    local.set_ontrack(Some(return_track_callback.as_ref().unchecked_ref()));
    return_track_callback.forget();

    let onicecandidate_callback =return_ice_callback(link.clone());
    local.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
    onicecandidate_callback.forget();




    let answer = JsFuture::from(local.create_answer())
        .await
        .expect("couldn't create answer :[");

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
            local_stream: None,
            remote_stream: Some(MediaStream::new().unwrap()),
            local_video : NodeRef::default(),
            remote_video : NodeRef::default(),
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

            Msg::CloseWebRtcConnection => {
                match self.local_web_rtc_connection.as_ref() {
                    Some(connection) => {
                        connection.close();
                    }
                    None => {
                        self.link.send_message(Msg::LogEvent(format!("There is no local webrtc connection to close.")));
                    }
                }

                let val =self.remote_video.cast::<HtmlMediaElement>().unwrap();
                let mut stream = self.remote_stream.clone().unwrap();
                for track in stream.clone().get_tracks().to_vec() {
                    let track = track.dyn_into::<MediaStreamTrack>().unwrap();
                    self.link.send_message(Msg::LogEvent(format!("Removing the track {:?} from the remote stream", track)));
                    stream.remove_track(&track);
                    
                }
                val.set_src_object(Some(&stream));
                self.remote_stream = Some(stream);

                true
            }

            Msg::GetUserMediaPermission => {
                panic::set_hook(Box::new(console_error_panic_hook::hook));


                self.link.send_message(Msg::LogEvent(format!("getting the local webcam stream")));
                let link = self.link.clone();

                spawn_local(async move {
                    get_local_user_media(link).await;
                });
                true
            }
            Msg::ReportRtcDiagnostics() => {
                if self.local_web_rtc_connection.is_some() {
                    let local = self
                        .local_web_rtc_connection
                        .clone()
                        .expect("error unwraping the local_web_rtc_connection");
                    self.link.send_message(Msg::LogEvent(format!("::RTC Connection Status::\nIce Connection State: {:?}\nSignaling State: {:?}\nIce Gathering State: {:?}", local.ice_connection_state(), local.signaling_state(), local.ice_gathering_state())));
                    true
                } else {
                    false
                }
            }
            Msg::StoreMediaStream(stream) => {
                self.local_stream = Some(stream);
                self.link.send_message(Msg::LogEvent(format!(
                    "Need to update the local video to include the new stream."
                )));
                true
            }
            Msg::SetLocalMediaStream => {
                let val =self.local_video.cast::<HtmlMediaElement>().unwrap();
                let stream = self.local_stream.as_ref();
                val.set_src_object(stream);
                true
            }
            Msg::AddRemoteMediaStream(track) => {
                let val =self.remote_video.cast::<HtmlMediaElement>().unwrap();
                let stream = self.remote_stream.clone().unwrap();
                stream.add_track(&track);
                val.set_src_object(Some(&stream));
                self.remote_stream = Some(stream);
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
                    candidate.clone()
                )));
                self.link.send_message(Msg::SendIceCandidate(candidate));
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
                } else {
                    self.link.send_message(Msg::LogEvent(format!(
                        "Can't decrease the event log size even more..."
                    )));
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
                let local = self
                    .local_web_rtc_connection
                    .clone()
                    .expect("error with unwrapping the local web rtc in setlocalmedia msg ");

                true
            }
            Msg::ResetPage => {
                self.reset_state();

                self.link.send_message(Msg::CloseWebRtcConnection);

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
                    Command::ClosedConnection(self.user_id.clone().unwrap()),
                );
                self.link.send_message(Msg::LogEvent(format!(
                    "Informing the server about the desire to end the connection"
                )));
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
            Msg::InitiateWebsocketConnectionProcess => {
                

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
            Msg::ReceivedIceCandidate(ice_candidate) => {
                let local = self.local_web_rtc_connection.clone().unwrap();

                let mut mid : Option<String>  = None;

                // Right here is where we get the transceiver from the local web_rtc connection. With it we can get the media id (MID). Then the RtcIceCandidate it is initialized with the MID. I hate not knowing the abstractions that are assumed to be understood with a third party libraries 😭

                let mut init = RtcIceCandidateInit::new(&ice_candidate);
                

                for transceiver in local.get_transceivers().to_vec() {
                    let transceiver = transceiver.dyn_into::<RtcRtpTransceiver>().unwrap();



                    
                    let link = self.link.clone();

                     match transceiver.mid().clone()
                     {
                         Some(mid) => {
                             init.sdp_mid(Some(&mid));
                         }
                         None => {
                             link.send_message(Msg::LogEvent(format!("Wasn't able to set the mid value for the following transceiver: {:?}", transceiver)));
                         }
                     }

                     
                }



                let candidate = RtcIceCandidate::new(&init).unwrap();


                let link = self.link.clone();

                spawn_local(async move { 

                    match wasm_bindgen_futures::JsFuture::from(local.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate))).await {
                        Ok(_local) => {
                            let local = local.dyn_into::<RtcPeerConnection>().expect("Couldn't convert the RtcPeerConnection");
                            link.send_message(Msg::OverrideRtcPeer(local));
                            link.send_message(Msg::LogEvent(format!("Successfully added ice candidate from remote peer!")));
                        }
                            Err(err) => {link.send_message(Msg::LogEvent(format!("Had the following error trying to reset the local RtcPeerConnection: {:?}", err)));}
                    }

                    
    

                 });

                true
            },
            Msg::SendIceCandidate(ice_candidate) => {
                let partner_id = self.partner.clone().unwrap();
                
                let envelope = Envelope::new(
                    EntityDetails::Client(self.user_id.unwrap()),
                    EntityDetails::Client(partner_id),
                    Some(EntityDetails::Server),
                    Command::IceCandidate(ice_candidate.clone())
                );

                self.send_ws_message(envelope);


                true
            },
            Msg::SetLocalWebRtcOffer(offer) => {
                let local = self
                    .local_web_rtc_connection
                    .clone()
                    .expect("error getting the web_rtc in set local web rtc offer message");

                self.link
                    .send_message(Msg::LogEvent(format!("Set the local webRTC offer.")));
                let link = self.link.clone();

                spawn_local(async move { set_local_webrtc_offer(offer, link, local).await });

                true
            }
            Msg::MakeSdpRequestToClient(receiver) => {
                self.partner = Some(receiver);

                let local = self
                    .local_web_rtc_connection
                    .clone()
                    .expect("unable to get the local web rtc in making sdp request to client");

                let link = self.link.clone();

                spawn_local(
                    async move { create_webrtc_offer(link, receiver.clone(), local).await },
                );

                true
            }
            Msg::SendSdpRequestToClient(receiver, sdp) => {
                let sender = self
                    .user_id
                    .clone()
                    .expect("error getting the sender's user id in SendSdpRequestToClient msg");

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

                let sender = self
                    .user_id
                    .clone()
                    .expect("error getting the user_id in SendSdpResponse");
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

                let local = self
                    .local_web_rtc_connection
                    .clone()
                    .expect("error unwrapping the local_web_rtc_connection in ReceiveSdpResponse");
                let link = self.link.clone();

                spawn_local(async move { set_remote_webrtc_answer(sdp, local, link).await });

                true
            }
            Msg::MakeSdpResponse(sdp, client) => {

                self.partner = Some(client.clone());

                self.link.send_message(Msg::LogEvent(format!(
                    "Received the following sdp request: {:?} from client {:?}",
                    sdp, client
                )));

                let sdp = sdp.clone();
                let client = client.clone();

                let local = self
                    .local_web_rtc_connection
                    .clone()
                    .expect("error in MakeSdpReponse msg");
                let link = self.link.clone();
                let local_stream = self.local_stream.clone();

                spawn_local(async move { set_remote_webrtc_offer(sdp, client, local_stream, link).await });

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
            // <h1> {"Local Video"} </h1>
            <video  width="320" height="240" muted=true autoplay=true controls=true ref=self.local_video.clone()> </video>


            // <h1> "Remote Video" </h1>
            <video  width="320" height="240" autoplay=true controls=true ref=self.remote_video.clone()> </video>

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
                        Msg::GetUserMediaPermission
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
                            <button onclick=self.link.callback(|_| {
                                Msg::CloseWebRtcConnection
                            })>
                                {"Close WebRtcConnection"}
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
