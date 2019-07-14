
// TODO: Ensure that these are sufficient.

pub trait Display {
    fn refresh(&mut self, vram: &[bool]);
}

pub trait Sound: Send {
    fn start_buzz(&mut self);

    fn stop_buzz(&mut self);
}

pub trait Input {
    fn poll(&self, key: u8) -> bool;

    fn block(&self) -> u8;
}
