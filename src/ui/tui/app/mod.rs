use ftui::Cmd;
use ftui::render::frame::Frame;

use super::{GroveApp, Msg};

pub(super) type AppModel = GroveApp;
pub(super) type AppMsg = Msg;

pub(super) fn update(model: &mut AppModel, msg: AppMsg) -> Cmd<AppMsg> {
    model.update_model(msg)
}

pub(super) fn view(model: &AppModel, frame: &mut Frame) {
    model.render_model(frame);
}
