use iced::{
    Element,
    widget::{self, button},
};

use crate::{
    BBImagerMessage, constants, state::FlashingFinishState, ui::helpers::{CircleBar, board_view_pane, page_type1}
};

pub(crate) fn view(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    page_type1(
        &state.common,
        info_view(state),
        progress_view(state),
        [button("Restart")
            .style(widget::button::primary)
            .on_press(BBImagerMessage::Restart)],
    )
}

pub(crate) fn progress_view(state: &FlashingFinishState) -> Element<'static, BBImagerMessage> {
    let msg = if state.is_download {
        "Successfully Downloaded Image"
    } else {
        "Successfully Flashed Image"
    };

    widget::column![
        CircleBar::new("100%", 10.0, constants::CHECK_MARK_GREEN),
        msg
    ]
    .align_x(iced::Center)
    .padding(16)
    .into()
}

pub(crate) fn info_view(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    board_view_pane(state.selected_board(), &state.common)
}
