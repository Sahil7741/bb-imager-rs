use bb_config::config;
use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{BBImagerMessage, constants, pages};

use super::helpers::search_bar;

pub(crate) fn view<'a>(
    devices: impl Iterator<Item = (usize, &'a config::Device)>,
    search_str: &'a str,
    downloader: &'a bb_downloader::Downloader,
) -> Element<'a, BBImagerMessage> {
    let items = devices
        .filter(|(_, x)| x.name.to_lowercase().contains(&search_str.to_lowercase()))
        .map(|(id, dev)| {
            let image: Element<BBImagerMessage> = match &dev.icon {
                Some(url) => match downloader.clone().check_cache_from_url(url.clone()) {
                    Some(y) => img_or_svg(y, 100),
                    None => widget::svg(widget::svg::Handle::from_memory(
                        constants::DOWNLOADING_ICON,
                    ))
                    .width(100)
                    .into(),
                },
                None => widget::svg(widget::svg::Handle::from_memory(constants::BOARD_ICON))
                    .width(100)
                    .height(60)
                    .into(),
            };

            button(
                widget::row![
                    image,
                    widget::column![
                        text(&dev.name).size(18),
                        widget::horizontal_space(),
                        text(dev.description.as_str())
                    ]
                    .padding(5)
                ]
                .align_y(iced::Alignment::Center)
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::SelectBoard(id))
            .style(widget::button::secondary)
        })
        .map(Into::into);

    widget::column![
        search_bar(search_str, |x| BBImagerMessage::ReplaceScreen(
            pages::Screen::BoardSelection(pages::SearchState::new(x))
        )),
        widget::horizontal_rule(2),
        widget::scrollable(widget::column(items).spacing(10))
    ]
    .spacing(10)
    .padding(10)
    .into()
}

pub(crate) fn img_or_svg<'a>(path: std::path::PathBuf, width: u16) -> Element<'a, BBImagerMessage> {
    let img = std::fs::read(path).expect("Failed to open image");

    match image::guess_format(&img) {
        Ok(_) => widget::image(widget::image::Handle::from_bytes(img))
            .width(width)
            .height(width)
            .into(),

        Err(_) => widget::svg(widget::svg::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),
    }
}
