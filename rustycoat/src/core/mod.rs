use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub mod clock;
pub mod memory;
pub mod ports;

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
                handle.join().ok();
            }
        }
    }
}

impl Default for Computer {
    fn default() -> Computer {
        Computer::new()
    }
}
