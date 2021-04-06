use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;

use web_sys::{CustomEventInit, CustomEvent,HtmlElement,
Window, 
Document, Event, EventTarget};

use js_sys::JSON;

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    

    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello rusty world!"));

    let mut init = CustomEventInit::new();
    let mut message_details = js_sys::JSON::parse("{\"test\": \"value\"}").unwrap();

    let custom_event = CustomEvent::new_with_event_init_dict(
        "new_message", init.detail(&message_details)
    ).unwrap();

    let window = web_sys::window().expect("global window does not exists");    
    let document = window.document().expect("expecting a document on window");
    //let body = document.body().expect("document expect to have have a body");
    let val = document.get_element_by_id("app")
    .unwrap()
    .dyn_into::<web_sys::HtmlElement>()
    .unwrap();

    val.dispatch_event(&custom_event).unwrap();
    
    Ok(())
}



