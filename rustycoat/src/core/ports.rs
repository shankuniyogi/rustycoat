use crossbeam_channel::{unbounded, Receiver, Select, Sender};

pub struct OutputPort<T>
where
    T: Send + Default + Copy,
{
    value: T,
    sender: Option<Sender<T>>,
}

impl<T> Default for OutputPort<T>
where
    T: Send + Default + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OutputPort<T>
where
    T: Send + Default + Copy,
{
    pub fn new() -> Self {
        Self::with_initial_value(T::default())
    }

    pub fn with_initial_value(initial_value: T) -> Self {
        Self { value: initial_value, sender: None }
    }

    pub fn connect_to(&mut self, target: &mut InputPort<T>) {
        let (s, r): (Sender<T>, Receiver<T>) = unbounded();
        if self.sender.is_some() {
            panic!("Output port already connected");
        }
        self.sender = Some(s);
        target.receiver = Some(r);
    }

    pub fn update(&mut self, new_value: T) {
        self.value = new_value;
        if let Some(s) = self.sender.as_mut() {
            s.send(new_value).ok();
        }
    }

    pub fn value(&self) -> T {
        self.value
    }
}

pub type OutputPin = OutputPort<bool>;
pub type OutputPort8 = OutputPort<u8>;
pub type OutputPort16 = OutputPort<u16>;

pub struct InputPort<T>
where
    T: Send + Default + Copy,
{
    value: T,
    receiver: Option<Receiver<T>>,
}

impl<T> Default for InputPort<T>
where
    T: Send + Default + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InputPort<T>
where
    T: Send + Default + Copy,
{
    pub fn new() -> Self {
        Self::with_initial_value(T::default())
    }

    pub fn with_initial_value(initial_value: T) -> Self {
        Self { value: initial_value, receiver: None }
    }

    pub fn wait(&mut self) -> T {
        if let Some(r) = self.receiver.as_mut() {
            if let Ok(new_value) = r.recv() {
                self.value = new_value;
            }
            self.value
        } else {
            panic!("Input port not connected");
        }
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn wait_any(ports: &mut [&mut Self]) -> Option<usize> {
        let mut select = Select::new();
        for port in ports.iter() {
            if let Some(r) = &port.receiver {
                select.recv(r);
            }
        }
        let s = select.select();
        let mut idx = s.index();
        for (i, _) in ports.iter().enumerate() {
            if let Some(r) = &ports[i].receiver {
                if idx == 0 {
                    if let Ok(val) = s.recv(r) {
                        ports[i].value = val;
                        return Some(i);
                    } else {
                        break;
                    }
                } else {
                    idx -= 1;
                }
            }
        }

        None
    }
}

pub type InputPin = InputPort<bool>;
pub type InputPort8 = InputPort<u8>;
pub type InputPort16 = InputPort<u16>;
