use std::sync::mpsc::{self, Receiver, Sender};

pub enum Port<T>
where
    T: Send + Default + Copy,
{
    Output { value: T, s: Sender<T> },
    Input { value: T, r: Receiver<T> },
    None(T),
}

impl<T> Default for Port<T>
where
    T: Send + Default + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Port<T>
where
    T: Send + Default + Copy,
{
    pub fn new() -> Self {
        Self::None(T::default())
    }

    pub fn with_initial_value(initial_value: T) -> Self {
        Self::None(initial_value)
    }

    pub fn connect_to(&mut self, target: &mut Self) {
        let (s, r): (Sender<T>, Receiver<T>) = mpsc::channel();
        let value = if let Self::None(initial_value) = self {
            *initial_value
        } else {
            panic!("Pin already connected");
        };
        *self = Self::Output { value, s };
        *target = Self::Input { value, r };
    }

    pub fn update(&mut self, new_value: T) {
        match self {
            Self::Output { value, s } => {
                *value = new_value;
                s.send(new_value).unwrap();
            },
            Self::Input { .. } => panic!("Attempt to send to an input port"),
            _ => (),
        }
    }

    pub fn wait(&mut self) -> T {
        match self {
            Self::Input { r, value } => {
                if let Ok(new_value) = r.recv() {
                    *value = new_value;
                }
                *value
            },
            _ => panic!("Attempt to receive from a non-input port"),
        }
    }

    pub fn value(&self) -> T {
        match self {
            Self::Output { value, .. } => *value,
            Self::Input { value, .. } => *value,
            Self::None(value) => *value,
        }
    }
}

pub type Pin = Port<bool>;
pub type Port8 = Port<u8>;
pub type Port16 = Port<u16>;
