use iced::widget::{button, horizontal_space, row};
use iced::{Element, Length, Renderer};

use crate::core::settings_config::SETTINGS;

#[derive(Clone, Debug)]
pub enum Message {
    Save,
    Reset,
    RestoreDefault,
}

#[derive(Default)]
pub struct SettingsControls;

impl SettingsControls {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Save => SETTINGS.write().unwrap().save_settings(),
            Message::Reset => SETTINGS.write().unwrap().reset_settings(),
            Message::RestoreDefault => SETTINGS.write().unwrap().set_default_settings(),
        }
    }
    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let mut save_settings_button = button("Save Settings");
        let mut reset_settings_button = button("Reset Settings");
        let mut restore_default_settings_button = button("Restore Default Settings");

        if SETTINGS.read().unwrap().has_pending_save() {
            save_settings_button = save_settings_button.on_press(Message::Save);
            reset_settings_button = reset_settings_button.on_press(Message::Reset);
        }

        if !SETTINGS.read().unwrap().has_default_settings() {
            restore_default_settings_button =
                restore_default_settings_button.on_press(Message::RestoreDefault);
        }

        row![
            horizontal_space(Length::Fill),
            restore_default_settings_button,
            reset_settings_button,
            save_settings_button
        ]
        .spacing(10)
        .padding(5)
        .into()
    }
}
