use std::sync::mpsc;

use crate::core::api::tv_maze::series_information::SeriesMainInformation;
use crate::gui::assets::icons::FILM;
use crate::gui::styles;

use iced::widget::scrollable::{RelativeOffset, Viewport};
use iced::widget::{column, scrollable, text};
use iced::{Command, Element, Length, Renderer};

use my_shows_widget::{Message as MyShowsMessage, MyShows};
use upcoming_releases_widget::{Message as UpcomingReleasesMessage, UpcomingReleases};

use super::Tab;

mod my_shows_widget;
mod upcoming_releases_widget;

#[derive(Debug, Clone)]
pub enum Message {
    Ended(MyShowsMessage),
    Waiting(MyShowsMessage),
    Upcoming(UpcomingReleasesMessage),
    Untracked(MyShowsMessage),
    PageScrolled(Viewport),
}

pub struct MyShowsTab {
    waiting_releases: MyShows,
    upcoming_releases: UpcomingReleases,
    ended_releases: MyShows,
    untracked_releases: MyShows,
    scrollable_offset: RelativeOffset,
}

impl MyShowsTab {
    pub fn new(
        series_page_sender: mpsc::Sender<SeriesMainInformation>,
        scrollable_offset: Option<RelativeOffset>,
    ) -> (Self, Command<Message>) {
        let (untracked_releases, untracked_releases_commands) =
            MyShows::new_as_untracked_series(series_page_sender.clone());
        let (ended_releases, ended_releases_commands) =
            MyShows::new_as_ended_tracked_series(series_page_sender.clone());
        let (upcoming_releases, upcoming_releases_commands) =
            UpcomingReleases::new(series_page_sender.clone());
        let (waiting_releases, waiting_releases_commands) =
            MyShows::new_as_waiting_release_series(series_page_sender);

        (
            Self {
                ended_releases,
                untracked_releases,
                waiting_releases,
                upcoming_releases,
                scrollable_offset: scrollable_offset.unwrap_or(RelativeOffset::START),
            },
            Command::batch([
                untracked_releases_commands.map(Message::Untracked),
                ended_releases_commands.map(Message::Ended),
                waiting_releases_commands.map(Message::Waiting),
                upcoming_releases_commands.map(Message::Upcoming),
            ]),
        )
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        self.upcoming_releases.subscription().map(Message::Upcoming)
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Ended(message) => self.ended_releases.update(message).map(Message::Ended),
            Message::Waiting(message) => {
                self.waiting_releases.update(message).map(Message::Waiting)
            }
            Message::Upcoming(message) => self
                .upcoming_releases
                .update(message)
                .map(Message::Upcoming),
            Message::Untracked(message) => self
                .untracked_releases
                .update(message)
                .map(Message::Untracked),
            Message::PageScrolled(view_port) => {
                self.scrollable_offset = view_port.relative_offset();
                Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message, Renderer> {
        let upcoming_releases = self.upcoming_releases.view().map(Message::Upcoming);

        let waiting_releases: Element<'_, Message, Renderer> = column![
            text("Waiting for release date")
                .size(21)
                .style(styles::text_styles::green_text_theme()),
            self.waiting_releases.view().map(Message::Waiting)
        ]
        .spacing(5)
        .into();

        let ended_releases: Element<'_, Message, Renderer> = column![
            text("Ended")
                .size(21)
                .style(styles::text_styles::red_text_theme()),
            self.ended_releases.view().map(Message::Ended)
        ]
        .spacing(5)
        .into();

        let untracked_releases: Element<'_, Message, Renderer> = column![
            text("Untracked").size(21),
            self.untracked_releases.view().map(Message::Untracked)
        ]
        .spacing(5)
        .into();

        scrollable(
            column![
                upcoming_releases,
                waiting_releases,
                ended_releases,
                untracked_releases,
            ]
            .padding(10)
            .spacing(50)
            .width(Length::Fill)
            .align_items(iced::Alignment::Start),
        )
        .direction(styles::scrollable_styles::vertical_direction())
        .id(Self::scrollable_id())
        .on_scroll(Message::PageScrolled)
        .into()
    }
}

impl Tab for MyShowsTab {
    type Message = Message;

    fn title() -> &'static str {
        "My Shows"
    }

    fn icon_bytes() -> &'static [u8] {
        FILM
    }

    fn get_scrollable_offset(&self) -> scrollable::RelativeOffset {
        self.scrollable_offset
    }
}
