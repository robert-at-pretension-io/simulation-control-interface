#![recursion_limit="256"]

use wasm_bindgen::prelude::*;

//Each of the javascript api features must be added in both the YAML file and used here
use web_sys::{MessageEvent, WebSocket};

// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes. 
use yew::prelude::*;

// This local trait is for shared objects between the frontend and the backend
use models::{ControlMessages, Client};

use std::collections::HashSet;
#[derive(Hash, Eq, PartialEq, Debug)]
enum State {
    ConnectedToWebsocketServer,
    ConnectedToRtcPeer,
}

static WEBSOCKET_URL: &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
    client: Client,
    partner: Option<Client>,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
    peers: HashSet<Client>,
    states : HashSet<State>
}


enum Msg {
    InitiateWebsocketConnectionProcess,
    UpdateUsername(Client),
    LogEvent(String),
    ServerSentWsMessage(String),
    UpdateOnlineUsers(HashSet<Client>),
    AddState(State),
    RemoveState(State),
    CloseWebsocketConnection
}

extern crate web_sys;

impl Model {
    fn show_events_in_table(&self) -> Html {
        html!(
            <>
            {for self.event_log.iter().map(|event| {
                html!(<div> <p> {event} </p> </div>)
            })  }
            </>
        )
    }

    fn setup_websocket_object_callbacks(&mut self, ws : WebSocket) -> WebSocket{
        let cloned = self.link.clone();

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // The only type of message that will be officially recognized is the almighty ArrayBuffer Binary Data!
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);

                match ControlMessages::deserialize(&array.to_vec()) {
                    Ok(result) => {
                        match result {
                            ControlMessages::Client(client) => {
                                cloned.send_message(Msg::UpdateUsername(client))
                            }
        
                            ControlMessages::Message(message, directionality) => {
                                cloned.send_message(Msg::ServerSentWsMessage(message));
                            }
        
                            ControlMessages::ServerInitiated(info) => {
                            //     let messages = vec!(Msg::LogEvent(format!("{:?} Connected To Websocket Server!", info)), 
                            //     Msg::AddState(State::ConnectedToWebsocketServer)
                            // );
                            //     cloned.send_message_batch(messages);
        
                                cloned.send_message(Msg::LogEvent(format!("{:?} Connected To Websocket Server!", info)))
                            }
        
                            ControlMessages::OnlineClients(clients) => {
                                cloned.send_message(Msg::UpdateOnlineUsers(clients))
                            }
                        }
                    }
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
            client: Client{user_id: String::from("Random ID"), username: String::from("Random Username"), current_socket_addr: None},
            partner: None,
            peers: HashSet::<Client>::new(),
            states: HashSet::<State>::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::CloseWebsocketConnection => {
                let ws = self.websocket.take();

                match ws {
                    Some(ws) => {
                        ws.close().unwrap();
                        self.states.remove(&State::ConnectedToWebsocketServer)
                    },
                    None => {
                        self.states.remove(&State::ConnectedToWebsocketServer)
                    }
                }
            },
            Msg::AddState(state) =>
            {self.states.insert(state)
            }
            Msg::RemoveState(state) =>
            {
                self.states.remove(&state)
            }
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
                        let messages : Vec::<Msg> = vec!(
                            Msg::LogEvent("attempting ws connection ...".to_string()),
                            Msg::AddState(State::ConnectedToWebsocketServer)
                    ); 
                        self.link.send_message_batch(messages);
                        true
                    }
                    Err(err) => {
                        self.link.send_message(Msg::LogEvent(format!("error: {:?}", err)));
                        true
                    }
                }


                
                



            }
            Msg::UpdateUsername(client) => {
                self.client = client;
                true
            }
            Msg::UpdateOnlineUsers(clients) => {
                self.peers = clients;
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
                        <h1>
                        {"Welcome!"}
                        </h1>
                        <p> {format!("How's it going {}",self.client.username)} </p>

                        <div>
                        <p> {format!("The following details the event log of the application:")} </p>
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
                                html!(<button onclick=self.link.callback(|_| {
                                    Msg::CloseWebsocketConnection
                                })>
                                    {"Disconnect"}
                                </button>)
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
