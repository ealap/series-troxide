use super::{
    api::tv_maze::{episodes_information::Episode, series_information::SeriesMainInformation},
    caching::series_list,
    settings_config,
};
use anyhow::Context;
use chrono::Duration;
use directories::ProjectDirs;
use notify::{recommended_watcher, EventHandler, Watcher};
use std::sync::mpsc;
use tokio::task::JoinHandle;

enum Signal {
    SettingsFileChanged,
    NotificationSent,
}

pub struct TroxideNotify {
    signal_receiver: mpsc::Receiver<Signal>,
    signal_sender: mpsc::Sender<Signal>,
}

impl TroxideNotify {
    pub fn new() -> anyhow::Result<Self> {
        let (signal_sender, signal_receiver) = mpsc::channel();

        let file_change_signal_sender = signal_sender.clone();
        std::thread::spawn(move || Self::file_change_watcher(file_change_signal_sender));

        Ok(Self {
            signal_receiver,
            signal_sender,
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        tokio::runtime::Runtime::new()?.block_on(async {
            let mut current_notification_time_setting = get_current_notification_time_setting();

            loop {
                // This is the time before the actual release of an episode that should be used by the notification
                // to send notifications before the actual release of an episode.
                let duration_before_release =
                    Duration::minutes(current_notification_time_setting as i64);

                // Creating a handle for each episode release notification so that we can be able to abort them at anytime
                // we want.
                let notification_handles: Vec<_> = get_releases_with_duration_to_release()
                    .await
                    .into_iter()
                    .map(|(series_info, episode, duration)| {
                        (series_info, episode, duration - duration_before_release)
                    })
                    .filter(|(_, _, duration)| duration.to_std().is_ok())
                    .map(|(series_info, episode, duration)| {
                        let signal_sender = self.signal_sender.clone();
                        tokio::spawn(async move {
                            tracing::info!(
                                "waiting {} minutes for \"{}'s\" notification",
                                duration.num_minutes(),
                                series_info.name,
                            );
                            tokio::time::sleep(duration.to_std().unwrap()).await;
                            notify_episode_release(
                                &series_info,
                                &episode,
                                current_notification_time_setting,
                            );
                            signal_sender.send(Signal::NotificationSent).unwrap();
                        })
                    })
                    .collect();

                match &self.signal_receiver.recv().unwrap() {
                    Signal::SettingsFileChanged => {
                        /*
                        Since the settings file can change the time to notify before the actual release, our notifications will
                        be waiting to notify with a delay that is no longer correct, so be obtain the current settings from the
                        settings file and abort all the upcoming notifications and reobtain all of them in the next loop iteration
                        TODO: Make it detect only when the nofification settings changed
                        */
                        tracing::info!("config file change detected, refreshing notifications");
                        current_notification_time_setting = get_current_notification_time_setting();

                        Self::abort_notifications(notification_handles);
                    }
                    Signal::NotificationSent => {
                        /*
                        When a new episode has been notified, when can't keep on using the same obtained episode releases as it might
                        turn out that that series is being released regulary(weekly) and thus the currently obtained releases won't
                        have that information. So we just abort all the handles to reobtain all the releases information in the next
                        iteration of the loop.
                        */
                        tracing::info!(
                            "episode release notification sent, refreshing notifications"
                        );

                        Self::abort_notifications(notification_handles);
                    }
                }
            }
        });
        Ok(())
    }

    fn abort_notifications(notification_handles: Vec<JoinHandle<()>>) {
        notification_handles
            .into_iter()
            .for_each(|handle| handle.abort())
    }

    fn file_change_watcher(signal_sender: mpsc::Sender<Signal>) {
        let file_watcher_event_handler = FileWatcherEventHandler::new(signal_sender);
        let mut settings_file_watcher = recommended_watcher(file_watcher_event_handler)
            .context("failed to create settings file watcher")
            .unwrap();

        if let Some(proj_dirs) = ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            let config_directory = std::path::PathBuf::from(proj_dirs.config_dir());
            let mut config_file = config_directory.clone();
            config_file.push(super::settings_config::CONFIG_FILE_NAME);

            settings_file_watcher
                .watch(&config_file, notify::RecursiveMode::NonRecursive)
                .unwrap();
            std::thread::park();
        }
    }
}

async fn get_releases_with_duration_to_release() -> Vec<(SeriesMainInformation, Episode, Duration)>
{
    series_list::SeriesList::new()
        .get_upcoming_release_series_informations_and_episodes()
        .await
        .context("failed to get upcoming series releases")
        .unwrap()
        .into_iter()
        .map(|(series_info, next_episode, release_time)| {
            (
                series_info,
                next_episode,
                release_time.get_remaining_release_duration(),
            )
        })
        .collect()
}

fn notify_episode_release(
    series_info: &SeriesMainInformation,
    episode: &Episode,
    release_time_in_minute: u32,
) {
    let series_name = series_info.name.as_str();
    let episode_name = episode.name.as_str();
    let episode_order = crate::gui::helpers::season_episode_str_gen(
        episode.season,
        episode
            .number
            .expect("an episode should have a valid number"),
    );

    let notification_summary = format!("\"{}\" episode release", series_name);

    let notification_body = format!(
        "{}: {}, will be released in {} minutes",
        episode_order, episode_name, release_time_in_minute
    );

    notify_rust::Notification::new()
        .appname("Series Troxide")
        .summary(&notification_summary)
        .body(&notification_body)
        .timeout(0)
        .auto_icon()
        .show()
        .expect("failed to show notification");
}

struct FileWatcherEventHandler {
    sender: mpsc::Sender<Signal>,
}

fn get_current_notification_time_setting() -> u32 {
    settings_config::Settings::new()
        .get_current_settings()
        .notifications
        .time_to_notify
}

impl FileWatcherEventHandler {
    fn new(sender: mpsc::Sender<Signal>) -> Self {
        Self { sender }
    }
}

impl EventHandler for FileWatcherEventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let event = event.unwrap();

        if let notify::EventKind::Remove(_) = event.kind {
            self.sender.send(Signal::SettingsFileChanged).unwrap();
        };
        if let notify::EventKind::Modify(_) = event.kind {
            self.sender.send(Signal::SettingsFileChanged).unwrap();
        };
    }
}
