use iui::controls::*;
use iui::UI;

use crate::core::ports::InputPin;
use crate::core::{SyncComponent, UiComponent};

pub struct Led {
    input: InputPin,
    ui: Option<UI>,
    label: Option<Label>,
}

impl Led {
    pub fn new() -> Self {
        Self {
            input: InputPin::new(),
            ui: None,
            label: None,
        }
    }

    pub fn input(&mut self) -> &mut InputPin {
        &mut self.input
    }

    fn update(&mut self) {
        self.label
            .as_mut()
            .unwrap()
            .set_text(self.ui.as_ref().unwrap(), if self.input.value() { "ON" } else { "OFF" });
    }
}

impl Default for Led {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncComponent for Led {
    fn start(&mut self) {
        self.update();
    }

    fn tick(&mut self) {
        if self.input.try_recv().is_some() {
            self.update();
        }
    }

    fn stop(&mut self) {}
}

impl UiComponent for Led {
    fn ui_new(&mut self, ui: iui::UI) -> Control {
        let label = Label::new(&ui, "LED");
        self.label = Some(label.clone());
        self.ui = Some(ui);
        label.into()
    }
}
