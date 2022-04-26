use std::cell::RefCell;
use std::rc::Rc;

use iui::controls::*;
use iui::draw::*;
use iui::UI;

use crate::core::ports::InputPin;
use crate::core::{SyncComponent, UiComponent};
use crate::widgets::Color;

pub struct Led {
    input: InputPin,
    ui: Option<UI>,
    area: Option<Area>,
    draw_state: Rc<RefCell<DrawState>>
}

impl Led {
    pub fn new(on_color: Color, off_color: Color) -> Self {
        Self {
            input: InputPin::new(),
            ui: None,
            area: None,
            draw_state: Rc::new(RefCell::new(DrawState { state: false, on_color, off_color }))
        }
    }

    pub fn input(&mut self) -> &mut InputPin {
        &mut self.input
    }

    fn update(&mut self) {
        self.draw_state.borrow_mut().state = self.input.value();
        self.area.as_ref().unwrap().queue_redraw_all(&self.ui.as_ref().unwrap());
    }
}

impl Default for Led {
    fn default() -> Self {
        Self::new(Color { r: 1.0, g: 0.0, b: 0.0 }, Color { r: 0.4, g: 0.4, b: 0.4 })
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
    fn create_control(&mut self, ui: iui::UI) -> Control {
        let area = Area::new(&ui, self.draw_state.clone());
        self.area = Some(area.clone());
        self.ui = Some(ui);
        area.into()
    }
}

struct DrawState {
    state: bool,
    on_color: Color,
    off_color: Color
}

impl AreaHandler for DrawState {
    fn draw(&mut self, _: &Area, draw_params: &AreaDrawParams) {
        let ctx = &draw_params.context;
        let path = Path::new(ctx, FillMode::Winding);
        let radius = f64::min(draw_params.area_width, draw_params.area_height) / 2.0;
        path.new_figure_with_arc(ctx, draw_params.area_width / 2.0, draw_params.area_height / 2.0,
            radius, 0.0, std::f64::consts::PI * 2.0, false);
        path.end(ctx);
        let brush = if self.state {
            Brush::Solid(SolidBrush::from(&self.on_color))
        } else {
            Brush::Solid(SolidBrush::from(&self.off_color))
        };
        ctx.fill(&path, &brush);
    }
}
