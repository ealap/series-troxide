use iced::{widget::container, Element, Length, Renderer};

use crate::gui::{Message as GuiMessage, Tab};

#[derive(Clone, Debug)]
pub enum Message {}

#[derive(Default)]
pub struct StatisticsTab;

impl StatisticsTab {
    pub fn view(&self) -> Element<Message, Renderer> {
        container("Statistics View")
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Tab for StatisticsTab {
    type Message = GuiMessage;

    fn title(&self) -> String {
        "Statistics".to_owned()
    }

    fn tab_label(&self) -> iced_aw::TabLabel {
        iced_aw::TabLabel::Text("Statistics icon".to_owned())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        self.view().map(GuiMessage::Statistics)
    }
}
