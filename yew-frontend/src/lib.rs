#![recursion_limit = "512"]

use wasm_bindgen::prelude::*;

//Each of the javascript api features must be added in both the YAML file and used here
use web_sys::{MessageEvent, WebSocket};

// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes.
use yew::prelude::*;

// This local trait is for shared objects between the frontend and the backend
use models::{Client, ControlMessages, MessageDirection};

use std::collections::HashSet;
#[derive(Hash, Eq, PartialEq, Debug)]
enum State {
    ConnectedToWebsocketServer,
    ConnectedToRtcPeer,
}

static WEBSOCKET_URL: &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
    client: Option<Client>,
    partner: Option<Client>,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
    peers: HashSet<Client>,
    states: HashSet<State>,
}

impl Model {
    fn reset_state(&mut self) {
            self.client = None;
            self.partner =  None;
            self.websocket = None;
            self.peers = HashSet::new();
            self.states = HashSet::new();
            
    }
}

enum Msg {
    ResetPage,
    InitiateWebsocketConnectionProcess,
    UpdateUsername(Client),
    LogEvent(String),
    ServerSentWsMessage(String),
    UpdateOnlineUsers(HashSet<Client>),
    AddState(State),
    RemoveState(State),
    CloseWebsocketConnection,
    SendWsMessage(ControlMessages),
}

extern crate web_sys;

impl Model {
    fn send_ws_message(&mut self, data: &[u8]) {
        let ws = self.websocket.take().unwrap();

        ws.send_with_u8_array(data);

        self.websocket = Some(ws);
    }
    fn show_events_in_table(&self) -> Html {
        html!(
            <>
            {for self.event_log.iter().map(|event| {
                html!(<li> {event} </li>)
            })  }
            </>
        )
    }

    fn show_peers_online(&self) -> Html {
        html!(

            <ul>
            {for self.peers.iter().map(|client| {
                html!(<li> {format!("{:?} : {:?}", client.username , client.current_socket_addr)} </li>)
            })  }

            </ul>
        )
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
                        ControlMessages::ClientInfo(client) => {
                            cloned.send_message(Msg::UpdateUsername(client))
                        }

                        ControlMessages::Message(message, directionality) => {
                            cloned.send_message(Msg::ServerSentWsMessage(message));
                        }

                        ControlMessages::ServerInitiated(info) => cloned.send_message(
                            Msg::LogEvent(format!("{:?} Connected To Websocket Server!", info)),
                        ),

                        ControlMessages::OnlineClients(clients, round_number) => {
                            cloned.send_message(Msg::UpdateOnlineUsers(clients))
                        }
                        ControlMessages::ReadyForPartner(direction) => {}
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
            client: None,
            partner: None,
            peers: HashSet::<Client>::new(),
            states: HashSet::<State>::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ResetPage => {
                self.reset_state();
                true
            }
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
                    ControlMessages::ClientInfo(_) => self.link.send_message(Msg::LogEvent(
                        format!("ClientInfo isn't implemented on the client side."),
                    )),
                    ControlMessages::Message(_, message_direction) => match message_direction
                    {
                        MessageDirection::ClientToClient(_, _) => {
                            self.send_ws_message(&control_message.serialize());
                        }
                        MessageDirection::ServerToClient(_) => {}
                        MessageDirection::ClientToServer(_t) => {
                            self.send_ws_message(&control_message.serialize());
                        }
                    },
                    ControlMessages::OnlineClients(_, _) => self.link.send_message(Msg::LogEvent(
                        format!("OnlineClients isn't implemented on the client side."),
                    )),
                    ControlMessages::ReadyForPartner(client) => {
                        self.send_ws_message(&ControlMessages::ReadyForPartner(client).serialize())
                    }
                    ControlMessages::ClosedConnection(_) => {}
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
                        Msg::AddState(State::ConnectedToWebsocketServer),
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
            Msg::UpdateUsername(client) => {
                self.client = Some(client);
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
                        html!(<div>

                            {if !self.peers.is_empty() {
                                html!(
                            <div>
                            <h1> {"Peers online:"} </h1>
                            {self.show_peers_online()}
                            </div>
                                )
                            }  else {html!(<h1> {"No peers online this round."} </h1>)} }

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
