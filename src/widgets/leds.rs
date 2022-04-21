use crate::core::ports::InputPin;
use crate::core::SyncComponent;

pub struct Led {
    input: InputPin,
}

impl Led {
    pub fn new() -> Self {
        Self { input: InputPin::new() }
    }

    pub fn input(&mut self) -> &mut InputPin {
        &mut self.input
    }
}

impl Default for Led {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncComponent for Led {
    fn requires_ui(&self) -> bool {
        false
    }

    fn start(&mut self, _ui: Option<&iui::UI>) {}

    fn tick(&mut self, _ui: Option<&iui::UI>) {
        if self.input.try_recv().is_some() {
            println!("LED update to {}", self.input.value());
        }
    }

    fn stop(&mut self, _ui: Option<&iui::UI>) {}
}
