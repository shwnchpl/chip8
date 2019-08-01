use std::sync::mpsc::Sender;

use sdl2::audio::{AudioCallback, AudioDevice};

pub struct SquareWave {
    pub phase_inc: f32,
    pub phase: f32,
    pub volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 { self.volume } else { -self.volume };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub type Buzzer = AudioDevice<SquareWave>;

pub type Key = Option<u8>;

pub enum Command {
    BuzzStart,
    BuzzStop,
    DisplayRefresh(Vec<bool>),
    KeyBlock,
    KeyChanSet(Option<Sender<Key>>),
    KeyPoll(u8),
    Quit,
}
