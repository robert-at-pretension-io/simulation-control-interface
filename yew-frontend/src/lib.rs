use wasm_bindgen::prelude::*;
use yew::prelude::*;
use web_sys::{MessageEvent, WebSocket};
use wasm_bindgen::JsCast;
use js_sys;

use models::ControlMessages;

static WEBSOCKET_URL : &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
    current_state: State,
    user: String,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
}

enum State {
    WelcomeScreen,
    ConnectedToWebsocketServer,
}

enum Msg {
    ConnectToServer,
    ReadyForPartner,
    UpdateUsername(String),
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
            current_state: State::WelcomeScreen,
            user: String::from("Random User"),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ServerSentWsMessage(message) => {
                self.event_log.push(message);
                true
            },
            Msg::ConnectToServer => {

                let ws =  WebSocket::new(WEBSOCKET_URL).unwrap();
                //TODO: setup the websocket connection here...
                // * add listener and stuff?
                // * the listener should send messages to YEW
                
                let cloned = self.link.clone();

                // The onmessage_callback handles MOOOOST of the logic of client-server communication
                // This callback is suuuper important lol
                let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            //web_sys::console::log_1(&txt);

                            cloned.send_message(Msg::ServerSentWsMessage(txt.into()))
                    } 

                    else if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                        //console_log!("message event, received arraybuffer: {:?}", abuf);
                        let array = js_sys::Uint8Array::new(&abuf);
                        //let len = array.byte_length() as usize;
                        //console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());


                        web_sys::console::log_1(&abuf.to_string());
                        

                        match ControlMessages::deserialize(&array.to_vec()) {
                            ControlMessages::Id(new_id) => {
                                //update the clients id
                            }

                            ControlMessages::Message(message) => {
                                cloned.send_message(Msg::ServerSentWsMessage(message.into()))
                            }

                            ControlMessages::ServerInitiated => {
                                cloned.send_message(Msg::ServerSentWsMessage(String::from("Oh.. I guess the server said hi! ... Wow. I'm so embarassed!")))
                            }

                        }


                    }
                }) as Box<dyn FnMut(MessageEvent)>);
                // set message event handler on WebSocket
                ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                // forget the callback to keep it alive
                onmessage_callback.forget();
                
                self.websocket = Some(ws);
                self.current_state = State::ConnectedToWebsocketServer;
                true
            }
            Msg::ReadyForPartner => false,
            Msg::UpdateUsername(username) => {
                self.user = username;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        match self.current_state {
            State::WelcomeScreen => {
                html! {
                    <div>
                        <h1>
                        {"Welcome!"}
                        </h1>

                        <button onclick=self.link.callback(|_| {
                            Msg::ConnectToServer
                        })>
                            {"Click here to connect to the server."}
                        </button>



                    </div>
                }
            }
            State::ConnectedToWebsocketServer => {
                
                // TODO: display the websocket events here!
                html! {
                    <div>
                        <h1> {"You're connected to the server!"} </h1>
                        {if false {html!{<p> {"Cool"} </p>}} else {html!{<></>}}   }
                        <p> {format!("Here are some details about the connection:\n {:?}", self.websocket)} </p>
                        {self.show_events_in_table() }
                    </div>

                    

                }
            }
        }
    }
}



#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
