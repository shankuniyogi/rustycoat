use std::cell::RefCell;
use std::rc::Rc;

use iui::controls::*;
use iui::draw::*;
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
    fn create_control(&mut self, self_ref: &Rc<RefCell<dyn UiComponent>>, ui: iui::UI) -> Control {
        let label = Label::new(&ui, "LED");
        self.label = Some(label.clone());
        self.ui = Some(ui);
        label.into()
    }
}

struct Draw {
    component: Rc<RefCell<Led>>,
}

impl AreaHandler for Draw {
    fn draw(&mut self, _: &Area, draw_params: &AreaDrawParams) {
        let ctx = &draw_params.context;
        let path = Path::new(ctx, FillMode::Winding);
        path.add_rectangle(ctx, 0.0, 0.0, draw_params.area_width, draw_params.area_height);
        path.end(ctx);
        let brush = if self.component.borrow().input.value() {
            Brush::Solid(SolidBrush { r: 1.0, g: 0.0, b: 0.0, a: 1.0 })
        } else {
            Brush::Solid(SolidBrush { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
        };
        ctx.fill(&path, &brush);
    }
}
