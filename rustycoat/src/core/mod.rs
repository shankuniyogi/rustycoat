use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub mod clock;
pub mod memory;

pub enum Pin {
    Output { value: u8, s: Sender<u8> },
    Input { value: u8, r: Receiver<u8> },
    None(u8),
}

impl Pin {
    pub fn new(initial_value: u8) -> Self {
        Pin::None(initial_value)
    }

    pub fn connect_to(&mut self, target: &mut Self) {
        let (s, r): (Sender<u8>, Receiver<u8>) = mpsc::channel();
        let value = if let Pin::None(initial_value) = self {
            *initial_value
        } else {
            panic!("Pin already connected");
        };
        *self = Pin::Output { value, s };
        *target = Pin::Input { value, r };
    }

    pub fn update(&mut self, new_value: u8) {
        match self {
            Pin::Output { value, s } => {
                *value = new_value;
                s.send(new_value).unwrap();
            },
            Pin::Input { .. } => panic!("Attempt to send to an input port"),
            _ => (),
        }
    }

    pub fn wait(&mut self) -> u8 {
        match self {
            Pin::Input { r, value } => {
                if let Ok(new_value) = r.recv() {
                    *value = new_value;
                }
                *value
            },
            _ => panic!("Attempt to receive from a non-input port"),
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            Pin::Output { value, .. } => *value,
            Pin::Input { value, .. } => *value,
            Pin::None(value) => *value,
        }
    }
}

pub trait Component: Send {
    fn run(&mut self, stop: Arc<AtomicBool>);
}

enum ComponentState {
    Initial(Box<dyn Component>),
    Running {
        handle: JoinHandle<()>,
        stop: Arc<AtomicBool>,
    },
    None,
}

pub struct Computer {
    components: Vec<ComponentState>,
}

impl Computer {
    pub fn new() -> Computer {
        Computer { components: Vec::new() }
    }

    pub fn add<T>(&mut self, c: T) -> &mut dyn Component
    where
        T: Component + Sized + 'static,
    {
        let c = Box::new(c);
        self.components.push(ComponentState::Initial(c));
        match self.components.last_mut().unwrap() {
            ComponentState::Initial(c) => c.as_mut(),
            _ => panic!("unreachable"),
        }
    }

    pub fn start(&mut self) {
        for component in self.components.iter_mut() {
            if let ComponentState::Initial(mut c) = mem::replace(component, ComponentState::None) {
                let stop = Arc::new(AtomicBool::new(false));
                let stop_clone = stop.clone();
                let handle = thread::spawn(move || {
                    c.run(stop_clone);
                });
                *component = ComponentState::Running { handle, stop };
            } else {
                panic!("component already running");
            }
        }
    }

    pub fn stop(&mut self) {
        for component in self.components.iter() {
            if let ComponentState::Running { stop, .. } = component {
                stop.store(true, Ordering::Relaxed);
            }
        }
        for component in self.components.iter_mut() {
            if let ComponentState::Running { handle, .. } = mem::replace(component, ComponentState::None) {
                handle.join().unwrap();
            }
        }
    }
}

impl Default for Computer {
    fn default() -> Computer {
        Computer::new()
    }
}
