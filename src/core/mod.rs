use crossbeam_channel::{unbounded, Receiver, Sender};
use iui::prelude::*;
use std::cell::RefCell;
use std::io::stdin;
use std::mem;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub mod clock;
pub mod memory;
pub mod ports;

pub trait AsyncComponent: Send {
    fn run(&mut self, stop: Arc<AtomicBool>);
}

pub trait SyncComponent {
    fn requires_ui(&self) -> bool;
    fn start(&mut self, ui: Option<&iui::UI>);
    fn tick(&mut self, ui: Option<&iui::UI>);
    fn stop(&mut self, ui: Option<&iui::UI>);
}

enum AsyncComponentState {
    Initial(Box<dyn AsyncComponent>),
    Running(JoinHandle<()>),
    None,
}

pub struct Computer {
    async_components: Vec<AsyncComponentState>,
    sync_components: Vec<Rc<RefCell<dyn SyncComponent>>>,
    stop: Arc<AtomicBool>,
    requires_ui: bool,
    iui: Option<iui::UI>,
}

impl Computer {
    pub fn new() -> Self {
        Self {
            async_components: Vec::new(),
            sync_components: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            requires_ui: false,
            iui: None,
        }
    }

    pub fn add_async<T>(&mut self, c: T) -> &mut dyn AsyncComponent
    where
        T: AsyncComponent + Sized + 'static,
    {
        let c = Box::new(c);
        self.async_components.push(AsyncComponentState::Initial(c));
        match self.async_components.last_mut().unwrap() {
            AsyncComponentState::Initial(c) => c.as_mut(),
            _ => panic!("unreachable"),
        }
    }

    pub fn add_sync<T>(&mut self, c: T) -> Rc<RefCell<dyn SyncComponent>>
    where
        T: SyncComponent + Sized + 'static,
    {
        let c = Rc::new(RefCell::new(c));
        let ret = c.clone();
        self.sync_components.push(c);
        self.requires_ui |= ret.borrow().requires_ui();
        ret
    }

    pub fn run(&mut self) {
        self.start();
        let iui = self.iui.clone();
        if let Some(iui) = &iui {
            let mut event_loop = iui.event_loop();
            event_loop.on_tick(iui, || self.tick());
            event_loop.run_delay(iui, 100);
        } else {
            let (s, r): (Sender<()>, Receiver<()>) = unbounded();
            thread::spawn(move || {
                let mut buffer = String::new();
                stdin().read_line(&mut buffer).unwrap();
                s.send(()).unwrap();
            });
            println!("Hit enter to stop");
            while r.recv().is_err() {
                thread::sleep(Duration::from_millis(100));
                self.tick();
            }
        }
        self.stop();
    }

    pub fn start(&mut self) {
        if self.requires_ui {
            self.iui = Some(UI::init().expect("Couldn't initialize UI library"));
        }
        self.stop = Arc::new(AtomicBool::new(false));
        for component in self.async_components.iter_mut() {
            if let AsyncComponentState::Initial(mut c) = mem::replace(component, AsyncComponentState::None) {
                let stop_clone = self.stop.clone();
                let handle = thread::spawn(move || {
                    c.run(stop_clone);
                });
                *component = AsyncComponentState::Running(handle);
            } else {
                panic!("async component already running");
            }
        }
        for component in self.sync_components.iter_mut() {
            component.borrow_mut().start(self.iui.as_ref());
        }
    }

    pub fn tick(&mut self) {
        for component in self.sync_components.iter_mut() {
            component.borrow_mut().tick(self.iui.as_ref());
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for component in self.async_components.iter_mut() {
            if let AsyncComponentState::Running(handle) = mem::replace(component, AsyncComponentState::None) {
                handle.join().ok();
            }
        }
        for component in self.sync_components.iter_mut() {
            component.borrow_mut().stop(self.iui.as_ref());
        }
    }
}

impl Default for Computer {
    fn default() -> Computer {
        Computer::new()
    }
}
