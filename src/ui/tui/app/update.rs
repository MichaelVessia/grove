use ftui::Cmd;

use super::model::AppModel;
use super::msg::AppMsg;

pub(super) fn update(model: &mut AppModel, msg: AppMsg) -> Cmd<AppMsg> {
    model.update_model(msg)
}
