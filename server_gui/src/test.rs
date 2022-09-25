use eframe::{App, Frame};
use egui::Context;

#[derive(Default)]
pub struct Test {}

impl App for Test {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.request_repaint()
    }
}