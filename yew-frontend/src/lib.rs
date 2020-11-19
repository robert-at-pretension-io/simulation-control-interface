
use wasm_bindgen::prelude::*;

//Each of the javascript api features must be added in both the YAML file and used here
use web_sys::{MessageEvent, WebSocket};

// Needed for converting boxed closures into js closures *🤷*
use wasm_bindgen::JsCast;

// God knows what evils this crate includes. 
use yew::prelude::*;

// This local trait is for shared objects between the frontend and the backend
use models::ControlMessages;

static WEBSOCKET_URL: &str = "ws://127.0.0.1:80";

struct Model {
    event_log: Vec<String>,
    user_id: String,
    user: String,
    link: ComponentLink<Self>,
    websocket: Option<WebSocket>,
}


enum Msg {
    InitialPage,
    InitiateWebsocketConnectionProcess,
    EstablishingConnectionToWebsocketServer,
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

    fn setup_websocket_object_callbacks(&mut self, ws : WebSocket) -> WebSocket{
        let cloned = self.link.clone();

        // The onmessage_callback handles MOOOOST of the logic of client-server communication
        // This callback is suuuper important lol

            // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // The only type of message that will be officially recognized is the almighty ArrayBuffer Binary Data!
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                //console_log!("message event, received arraybuffer: {:?}", abuf);
                let array = js_sys::Uint8Array::new(&abuf);
                //let len = array.byte_length() as usize;
                //console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());

                cloned.send_message(Msg::LogEvent("Received a binary message: ".into()));

                //web_sys::console::log_1(&abuf.to_string());


                match ControlMessages::deserialize(&array.to_vec()) {
                    ControlMessages::Id(new_id) => {
                        cloned.send_message(Msg::UpdateUsername(new_id))
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
            user_id: String::from("Random ID"),
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
                true
            }
            Msg::InitiateWebsocketConnectionProcess => {
                let ws = WebSocket::new(WEBSOCKET_URL).unwrap();


                let ws = self.setup_websocket_object_callbacks(ws);

                self.websocket = Some(ws);
                let messages : Vec::<Msg> = vec!(
                    Msg::LogEvent("attempting ws connection ...".to_string()),
                    Msg::EstablishingConnectionToWebsocketServer,
            ); 
                self.link.send_message_batch(messages);
                false
                



            }
            Msg::ReadyForPartner => false,
            Msg::UpdateUsername(username) => {
                self.user = username;
                true
            }
            Msg::EstablishingConnectionToWebsocketServer => {
true

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
                        <p> {format!("How's it going {}",self.user)} </p>

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
