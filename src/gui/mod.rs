use std::sync::mpsc;

use view::discover_view::{DiscoverTab, Message as DiscoverMessage};
use view::my_shows_view::{Message as MyShowsMessage, MyShowsTab};
use view::series_view::Message as SeriesMessage;
use view::series_view::Series;
use view::settings_view::{Message as SettingsMessage, SettingsTab};
use view::statistics_view::{Message as StatisticsMessage, StatisticsTab};
use view::watchlist_view::{Message as WatchlistMessage, WatchlistTab};

use iced::widget::{container, text, Column};
use iced::{Application, Command, Element, Length};

use super::core::settings_config;
use crate::core::settings_config::SETTINGS;

pub mod assets;
pub mod helpers;
mod styles;
mod troxide_widget;
mod view;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabId {
    Discover,
    Watchlist,
    MyShows,
    Statistics,
    Settings,
}

impl From<usize> for TabId {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Discover,
            1 => Self::Watchlist,
            2 => Self::MyShows,
            3 => Self::Statistics,
            4 => Self::Settings,
            _ => unreachable!("no more tabs"),
        }
    }
}

impl From<TabId> for usize {
    fn from(val: TabId) -> Self {
        match val {
            TabId::Discover => 0,
            TabId::Watchlist => 1,
            TabId::MyShows => 2,
            TabId::Statistics => 3,
            TabId::Settings => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(usize),
    Discover(DiscoverMessage),
    Watchlist(WatchlistMessage),
    MyShows(MyShowsMessage),
    Statistics(StatisticsMessage),
    Settings(SettingsMessage),
    Series(SeriesMessage),
    FontLoaded(Result<(), iced::font::Error>),
}

pub struct TroxideGui {
    active_tab: TabId,
    series_view_active: bool,
    discover_tab: DiscoverTab,
    watchlist_tab: WatchlistTab,
    my_shows_tab: MyShowsTab,
    statistics_tab: StatisticsTab,
    settings_tab: SettingsTab,
    series_view: Option<Series>,
    // TODO: to use iced::subscription
    series_page_sender: mpsc::Sender<(Series, Command<SeriesMessage>)>,
    series_page_receiver: mpsc::Receiver<(Series, Command<SeriesMessage>)>,
}

impl Application for TroxideGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let font_command = iced::font::load(assets::get_static_cow_from_asset(
            assets::fonts::NOTOSANS_REGULAR_STATIC,
        ));
        let (sender, receiver) = mpsc::channel();

        let (discover_tab, discover_command) =
            view::discover_view::DiscoverTab::new(sender.clone());
        let (my_shows_tab, my_shows_command) = MyShowsTab::new(sender.clone());
        let (watchlist_tab, watchlist_command) = WatchlistTab::new(sender.clone());

        (
            Self {
                active_tab: TabId::Discover,
                series_view_active: false,
                discover_tab,
                watchlist_tab,
                statistics_tab: StatisticsTab::default(),
                my_shows_tab,
                settings_tab: view::settings_view::SettingsTab::new(),
                series_view: None,
                series_page_sender: sender,
                series_page_receiver: receiver,
            },
            Command::batch([
                font_command.map(Message::FontLoaded),
                discover_command.map(Message::Discover),
                my_shows_command.map(Message::MyShows),
                watchlist_command.map(Message::Watchlist),
            ]),
        )
    }

    fn title(&self) -> String {
        "Series Troxide".to_string()
    }

    fn theme(&self) -> iced::Theme {
        match SETTINGS
            .read()
            .unwrap()
            .get_current_settings()
            .appearance
            .theme
        {
            settings_config::Theme::Light => {
                let theme = styles::theme::TroxideTheme::Light;
                iced::Theme::Custom(Box::new(theme.get_theme()))
            }
            settings_config::Theme::Dark => {
                let theme = styles::theme::TroxideTheme::Dark;
                iced::Theme::Custom(Box::new(theme.get_theme()))
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        self.discover_tab.subscription().map(Message::Discover)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TabSelected(tab_id) => {
                self.series_view_active = false;
                let tab_id: TabId = tab_id.into();
                self.active_tab = tab_id.clone();

                if let TabId::Discover = tab_id {
                    return self.discover_tab.refresh().map(Message::Discover);
                }
                if let TabId::MyShows = tab_id {
                    let (my_shows_tab, my_shows_message) =
                        MyShowsTab::new(self.series_page_sender.clone());
                    self.my_shows_tab = my_shows_tab;
                    return my_shows_message.map(Message::MyShows);
                };
                if let TabId::Watchlist = tab_id {
                    let (watchlist_tab, watchlist_message) =
                        WatchlistTab::new(self.series_page_sender.clone());
                    self.watchlist_tab = watchlist_tab;
                    return watchlist_message.map(Message::Watchlist);
                };
                if let TabId::Statistics = tab_id {
                    return self.statistics_tab.refresh().map(Message::Statistics);
                };
                Command::none()
            }
            Message::Discover(message) => Command::batch([
                self.discover_tab.update(message).map(Message::Discover),
                self.try_series_page_switch(),
            ]),
            Message::Watchlist(message) => Command::batch([
                self.watchlist_tab.update(message).map(Message::Watchlist),
                self.try_series_page_switch(),
            ]),
            Message::MyShows(message) => Command::batch([
                self.my_shows_tab.update(message).map(Message::MyShows),
                self.try_series_page_switch(),
            ]),
            Message::Statistics(message) => {
                self.statistics_tab.update(message).map(Message::Statistics)
            }
            Message::Settings(message) => self.settings_tab.update(message).map(Message::Settings),
            Message::Series(message) => self
                .series_view
                .as_mut()
                .expect("for series view to send a message it must exist")
                .update(message)
                .map(Message::Series),
            Message::FontLoaded(res) => {
                if res.is_err() {
                    tracing::error!("failed to load font");
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Message, iced::Renderer<Self::Theme>> {
        let mut tabs: Vec<(
            troxide_widget::tabs::TabLabel,
            Element<'_, Message, iced::Renderer>,
        )> = vec![
            (
                self.discover_tab.tab_label(),
                self.discover_tab.view().map(Message::Discover),
            ),
            (
                self.watchlist_tab.tab_label(),
                self.watchlist_tab.view().map(Message::Watchlist),
            ),
            (
                self.my_shows_tab.tab_label(),
                self.my_shows_tab.view().map(Message::MyShows),
            ),
            (
                self.statistics_tab.tab_label(),
                self.statistics_tab.view().map(Message::Statistics),
            ),
            (
                self.settings_tab.tab_label(),
                self.settings_tab.view().map(Message::Settings),
            ),
        ];

        let active_tab_index: usize = self.active_tab.to_owned().into();

        // Hijacking the current tab view when series view is active
        if self.series_view_active {
            let (_, current_view): &mut (
                troxide_widget::tabs::TabLabel,
                Element<'_, Message, iced::Renderer>,
            ) = &mut tabs[active_tab_index];
            *current_view = self
                .series_view
                .as_ref()
                .unwrap()
                .view()
                .map(Message::Series);
        }

        troxide_widget::tabs::Tabs::with_tabs(tabs, Message::TabSelected)
            .set_active_tab(active_tab_index)
            .view()
    }
}

impl TroxideGui {
    fn try_series_page_switch(&mut self) -> Command<Message> {
        match self.series_page_receiver.try_recv() {
            Ok((series_page, series_page_command)) => {
                self.series_view = Some(series_page);
                self.series_view_active = true;
                series_page_command.map(Message::Series)
            }
            Err(err) => match err {
                mpsc::TryRecvError::Empty => Command::none(),
                mpsc::TryRecvError::Disconnected => panic!("series page senders disconnected"),
            },
        }
    }
}

trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> troxide_widget::tabs::TabLabel;

    fn view(&self) -> Element<'_, Self::Message> {
        let column = Column::new()
            .spacing(20)
            .push(text(self.title()).size(32))
            .push(self.content());

        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn content(&self) -> Element<'_, Self::Message>;
}
