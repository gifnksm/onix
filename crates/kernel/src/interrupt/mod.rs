use core::marker::PhantomData;

mod imp;
pub mod timer;
pub mod trap;

pub fn disable() -> Guard {
    Guard {
        state: imp::read_and_disable(),
        _not_send: PhantomData,
    }
}

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

#[derive(Debug)]
pub struct Guard {
    state: imp::State,
    _not_send: PhantomData<*mut ()>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        imp::restore(self.state);
    }
}
