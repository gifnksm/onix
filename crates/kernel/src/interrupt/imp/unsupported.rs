#[derive(Debug, Clone, Copy)]
pub struct State();

impl State {
    pub const fn new() -> Self {
        Self()
    }
}

pub fn read_and_disable() -> State {
    unimplemented!("unsupported architecture");
}

pub fn is_enabled() -> bool {
    unimplemented!("unsupported architecture");
}

pub fn restore(_state: State) {
    unimplemented!("unsupported architecture");
}
