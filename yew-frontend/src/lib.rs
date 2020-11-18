use js_sys;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use yew::prelude::*;

use models::ControlMessages;

static WEBSOCKET_URL: &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
    user: String,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
}


enum Msg {
    InitialPage,
    InitiateWebsocketConnectionProcess,
    EstablishingConnectionToWebsocketServer(u8),
    ConnectedToWebsocketServer,
    ReadyForPartner,
    UpdateUsername(String),
    LogEvent(String),
    ServerSentWsMessage(String),
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
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            link,
            websocket: None,
            event_log: Vec::<String>::new(),
            user: String::from("Random User"),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::InitialPage => {
                true
            }
            Msg::LogEvent(event) => {
                self.event_log.push(event);
                true
            }
            Msg::ServerSentWsMessage(message) => {
                self.link.send_message(Msg::LogEvent(message));
                false
            }
            Msg::InitiateWebsocketConnectionProcess => {
                let ws = WebSocket::new(WEBSOCKET_URL).unwrap();

                if ws.ready_state() == 1 {
                    self.websocket = Some(ws);

                    let messages : Vec<Msg> = vec!(
                        Msg::LogEvent("The connection is open and ready to communicate.".to_string()),
                        Msg::ConnectedToWebsocketServer
                    );
                    self.link.send_message_batch(messages);
                    false
                }
                else {

                self.websocket = Some(ws);
                let messages : Vec::<Msg> = vec!(
                    Msg::LogEvent("attempting ws connection ...".to_string()),
                    Msg::EstablishingConnectionToWebsocketServer(1),
            ); 
                self.link.send_message_batch(messages);
                false
                }



            }
            Msg::ReadyForPartner => false,
            Msg::UpdateUsername(username) => {
                self.user = username;
                true
            }
            Msg::EstablishingConnectionToWebsocketServer(attempt) => {
                if attempt > 3 {
                    let messages = vec![
                        Msg::LogEvent("Ran out of connection attempts...".to_string()),
                        Msg::InitialPage
                    ];

                    self.link.send_message_batch(messages);
                    true
                }
                else {
                let state = match &self.websocket {
                    Some(ws) => {
                        
                        ws.ready_state()
                    },
                    None => {
                        let messages : Vec<Msg> = vec!(
                            Msg::LogEvent("Somehow there is no websocket...".to_string()),
                            Msg::InitiateWebsocketConnectionProcess
                        );
                        self.link.send_message_batch(messages);
                        3 // we'll just show an error because the websocket wasn't 
                    }
                };

                self.link.send_message(Msg::LogEvent(format!("Current State: {}", state)));

                if state == WebSocket::OPEN  {
                    let messages : Vec<Msg> = vec!(
                        Msg::LogEvent("The connection is open and ready to communicate.".to_string()),
                        Msg::ConnectedToWebsocketServer
                    );
                    self.link.send_message_batch(messages);
                   } 

               else if state == WebSocket::CONNECTING  {
                   
                let messages : Vec<Msg> = vec!(
                    Msg::LogEvent("Socket has been created. The connection is not yet open.".to_string()),
                    Msg::EstablishingConnectionToWebsocketServer(attempt + 1)
                );
                self.link.send_message_batch(messages);
               } 
               
               else if state == WebSocket::CLOSED{
                let messages : Vec<Msg> = vec!(
                    Msg::LogEvent("The connection is closed or couldn't be opened.".to_string()),
                    Msg::InitiateWebsocketConnectionProcess
                );
                self.link.send_message_batch(messages);
               }
                // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
                // ws.readyState is an enum with the values...
//                  Value 	State 	    Description
//                  0 	    CONNECTING 	Socket has been created. The connection is not yet open.
//                  1 	    OPEN 	    The connection is open and ready to communicate.
//                  2 	    CLOSING 	The connection is in the process of closing.
//                  3 	    CLOSED 	    The connection is closed or couldn't be opened.
true
}
            }
            Msg::ConnectedToWebsocketServer => {
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

                        <button onclick=self.link.callback(|_| {
                            Msg::InitiateWebsocketConnectionProcess
                        })>
                            {"Click here to connect to the server."}
                        </button>



                    </div>
                }

        }
    }



#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
