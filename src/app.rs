// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{self, Padding};
use cosmic::iced::{Subscription, window::Id};
use cosmic::prelude::*;
use cosmic::widget::mouse_area;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[derive(Default)]
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<Id>,
    /// Configuration data that persists between application runs.
    config: Config,
    price: String,
    is_random_wallpaper: bool,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    PopupClosed(Id),
    UpdateConfig(Config),
    UpdatePrice(String),
    FetchPrice,
    SetRandomWallpaper,
    ToggleRandomWallpaper,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.luthfynur.te-pgd-applet";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Construct the app model with the runtime's core.
        let app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| {
                    Config::get_entry(&context).unwrap_or_else(|(_errors, config)| {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    })
                })
                .unwrap_or_default(),
            price: "Memuat harga tabungan emas...".to_string(),
            is_random_wallpaper: false,
            ..Default::default()
        };
        (
            app,
            Task::perform(get_price(), |res| {
                cosmic::Action::App(Message::UpdatePrice(res))
            }),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-lived async tasks running in the background which
    /// emit messages to the application through a channel. They may be conditionally
    /// activated by selectively appending to the subscription batch, and will
    /// continue to execute for the duration that they remain in the batch.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
            iced::time::every(Duration::from_secs(300)).map(|_| Message::FetchPrice),
            iced::time::every(Duration::from_secs(1)).map(|_| Message::SetRandomWallpaper),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime. The application will not exit until all
    /// tasks are finished.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::FetchPrice => {
                self.price = "Memuat harga...".into();
                return Task::perform(get_price(), |res| {
                    cosmic::Action::App(Message::UpdatePrice(res))
                });
            }
            Message::UpdatePrice(price) => self.price = price,
            Message::SetRandomWallpaper => {
                if self.is_random_wallpaper {
                    let result = std::process::Command::new("waypaper")
                        .arg("--random")
                        .output();
                    match result {
                        Ok(_) => {}
                        Err(err) => println!("{}", err),
                    }
                }
            }
            Message::ToggleRandomWallpaper => {
                self.is_random_wallpaper = !self.is_random_wallpaper;
            }
        }
        Task::none()
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// The applet's button in the panel will be drawn using the main view method.
    /// This view should emit messages to toggle the applet's popup window, which will
    /// be drawn using the `view_window` method.
    fn view(&self) -> Element<'_, Self::Message> {
        let text = self.core.applet.text(&self.price).size(15.0);
        let container: cosmic::widget::Container<Message, Theme> = cosmic::widget::container(text)
            .padding(Padding {
                top: 1.0,
                bottom: 1.0,
                left: 5.0,
                right: 5.0,
            });
        let mouse_area = mouse_area(container)
            .on_double_press(Message::ToggleRandomWallpaper)
            .on_press(Message::FetchPrice);
        self.core.applet.autosize_window(mouse_area).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PgdResponse {
    data: PgdResponseData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PgdResponseData {
    pub harga_beli: String,
    pub harga_jual: String,
    pub tgl_berlaku: String,
}

async fn get_price() -> String {
    let client = Client::new();
    let response = client
        .get("https://sahabat.pegadaian.co.id/gold/prices/savings")
        .send()
        .await;

    if let Ok(res) = response {
        if let Ok(json) = res.json::<PgdResponse>().await {
            let harga_beli = format_rupiah(&json.data.harga_beli);
            let harga_jual = format_rupiah(&json.data.harga_jual);

            return format!(
                "Beli: {}, Jual: {} ({})",
                harga_beli, harga_jual, json.data.tgl_berlaku
            );
        }
    }

    "Gagal memuat harga tabungan emas".to_string()
}

fn format_rupiah(s: &str) -> String {
    let num: i64 = match s.parse() {
        Ok(n) => n,
        Err(_) => return format!("Rp. {}", s), // fallback
    };

    let chars: Vec<char> = num.to_string().chars().collect();
    let mut result = String::new();

    for (i, &c) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }

    format!("Rp. {}", result.chars().rev().collect::<String>())
}
