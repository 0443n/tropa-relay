use crate::autostart;
use crate::config::{AppConfig, ProxyEntry};
use crate::relay;
use iced::widget::{
    button, checkbox, column, container, row, rule, scrollable, space, text, text_input,
};
use iced::{Border, Color, Element, Length, Shadow, Size, Task, Theme, Vector};
use std::collections::HashMap;
use tokio::sync::watch;
use tokio::task::JoinHandle;

// ── Colors ──────────────────────────────────────────────────────
const ACCENT: Color = Color::from_rgb(0.235, 0.514, 0.969);
const ACCENT_HOVER: Color = Color::from_rgb(0.30, 0.58, 1.0);
const ACCENT_PRESS: Color = Color::from_rgb(0.18, 0.44, 0.85);
const DANGER: Color = Color::from_rgb(0.898, 0.224, 0.278);
const DANGER_HOVER: Color = Color::from_rgb(1.0, 0.30, 0.35);
const DANGER_PRESS: Color = Color::from_rgb(0.75, 0.15, 0.20);
const SURFACE: Color = Color::from_rgb(0.16, 0.16, 0.16);
const SURFACE_LIGHT: Color = Color::from_rgb(0.22, 0.22, 0.22);
const SURFACE_LIGHTER: Color = Color::from_rgb(0.28, 0.28, 0.28);
const TEXT_DIM: Color = Color::from_rgb(0.55, 0.55, 0.55);
const TEXT_MUTED: Color = Color::from_rgb(0.70, 0.70, 0.70);
const BORDER_SUBTLE: Color = Color::from_rgb(0.25, 0.25, 0.25);

// ── Relay handle ────────────────────────────────────────────────
struct RelayHandle {
    shutdown_tx: watch::Sender<bool>,
    _task: JoinHandle<()>,
}

// ── Views ───────────────────────────────────────────────────────
#[derive(Debug, Clone)]
enum View {
    List,
    EditForm,
}

// ── Draft proxy (form state) ────────────────────────────────────
#[derive(Debug, Clone)]
struct DraftProxy {
    name: String,
    remote_host: String,
    remote_port: String,
    username: String,
    password: String,
    local_port: String,
    enabled: bool,
}

impl Default for DraftProxy {
    fn default() -> Self {
        Self {
            name: String::new(),
            remote_host: String::new(),
            remote_port: "1080".into(),
            username: String::new(),
            password: String::new(),
            local_port: String::new(),
            enabled: true,
        }
    }
}

// ── App state ───────────────────────────────────────────────────
struct State {
    rt: tokio::runtime::Runtime,
    config: AppConfig,
    relays: HashMap<usize, RelayHandle>,
    view: View,
    editing_index: Option<usize>,
    draft: DraftProxy,
    edit_error: String,
    confirm_delete: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        let mut config = AppConfig::load();

        // Sync autostart config with actual filesystem/registry state
        let autostart_actual = autostart::is_enabled();
        if config.autostart != autostart_actual {
            config.autostart = autostart_actual;
            let _ = config.save();
        }

        let mut state = Self {
            rt,
            config,
            relays: HashMap::new(),
            view: View::List,
            editing_index: None,
            draft: DraftProxy::default(),
            edit_error: String::new(),
            confirm_delete: None,
        };

        let enabled: Vec<usize> = state
            .config
            .proxies
            .iter()
            .enumerate()
            .filter(|(_, p)| p.enabled)
            .map(|(i, _)| i)
            .collect();
        for i in enabled {
            state.start_relay(i);
        }

        state
    }
}

// ── Messages ────────────────────────────────────────────────────
#[derive(Debug, Clone)]
enum Message {
    OpenAddForm,
    OpenEditForm(usize),
    GoBack,
    NameChanged(String),
    RemoteHostChanged(String),
    RemotePortChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    LocalPortChanged(String),
    EnabledToggled(bool),
    SaveProxy,
    ToggleProxy(usize),
    RequestDelete(usize),
    ConfirmDelete(usize),
    CancelDelete,
    ToggleAutostart(bool),
}

// ── Relay management ────────────────────────────────────────────
impl State {
    fn start_relay(&mut self, index: usize) {
        if self.relays.contains_key(&index) {
            return;
        }
        if let Some(entry) = self.config.proxies.get(index) {
            let entry = entry.clone();
            let (tx, rx) = watch::channel(false);
            let task = self.rt.spawn(async move {
                relay::run_relay(entry, rx).await;
            });
            self.relays.insert(
                index,
                RelayHandle {
                    shutdown_tx: tx,
                    _task: task,
                },
            );
        }
    }

    fn stop_relay(&mut self, index: usize) {
        if let Some(handle) = self.relays.remove(&index) {
            let _ = handle.shutdown_tx.send(true);
        }
    }

    fn remove_proxy(&mut self, index: usize) {
        self.stop_relay(index);
        self.config.proxies.remove(index);
        let mut new_relays = HashMap::new();
        for (i, handle) in self.relays.drain() {
            if i > index {
                new_relays.insert(i - 1, handle);
            } else {
                new_relays.insert(i, handle);
            }
        }
        self.relays = new_relays;
        let _ = self.config.save();
    }

    fn save_draft(&mut self) {
        let remote_port: u16 = match self.draft.remote_port.parse() {
            Ok(p) => p,
            Err(_) => {
                self.edit_error = "Invalid remote port".into();
                return;
            }
        };
        let local_port: u16 = match self.draft.local_port.parse() {
            Ok(p) => p,
            Err(_) => {
                self.edit_error = "Invalid local port".into();
                return;
            }
        };
        if self.draft.name.trim().is_empty() {
            self.edit_error = "Name is required".into();
            return;
        }
        if self.draft.remote_host.trim().is_empty() {
            self.edit_error = "Remote host is required".into();
            return;
        }

        let entry = ProxyEntry {
            name: self.draft.name.trim().to_string(),
            remote_host: self.draft.remote_host.trim().to_string(),
            remote_port,
            username: self.draft.username.clone(),
            password: self.draft.password.clone(),
            local_port,
            enabled: self.draft.enabled,
        };

        match self.editing_index {
            Some(i) => {
                let was_running = self.relays.contains_key(&i);
                if was_running {
                    self.stop_relay(i);
                }
                self.config.proxies[i] = entry;
                if self.config.proxies[i].enabled {
                    self.start_relay(i);
                }
            }
            None => {
                self.config.proxies.push(entry);
                let i = self.config.proxies.len() - 1;
                if self.config.proxies[i].enabled {
                    self.start_relay(i);
                }
            }
        }

        let _ = self.config.save();
        self.view = View::List;
    }
}

// ── Update ──────────────────────────────────────────────────────
impl State {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenAddForm => {
                self.editing_index = None;
                self.draft = DraftProxy::default();
                self.edit_error.clear();
                self.view = View::EditForm;
            }
            Message::OpenEditForm(index) => {
                if let Some(proxy) = self.config.proxies.get(index) {
                    self.editing_index = Some(index);
                    self.draft = DraftProxy {
                        name: proxy.name.clone(),
                        remote_host: proxy.remote_host.clone(),
                        remote_port: proxy.remote_port.to_string(),
                        username: proxy.username.clone(),
                        password: proxy.password.clone(),
                        local_port: proxy.local_port.to_string(),
                        enabled: proxy.enabled,
                    };
                    self.edit_error.clear();
                    self.view = View::EditForm;
                }
            }
            Message::GoBack => {
                self.view = View::List;
            }
            Message::NameChanged(val) => self.draft.name = val,
            Message::RemoteHostChanged(val) => self.draft.remote_host = val,
            Message::RemotePortChanged(val) => self.draft.remote_port = val,
            Message::UsernameChanged(val) => self.draft.username = val,
            Message::PasswordChanged(val) => self.draft.password = val,
            Message::LocalPortChanged(val) => self.draft.local_port = val,
            Message::EnabledToggled(val) => self.draft.enabled = val,
            Message::SaveProxy => {
                self.save_draft();
            }
            Message::ToggleProxy(index) => {
                if index < self.config.proxies.len() {
                    self.config.proxies[index].enabled = !self.config.proxies[index].enabled;
                    if self.config.proxies[index].enabled {
                        self.start_relay(index);
                    } else {
                        self.stop_relay(index);
                    }
                    let _ = self.config.save();
                }
            }
            Message::RequestDelete(index) => {
                self.confirm_delete = Some(index);
            }
            Message::ConfirmDelete(index) => {
                self.remove_proxy(index);
                self.confirm_delete = None;
            }
            Message::CancelDelete => {
                self.confirm_delete = None;
            }
            Message::ToggleAutostart(enabled) => {
                let result = if enabled {
                    autostart::enable()
                } else {
                    autostart::disable()
                };
                match result {
                    Ok(()) => {
                        self.config.autostart = enabled;
                        let _ = self.config.save();
                    }
                    Err(e) => {
                        eprintln!("autostart error: {e}");
                    }
                }
            }
        }
        Task::none()
    }
}

// ── View ────────────────────────────────────────────────────────
impl State {
    fn view(&self) -> Element<'_, Message> {
        match &self.view {
            View::List => self.view_list(),
            View::EditForm => self.view_edit_form(),
        }
    }

    fn view_list(&self) -> Element<'_, Message> {
        let header = row![
            text("Tropa Relay").size(22),
            space::horizontal(),
            button(text("+ Add Proxy").size(14))
                .on_press(Message::OpenAddForm)
                .padding([6, 16])
                .style(accent_button_style),
        ]
        .align_y(iced::Alignment::Center);

        let content: Element<'_, Message> = if self.config.proxies.is_empty() {
            container(
                column![
                    text("No proxies configured").size(18).color(TEXT_DIM),
                    text("Click \"+ Add Proxy\" to get started.")
                        .size(14)
                        .color(TEXT_DIM),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            let cards: Vec<Element<'_, Message>> = self
                .config
                .proxies
                .iter()
                .enumerate()
                .map(|(i, proxy)| self.view_proxy_card(i, proxy))
                .collect();

            scrollable(column(cards).spacing(8).width(Length::Fill)).into()
        };

        let autostart_toggle = row![
            checkbox(self.config.autostart)
                .on_toggle(Message::ToggleAutostart)
                .style(checkbox_style),
            text("Start on login").size(13),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        container(
            column![header, rule::horizontal(1), content, autostart_toggle]
                .spacing(8)
                .padding(16),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_proxy_card<'a>(
        &'a self,
        index: usize,
        proxy: &'a ProxyEntry,
    ) -> Element<'a, Message> {
        if self.confirm_delete == Some(index) {
            let confirm_row = row![
                text(format!("Delete \"{}\"?", proxy.name)).size(15),
                space::horizontal(),
                button(text("Delete").size(13))
                    .on_press(Message::ConfirmDelete(index))
                    .padding([6, 16])
                    .style(danger_button_style),
                button(text("Cancel").size(13))
                    .on_press(Message::CancelDelete)
                    .padding([6, 16])
                    .style(neutral_button_style),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            return container(confirm_row)
                .padding(16)
                .width(Length::Fill)
                .style(card_container_style)
                .into();
        }

        let toggle_label = if proxy.enabled { "ON" } else { "OFF" };
        let toggle_style: fn(&Theme, button::Status) -> button::Style = if proxy.enabled {
            on_button_style
        } else {
            off_button_style
        };

        let name_text = text(&proxy.name).size(16).font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::DEFAULT
        });

        let row1 = row![
            name_text,
            space::horizontal(),
            button(text(toggle_label).size(12))
                .on_press(Message::ToggleProxy(index))
                .padding([4, 12])
                .style(toggle_style),
        ]
        .align_y(iced::Alignment::Center);

        let subtitle = text(format!(
            "{}:{} \u{2192} local {}",
            proxy.remote_host, proxy.remote_port, proxy.local_port
        ))
        .size(13)
        .color(TEXT_DIM);

        let row3 = row![
            space::horizontal(),
            button(text("Edit").size(13).color(TEXT_MUTED))
                .on_press(Message::OpenEditForm(index))
                .padding([4, 8])
                .style(ghost_button_style),
            button(text("Delete").size(13).color(DANGER))
                .on_press(Message::RequestDelete(index))
                .padding([4, 8])
                .style(ghost_button_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        container(
            column![row1, subtitle, row3]
                .spacing(4)
                .width(Length::Fill),
        )
        .padding(16)
        .width(Length::Fill)
        .style(card_container_style)
        .into()
    }

    fn view_edit_form(&self) -> Element<'_, Message> {
        let title = if self.editing_index.is_some() {
            "Edit Proxy"
        } else {
            "Add Proxy"
        };

        let header = row![
            text(title).size(22),
            space::horizontal(),
            button(text("Back").size(14))
                .on_press(Message::GoBack)
                .padding([6, 16])
                .style(neutral_button_style),
        ]
        .align_y(iced::Alignment::Center);

        let form = column![
            form_field("Name", &self.draft.name, Message::NameChanged),
            form_field(
                "Remote host",
                &self.draft.remote_host,
                Message::RemoteHostChanged
            ),
            form_field(
                "Remote port",
                &self.draft.remote_port,
                Message::RemotePortChanged
            ),
            form_field("Username", &self.draft.username, Message::UsernameChanged),
            form_field("Password", &self.draft.password, Message::PasswordChanged),
            form_field(
                "Local port",
                &self.draft.local_port,
                Message::LocalPortChanged
            ),
            row![
                text("Enabled").width(100),
                checkbox(self.draft.enabled)
                    .on_toggle(Message::EnabledToggled)
                    .style(checkbox_style),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(10);

        let mut content = column![header, rule::horizontal(1)].spacing(8);

        if !self.edit_error.is_empty() {
            content = content.push(
                text(&self.edit_error)
                    .size(14)
                    .color(Color::from_rgb(1.0, 0.3, 0.3)),
            );
        }

        content = content.push(form);

        let buttons = row![
            button(text("Save").size(14))
                .on_press(Message::SaveProxy)
                .padding([6, 16])
                .style(accent_button_style),
            button(text("Cancel").size(14))
                .on_press(Message::GoBack)
                .padding([6, 16])
                .style(neutral_button_style),
        ]
        .spacing(8);

        content = content.push(buttons);

        container(content.padding(16))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl Drop for State {
    fn drop(&mut self) {
        for (_, handle) in self.relays.drain() {
            let _ = handle.shutdown_tx.send(true);
        }
    }
}

// ── Helper: form field row ──────────────────────────────────────
fn form_field<'a>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).width(100),
        text_input("", value)
            .on_input(on_input)
            .style(input_style),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .into()
}

// ── Style: square border helper ─────────────────────────────────
const SQUARE: Border = Border {
    color: Color::TRANSPARENT,
    width: 0.0,
    radius: iced::border::Radius {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    },
};

fn square_border_with(color: Color, width: f32) -> Border {
    Border {
        color,
        width,
        ..SQUARE
    }
}

// ── Style: buttons ──────────────────────────────────────────────
fn accent_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(ACCENT.into()),
        text_color: Color::WHITE,
        border: SQUARE,
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(ACCENT_HOVER.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(ACCENT_PRESS.into()),
            ..base
        },
        _ => base,
    }
}

fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(DANGER.into()),
        text_color: Color::WHITE,
        border: SQUARE,
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(DANGER_HOVER.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(DANGER_PRESS.into()),
            ..base
        },
        _ => base,
    }
}

fn neutral_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(SURFACE_LIGHT.into()),
        text_color: Color::WHITE,
        border: square_border_with(BORDER_SUBTLE, 1.0),
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(SURFACE_LIGHTER.into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(SURFACE.into()),
            ..base
        },
        _ => base,
    }
}

fn on_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(ACCENT.into()),
        text_color: Color::WHITE,
        border: SQUARE,
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(ACCENT_HOVER.into()),
            ..base
        },
        _ => base,
    }
}

fn off_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(SURFACE_LIGHT.into()),
        text_color: TEXT_MUTED,
        border: square_border_with(BORDER_SUBTLE, 1.0),
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(SURFACE_LIGHTER.into()),
            ..base
        },
        _ => base,
    }
}

fn ghost_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(SURFACE_LIGHT.into()),
            text_color: Color::WHITE,
            border: SQUARE,
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: None,
            text_color: TEXT_MUTED,
            border: SQUARE,
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

// ── Style: card container ───────────────────────────────────────
fn card_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: square_border_with(BORDER_SUBTLE, 1.0),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 4.0,
        },
        text_color: None,
        snap: false,
    }
}

// ── Style: text input ───────────────────────────────────────────
fn input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::Style {
        background: SURFACE.into(),
        border: square_border_with(BORDER_SUBTLE, 1.0),
        icon: TEXT_DIM,
        placeholder: TEXT_DIM,
        value: Color::WHITE,
        selection: ACCENT,
    };
    match status {
        text_input::Status::Focused { .. } => text_input::Style {
            border: square_border_with(ACCENT, 1.0),
            ..base
        },
        text_input::Status::Hovered => text_input::Style {
            border: square_border_with(TEXT_MUTED, 1.0),
            ..base
        },
        text_input::Status::Disabled => text_input::Style {
            value: TEXT_DIM,
            ..base
        },
        _ => base,
    }
}

// ── Style: checkbox ─────────────────────────────────────────────
fn checkbox_style(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    match status {
        checkbox::Status::Active { is_checked: true }
        | checkbox::Status::Disabled { is_checked: true } => checkbox::Style {
            background: ACCENT.into(),
            icon_color: Color::WHITE,
            border: SQUARE,
            text_color: None,
        },
        checkbox::Status::Hovered { is_checked: true } => checkbox::Style {
            background: ACCENT_HOVER.into(),
            icon_color: Color::WHITE,
            border: SQUARE,
            text_color: None,
        },
        checkbox::Status::Hovered { is_checked: false } => checkbox::Style {
            background: SURFACE_LIGHTER.into(),
            icon_color: Color::WHITE,
            border: square_border_with(TEXT_MUTED, 1.0),
            text_color: None,
        },
        _ => checkbox::Style {
            background: SURFACE_LIGHT.into(),
            icon_color: Color::WHITE,
            border: square_border_with(BORDER_SUBTLE, 1.0),
            text_color: None,
        },
    }
}

// ── Entry point ─────────────────────────────────────────────────
pub fn run_gui() -> iced::Result {
    iced::application(|| State::default(), State::update, State::view)
        .title("Tropa Relay")
        .window_size(Size::new(550.0, 400.0))
        .resizable(false)
        .theme(State::theme)
        .run()
}
