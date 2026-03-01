use ftui::render::frame::Frame;

use super::model::AppModel;

pub(super) fn view(model: &AppModel, frame: &mut Frame) {
    model.render_model(frame);
}
