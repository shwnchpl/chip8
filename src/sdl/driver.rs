use std::sync::mpsc::{Sender, Receiver};

use crate::core::driver::{Input, Sound, Display};
use super::io;

pub struct SoundDriver {
    pub cido_tx: Sender<io::Command>,
}

impl Sound for SoundDriver {
    fn start_buzz(&self) {
        self.cido_tx.send(io::Command::BuzzStart).unwrap();
    }

    fn stop_buzz(&self) {
        self.cido_tx.send(io::Command::BuzzStop).unwrap();
    }
}

pub struct InputDriver {
    pub codi_rx: Receiver<io::Key>,
    pub cido_tx: Sender<io::Command>,
}

impl Input for InputDriver {
    fn poll(&self, key: u8) -> bool {
        self.cido_tx.send(io::Command::KeyPoll(key)).unwrap();
        self.codi_rx.recv() == Ok(Some(key))
    }

    fn block(&self) -> u8 {
        self.cido_tx.send(io::Command::KeyBlock).unwrap();
        self.codi_rx.recv().unwrap_or(Some(0)).unwrap()
    }
}

pub struct DisplayDriver {
    pub cido_tx: Sender<io::Command>,
}

impl Display for DisplayDriver {
    fn refresh(&mut self, vram: &[bool]) {
        self.cido_tx.send(
            io::Command::DisplayRefresh(
                vram.to_owned()
        )).unwrap();
    }
}
