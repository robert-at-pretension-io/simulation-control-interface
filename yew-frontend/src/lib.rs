use wasm_bindgen::prelude::*;
use yew::prelude::*;

struct Model {
    event_log: Vec<String>,
    current_state: State,
    user: String,
    link: ComponentLink<Self>,
}

enum State {
    WelcomeScreen,
    ConnectedToWebsocketServer,
}

enum Msg {
    ConnectToServer,
    ReadyForPartner,
    UpdateUsername(String),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            link,
            event_log: Vec::<String>::new(),
            current_state: State::WelcomeScreen,
            user: String::from("Random User"),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ConnectToServer => {
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
                            {"Connect to the server."}
                        </button>

                    </div>
                }
            }
            State::ConnectedToWebsocketServer => {
                html! {
                    <h1> {"You're connected to the server!"} </h1>


                }
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
