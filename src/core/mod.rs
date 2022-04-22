use crossbeam_channel::{unbounded, Receiver, Sender};
use iui::controls::*;
use iui::prelude::*;
use std::cell::RefCell;
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

enum AsyncComponentEntry {
    Initial(Box<dyn AsyncComponent>),
    Running(JoinHandle<()>),
    None,
}

pub trait SyncComponent {
    fn start(&mut self);
    fn tick(&mut self);
    fn stop(&mut self);
}

pub trait UiComponent: SyncComponent {
    fn create_control(&mut self, self_ref: &Rc<RefCell<dyn UiComponent>>, ui: iui::UI) -> Control;
}

enum SyncComponentEntry {
    UI(Rc<RefCell<dyn UiComponent>>),
    NonUI(Rc<RefCell<dyn SyncComponent>>),
}

pub struct Computer {
    async_components: Vec<AsyncComponentEntry>,
    sync_components: Vec<SyncComponentEntry>,
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
        self.async_components.push(AsyncComponentEntry::Initial(c));
        match self.async_components.last_mut().unwrap() {
            AsyncComponentEntry::Initial(c) => c.as_mut(),
            _ => panic!("unreachable"),
        }
    }

    pub fn add_sync<T>(&mut self, c: T) -> Rc<RefCell<dyn SyncComponent>>
    where
        T: SyncComponent + Sized + 'static,
    {
        let c = Rc::new(RefCell::new(c));
        let ret = c.clone();
        self.sync_components.push(SyncComponentEntry::NonUI(c));
        ret
    }

    pub fn add_ui<T>(&mut self, c: T) -> Rc<RefCell<dyn UiComponent>>
    where
        T: UiComponent + Sized + 'static,
    {
        let c = Rc::new(RefCell::new(c));
        let ret = c.clone();
        self.sync_components.push(SyncComponentEntry::UI(c));
        self.requires_ui = true;
        ret
    }

    pub fn run(&mut self) {
        self.start();
        let iui = self.iui.clone();
        if let Some(iui) = &iui {
            let mut event_loop = iui.event_loop();
            event_loop.on_tick(iui, || self.tick());
            event_loop.run_delay(iui, 1);
        } else {
            let (s, r): (Sender<()>, Receiver<()>) = unbounded();
            ctrlc::set_handler(move || {
                s.send(()).unwrap();
            })
            .expect("Error setting Ctrl-C handler");
            println!("Hit Ctrl-C to stop");
            while r.try_recv().is_err() {
                thread::sleep(Duration::from_millis(1));
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
            if let AsyncComponentEntry::Initial(mut c) = mem::replace(component, AsyncComponentEntry::None) {
                let stop_clone = self.stop.clone();
                let handle = thread::spawn(move || {
                    c.run(stop_clone);
                });
                *component = AsyncComponentEntry::Running(handle);
            } else {
                panic!("async component already running");
            }
        }
        for component in self.sync_components.iter_mut() {
            match component {
                SyncComponentEntry::UI(component) => {
                    let ui = self.iui.as_ref().unwrap();
                    let mut c = component.borrow_mut();
                    let mut window = Window::new(ui, "Rustycoat", 100, 100, WindowType::NoMenubar);
                    let ctrl = c.create_control(&component, ui.clone());
                    window.set_child(ui, ctrl);
                    c.start();
                    window.show(ui);
                },
                SyncComponentEntry::NonUI(c) => {
                    c.borrow_mut().start();
                },
            }
        }
    }

    pub fn tick(&mut self) {
        for component in self.sync_components.iter_mut() {
            match component {
                SyncComponentEntry::UI(c) => {
                    c.borrow_mut().tick();
                },
                SyncComponentEntry::NonUI(c) => {
                    c.borrow_mut().tick();
                },
            }
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for component in self.async_components.iter_mut() {
            if let AsyncComponentEntry::Running(handle) = mem::replace(component, AsyncComponentEntry::None) {
                handle.join().ok();
            }
        }
        for component in self.sync_components.iter_mut() {
            match component {
                SyncComponentEntry::UI(c) => {
                    c.borrow_mut().stop();
                },
                SyncComponentEntry::NonUI(c) => {
                    c.borrow_mut().stop();
                },
            };
        }
    }
}

impl Default for Computer {
    fn default() -> Computer {
        Computer::new()
    }
}
