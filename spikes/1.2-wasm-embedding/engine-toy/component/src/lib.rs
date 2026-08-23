#[allow(warnings)]
mod bindings;

use bindings::exports::pact::toy_engine::pipe::Guest;

struct Engine;

impl Guest for Engine {
    fn call(request: Vec<u8>) -> Vec<u8> {
        toy_logic::handle_frame(&request)
    }
}

bindings::export!(Engine with_types_in bindings);
