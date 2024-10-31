use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{constants, BBImagerMessage};

pub fn view(bbimager: &crate::BBImager) -> Element<BBImagerMessage> {
    let items = bbimager
        .destinations
        .iter()
        .filter(|x| {
            x.to_string()
                .to_lowercase()
                .contains(&bbimager.search_bar.to_lowercase())
        })
        .map(|x| {
            let mut row2 = widget::column![text(x.to_string())];

            if let bb_imager::Destination::SdCard { size, .. } = x {
                let s = (*size as f32) / (1024.0 * 1024.0 * 1024.0);
                row2 = row2.push(text(format!("{:.2} GB", s)));
            }

            button(
                widget::row![
                    widget::svg(widget::svg::Handle::from_memory(constants::USB_ICON)).width(40),
                    row2
                ]
                .align_y(iced::Alignment::Center)
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::SelectPort(x.clone()))
            .style(widget::button::secondary)
        })
        .map(Into::into);

    widget::column![
        bbimager.search_bar(Some(BBImagerMessage::RefreshDestinations)),
        widget::horizontal_rule(2),
        widget::scrollable(widget::column(items).spacing(10))
    ]
    .spacing(10)
    .padding(10)
    .into()
}
