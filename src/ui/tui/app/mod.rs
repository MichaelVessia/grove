mod model;
mod msg;
mod update;
mod view;

use model::AppModel;
use msg::AppMsg;

pub(super) fn update(model: &mut AppModel, msg: AppMsg) -> ftui::Cmd<AppMsg> {
    update::update(model, msg)
}

pub(super) fn view(model: &AppModel, frame: &mut ftui::render::frame::Frame) {
    view::view(model, frame);
}
