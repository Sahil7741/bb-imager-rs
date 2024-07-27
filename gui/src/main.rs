use std::path::PathBuf;

use bb_imager::FlashingStatus;
use futures_util::TryStreamExt;
use iced::{executor, futures::Stream, Application, Command, Element, Settings};

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    let settings = Settings {
        window: iced::window::Settings {
            size: iced::Size::new(800.0, 500.0),
            icon: iced::window::icon::from_file("icons/bb-imager.png").ok(),
            ..Default::default()
        },
        flags: bb_imager::config::Config::from_json(include_bytes!("../../config.json"))
            .expect("Failed to parse config"),
        ..Default::default()
    };

    BBImager::run(settings)
}

#[derive(Default, Debug)]
struct BBImager {
    config: bb_imager::config::Config,
    downloader: bb_imager::download::Downloader,
    screen: Screen,
    selected_board: Option<bb_imager::config::Device>,
    selected_image: Option<OsImage>,
    selected_dst: Option<String>,
    download_status: Option<Result<bb_imager::DownloadStatus, String>>,
    flashing_status: Option<Result<bb_imager::FlashingStatus, String>>,
    search_bar: String,
}

#[derive(Debug, Clone)]
enum OsImage {
    Local(PathBuf),
    Remote(bb_imager::config::OsList),
}

impl std::fmt::Display for OsImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsImage::Local(p) => write!(f, "{}", p.file_name().unwrap().to_string_lossy()),
            OsImage::Remote(r) => write!(f, "{}", r.name),
        }
    }
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    BoardSelected(bb_imager::config::Device),
    SelectImage(Option<bb_imager::config::OsList>),
    SelectPort(String),
    StartFlashing,
    FlashImage {
        path: PathBuf,
        inner_path: Option<String>,
        sha256: Option<[u8; 32]>,
    },

    DownloadStatus(Result<bb_imager::DownloadStatus, String>),
    FlashingStatus(Result<bb_imager::FlashingStatus, String>),
    Reset,

    BoardSectionPage,
    ImageSelectionPage,
    DestinationSelectionPage,
    HomePage,

    Search(String),
    BoardImageDownloaded {
        index: usize,
        path: PathBuf,
    },
    BoardImageDownloadFailed {
        index: usize,
        error: String,
    },

    OsListImageDownloaded {
        index: usize,
        path: PathBuf,
    },
    OsListDownloadFailed {
        index: usize,
        error: String,
    },
    Null,
}

impl Application for BBImager {
    type Message = BBImagerMessage;
    type Executor = executor::Default;
    type Flags = bb_imager::config::Config;
    type Theme = iced::theme::Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let downloader = bb_imager::download::Downloader::default();

        let board_image_from_cache = flags.devices().iter().enumerate().map(|(index, v)| {
            Command::perform(
                downloader
                    .clone()
                    .check_cache(v.icon.clone(), v.icon_sha256),
                move |p| match p {
                    Some(path) => BBImagerMessage::BoardImageDownloaded { index, path },
                    None => BBImagerMessage::Null,
                },
            )
        });

        let os_image_from_cache = flags.os_list.iter().enumerate().map(|(index, v)| {
            Command::perform(
                downloader
                    .clone()
                    .check_cache(v.icon.clone(), v.icon_sha256),
                move |p| match p {
                    Some(path) => BBImagerMessage::OsListImageDownloaded { index, path },
                    None => BBImagerMessage::Null,
                },
            )
        });

        (
            Self {
                config: flags.clone(),
                downloader: downloader.clone(),
                ..Default::default()
            },
            Command::batch(board_image_from_cache.chain(os_image_from_cache)),
        )
    }

    fn title(&self) -> String {
        String::from("BeagleBoard Imager")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            BBImagerMessage::BoardSelected(x) => {
                self.selected_board = Some(x);
                self.back_home();
                Command::none()
            }
            BBImagerMessage::SelectImage(x) => {
                self.selected_image = match x {
                    Some(y) => Some(OsImage::Remote(y)),
                    None => rfd::FileDialog::new()
                        .add_filter("firmware", &["bin"])
                        .pick_file()
                        .map(OsImage::Local),
                };
                self.back_home();
                Command::none()
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
                self.back_home();
                Command::none()
            }
            BBImagerMessage::FlashImage {
                path,
                inner_path,
                sha256,
            } => {
                let board = self.selected_board.clone().expect("No board selected");
                let dst = self.selected_dst.clone().expect("No destination selected");

                tracing::info!("Start flashing image {:?}", path);
                let stream =
                    Command::run(flash_helper(path, inner_path, sha256, board, dst), |x| {
                        BBImagerMessage::FlashingStatus(x.map_err(|e| e.to_string()))
                    });

                Command::batch([
                    Command::perform(std::future::ready(FlashingStatus::Preparing), |x| {
                        BBImagerMessage::FlashingStatus(Ok(x))
                    }),
                    stream,
                ])
            }
            BBImagerMessage::FlashingStatus(x) => {
                self.flashing_status = Some(x.map_err(|e| e.to_string()));
                Command::none()
            }
            BBImagerMessage::Reset => {
                self.selected_dst = None;
                self.selected_image = None;
                self.selected_board = None;
                self.search_bar.clear();
                Command::none()
            }
            BBImagerMessage::HomePage => {
                self.back_home();
                Command::none()
            }
            BBImagerMessage::BoardSectionPage => {
                self.screen = Screen::BoardSelection;
                let jobs = self
                    .config
                    .devices()
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.icon_local.is_none())
                    .map(|(index, v)| {
                        Command::perform(
                            self.downloader
                                .clone()
                                .download(v.icon.clone(), v.icon_sha256),
                            move |p| match p {
                                Ok(path) => BBImagerMessage::BoardImageDownloaded { index, path },
                                Err(e) => BBImagerMessage::BoardImageDownloadFailed {
                                    index,
                                    error: e.to_string(),
                                },
                            },
                        )
                    });
                Command::batch(jobs)
            }
            BBImagerMessage::ImageSelectionPage => {
                self.screen = Screen::ImageSelection;
                let board = self.selected_board.as_ref().unwrap().name.clone();
                let jobs = self
                    .config
                    .os_list
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.icon_local.is_none())
                    .filter(|(_, v)| v.devices.contains(&board))
                    .map(|(index, v)| {
                        Command::perform(
                            self.downloader
                                .clone()
                                .download(v.icon.clone(), v.icon_sha256),
                            move |p| match p {
                                Ok(path) => BBImagerMessage::OsListImageDownloaded { index, path },
                                Err(e) => BBImagerMessage::OsListDownloadFailed {
                                    index,
                                    error: e.to_string(),
                                },
                            },
                        )
                    });

                Command::batch(jobs)
            }
            BBImagerMessage::DestinationSelectionPage => {
                self.screen = Screen::DestinationSelection;
                Command::none()
            }

            BBImagerMessage::Search(x) => {
                self.search_bar = x;
                Command::none()
            }
            BBImagerMessage::BoardImageDownloaded { index, path } => {
                tracing::info!("Successfully downloaded to {:?}", path);
                self.config.imager.devices[index].icon_local = Some(path);
                Command::none()
            }
            BBImagerMessage::BoardImageDownloadFailed { index, error } => {
                tracing::warn!(
                    "Failed to fetch icon for {:?}, Error: {error}",
                    self.config.imager.devices[index]
                );
                Command::none()
            }
            BBImagerMessage::OsListImageDownloaded { index, path } => {
                tracing::info!(
                    "Successfully downloaded os icon for {:?} to {:?}",
                    self.config.os_list[index],
                    path
                );
                self.config.os_list[index].icon_local = Some(path);
                Command::none()
            }
            BBImagerMessage::OsListDownloadFailed { index, error } => {
                tracing::warn!(
                    "Failed to fetch icon for {:?}, Error: {error}",
                    self.config.imager.devices[index]
                );
                Command::none()
            }
            BBImagerMessage::StartFlashing => match self.selected_image.clone().unwrap() {
                OsImage::Local(p) => {
                    Command::perform(std::future::ready((p, None)), |(path, inner_path)| {
                        BBImagerMessage::FlashImage {
                            path,
                            inner_path,
                            sha256: None,
                        }
                    })
                }
                OsImage::Remote(r) => {
                    tracing::info!("Downloading Remote Os");
                    Command::run(
                        self.downloader.download_progress(r.url, r.download_sha256),
                        |x| BBImagerMessage::DownloadStatus(x.map_err(|y| y.to_string())),
                    )
                }
            },
            BBImagerMessage::DownloadStatus(s) => {
                if let Ok(bb_imager::DownloadStatus::Finished(p)) = s {
                    tracing::info!("Os download finished");
                    self.download_status.take();
                    if let Some(OsImage::Remote(x)) = &self.selected_image {
                        let sha256 = x.extracted_sha256;
                        Command::perform(
                            std::future::ready((p, x.extract_path.clone())),
                            move |(path, inner_path)| BBImagerMessage::FlashImage {
                                path,
                                inner_path,
                                sha256: Some(sha256),
                            },
                        )
                    } else {
                        unreachable!()
                    }
                } else {
                    tracing::debug!("Os download progress: {:?}", s);
                    self.download_status = Some(s);
                    Command::none()
                }
            }
            BBImagerMessage::Null => Command::none(),
        }
    }

    fn view(&self) -> Element<Self::Message> {
        match self.screen {
            Screen::Home => self.home_view(),
            Screen::BoardSelection => self.board_selction_view(),
            Screen::ImageSelection => self.image_selection_view(),
            Screen::DestinationSelection => self.destination_selection_view(),
        }
    }

    fn theme(&self) -> Self::Theme {
        iced::Theme::KanagawaLotus
    }
}

impl BBImager {
    fn back_home(&mut self) {
        self.search_bar.clear();
        self.screen = Screen::Home;
    }

    fn home_view(&self) -> Element<BBImagerMessage> {
        const BTN_PADDING: u16 = 10;

        let logo = iced::widget::image("icons/logo_sxs_imager.png").width(500);

        let choose_device_btn = iced::widget::button(
            self.selected_board
                .as_ref()
                .map_or(iced::widget::text("CHOOSE DEVICE"), |x| {
                    iced::widget::text(x.name.as_str())
                }),
        )
        .on_press(BBImagerMessage::BoardSectionPage)
        .padding(BTN_PADDING);

        let choose_image_btn = iced::widget::button(
            self.selected_image
                .as_ref()
                .map_or(iced::widget::text("CHOOSE IMAGE"), |x| {
                    iced::widget::text(x)
                }),
        )
        .on_press_maybe(
            self.selected_board
                .as_ref()
                .map(|_| BBImagerMessage::ImageSelectionPage),
        )
        .padding(BTN_PADDING);

        let choose_dst_btn = iced::widget::button(
            self.selected_dst
                .as_ref()
                .map_or(iced::widget::text("CHOOSE DESTINATION"), iced::widget::text),
        )
        .on_press_maybe(
            self.selected_image
                .as_ref()
                .map(|_| BBImagerMessage::DestinationSelectionPage),
        )
        .padding(BTN_PADDING);

        let reset_btn = iced::widget::button("RESET")
            .on_press(BBImagerMessage::Reset)
            .padding(BTN_PADDING);
        let write_btn = if self.selected_board.is_some()
            && self.selected_image.is_some()
            && self.selected_dst.is_some()
        {
            iced::widget::button("WRITE").on_press(BBImagerMessage::StartFlashing)
        } else {
            iced::widget::button("WRITE")
        }
        .padding(BTN_PADDING);

        let choice_btn_row = iced::widget::row![
            choose_device_btn,
            iced::widget::horizontal_space(),
            choose_image_btn,
            iced::widget::horizontal_space(),
            choose_dst_btn
        ]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_items(iced::Alignment::Center);

        let action_btn_row =
            iced::widget::row![reset_btn, iced::widget::horizontal_space(), write_btn]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_items(iced::Alignment::Center);

        let (progress_label, progress_bar) = self.progress();

        iced::widget::column![
            logo,
            choice_btn_row,
            action_btn_row,
            progress_label,
            progress_bar
        ]
        .spacing(5)
        .padding(64)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_items(iced::Alignment::Center)
        .into()
    }

    fn board_selction_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .config
            .devices()
            .iter()
            .filter(|x| {
                x.name
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let image: Element<BBImagerMessage> = match &x.icon_local {
                    Some(y) => iced::widget::image(iced::widget::image::Handle::from_memory(
                        std::fs::read(y).unwrap(),
                    ))
                    .width(100)
                    .height(100)
                    .into(),
                    None => iced::widget::svg("icons/downloading.svg").width(40).into(),
                };

                iced::widget::button(
                    iced::widget::row![
                        image,
                        iced::widget::column![
                            iced::widget::text(x.name.as_str()).size(18),
                            iced::widget::horizontal_space(),
                            iced::widget::text(x.description.as_str())
                        ]
                        .padding(5)
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::BoardSelected(x.clone()))
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        let items = iced::widget::scrollable(iced::widget::column(items).spacing(10));

        iced::widget::column![self.search_bar(), iced::widget::horizontal_rule(2), items]
            .spacing(10)
            .padding(10)
            .into()
    }

    fn image_selection_view(&self) -> Element<BBImagerMessage> {
        let board = self.selected_board.as_ref().unwrap();
        let items = self
            .config
            .images_by_device(&board)
            .filter(|x| {
                x.name
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let mut row3 = iced::widget::row![
                    iced::widget::text(x.release_date),
                    iced::widget::horizontal_space(),
                ]
                .width(iced::Length::Fill);

                row3 = x.tags.iter().fold(row3, |acc, t| {
                    acc.push(iced_aw::Badge::new(iced::widget::text(t)))
                });

                iced::widget::button(
                    iced::widget::row![
                        iced::widget::svg(
                            x.icon_local
                                .clone()
                                .unwrap_or(PathBuf::from("icons/downloading.svg"))
                        )
                        .width(80),
                        iced::widget::column![
                            iced::widget::text(x.name.as_str()).size(18),
                            iced::widget::text(x.description.as_str()),
                            row3
                        ]
                        .padding(5)
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage(Some(x.clone())))
                .style(iced::widget::theme::Button::Secondary)
            })
            .chain(std::iter::once(
                iced::widget::button(
                    iced::widget::row![
                        iced::widget::svg("icons/file-add.svg").width(100),
                        iced::widget::text("Use Custom Image").size(18),
                    ]
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage(None))
                .style(iced::widget::theme::Button::Secondary),
            ))
            .map(Into::into);

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            iced::widget::scrollable(iced::widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn destination_selection_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .selected_board
            .as_ref()
            .expect("No Board Selected")
            .flasher
            .destinations()
            .unwrap()
            .into_iter()
            .filter(|x| x.to_lowercase().contains(&self.search_bar.to_lowercase()))
            .map(|x| {
                iced::widget::button(
                    iced::widget::row![
                        iced::widget::svg("icons/usb.svg").width(40),
                        iced::widget::text(x.as_str()),
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectPort(x))
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            iced::widget::scrollable(iced::widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn search_bar(&self) -> Element<BBImagerMessage> {
        iced::widget::row![
            iced::widget::button(iced::widget::svg("icons/arrow-back.svg").width(22))
                .on_press(BBImagerMessage::HomePage)
                .style(iced::widget::theme::Button::Secondary),
            iced::widget::text_input("Search", &self.search_bar).on_input(BBImagerMessage::Search)
        ]
        .spacing(10)
        .into()
    }

    fn progress(&self) -> (iced::widget::Text, iced::widget::ProgressBar) {
        if let Some(s) = &self.download_status {
            match s {
                Ok(x) => match x {
                    bb_imager::DownloadStatus::DownloadingProgress(p) => (
                        iced::widget::text(format!(
                            "Downloading... {}%",
                            (*p * 100.0).round() as usize
                        )),
                        iced::widget::progress_bar((0.0)..=1.0, *p),
                    ),
                    bb_imager::DownloadStatus::Finished(_) => (
                        iced::widget::text("Downloading Successful..."),
                        iced::widget::progress_bar((0.0)..=1.0, 1.0)
                            .style(iced::widget::theme::ProgressBar::Success),
                    ),
                    bb_imager::DownloadStatus::VerifyingProgress(p) => (
                        iced::widget::text(format!(
                            "Verifying... {}%",
                            (*p * 100.0).round() as usize
                        )),
                        iced::widget::progress_bar((0.0)..=1.0, *p),
                    ),
                },
                Err(e) => (
                    iced::widget::text(format!("Downloading Image Failed: {e}")),
                    iced::widget::progress_bar((0.0)..=1.0, 1.0)
                        .style(iced::widget::theme::ProgressBar::Danger),
                ),
            }
        } else if let Some(s) = &self.flashing_status {
            match s {
                Ok(x) => match x {
                    bb_imager::FlashingStatus::Preparing => (
                        iced::widget::text("Preparing..."),
                        iced::widget::progress_bar((0.0)..=1.0, 0.5),
                    ),
                    bb_imager::FlashingStatus::Flashing => (
                        iced::widget::text("Flashing..."),
                        iced::widget::progress_bar((0.0)..=1.0, 0.5),
                    ),
                    bb_imager::FlashingStatus::FlashingProgress(p) => (
                        iced::widget::text(format!(
                            "Flashing... {}%",
                            (*p * 100.0).round() as usize
                        )),
                        iced::widget::progress_bar((0.0)..=1.0, *p),
                    ),
                    bb_imager::FlashingStatus::Verifying => (
                        iced::widget::text("Verifying..."),
                        iced::widget::progress_bar((0.0)..=1.0, 0.5),
                    ),
                    bb_imager::FlashingStatus::VerifyingProgress(p) => (
                        iced::widget::text(format!(
                            "Verifying... {}%",
                            (*p * 100.0).round() as usize
                        )),
                        iced::widget::progress_bar((0.0)..=1.0, *p),
                    ),
                    bb_imager::FlashingStatus::Finished => (
                        iced::widget::text("Flashing Successful..."),
                        iced::widget::progress_bar((0.0)..=1.0, 1.0)
                            .style(iced::widget::theme::ProgressBar::Success),
                    ),
                },
                Err(e) => (
                    iced::widget::text(format!("Flashing Failed: {e}")),
                    iced::widget::progress_bar((0.0)..=1.0, 1.0)
                        .style(iced::widget::theme::ProgressBar::Danger),
                ),
            }
        } else {
            (
                iced::widget::text(""),
                iced::widget::progress_bar((0.0)..=1.0, 0.0),
            )
        }
    }
}

#[derive(Default, Debug)]
enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
}

fn flash_helper(
    path: std::path::PathBuf,
    inner_path: Option<String>,
    sha256: Option<[u8; 32]>,
    board: bb_imager::config::Device,
    dst: String,
) -> impl Stream<Item = Result<bb_imager::FlashingStatus, String>> {
    futures_util::stream::once(async move {
        bb_imager::img::OsImage::from_path(&path, inner_path.as_deref(), sha256)
            .await
            .map(|x| board.flasher.flash(x, dst))
    })
    .try_flatten()
    .map_err(|x| x.to_string())
}
