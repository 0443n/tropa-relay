use crate::config::{AppConfig, ProxyEntry};
use crate::relay;
use eframe::egui;
use std::cell::Cell;
use std::collections::HashMap;
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// Custom toggle switch widget (pill-shaped, like iOS/Android).
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = egui::vec2(36.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.15);
        let bg = egui::Color32::from_rgb(
            (160.0 + (76.0 - 160.0) * how_on) as u8,
            (160.0 + (175.0 - 160.0) * how_on) as u8,
            (160.0 + (80.0 - 160.0) * how_on) as u8,
        );
        let pill_r = rect.height() / 2.0;
        let knob_r = pill_r - 2.0;
        let knob_x = rect.left() + pill_r + (rect.width() - rect.height()) * how_on;
        ui.painter().rect_filled(rect, pill_r, bg);
        ui.painter().circle_filled(
            egui::pos2(knob_x, rect.center().y),
            knob_r,
            egui::Color32::WHITE,
        );
    }
    response
}

struct RelayHandle {
    shutdown_tx: watch::Sender<bool>,
    _task: JoinHandle<()>,
}

pub struct TropaApp {
    rt: tokio::runtime::Runtime,
    config: AppConfig,
    relays: HashMap<usize, RelayHandle>,
    // Edit dialog state
    show_edit_dialog: bool,
    editing_index: Option<usize>,
    draft_name: String,
    draft_remote_host: String,
    draft_remote_port: String,
    draft_username: String,
    draft_password: String,
    draft_local_port: String,
    draft_enabled: bool,
    edit_error: String,
    // Delete confirmation
    confirm_delete: Option<usize>,
    // Password visibility
    show_password: bool,
}

impl TropaApp {
    fn new() -> Self {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        let config = AppConfig::load();

        let mut app = Self {
            rt,
            config,
            relays: HashMap::new(),
            show_edit_dialog: false,
            editing_index: None,
            draft_name: String::new(),
            draft_remote_host: String::new(),
            draft_remote_port: String::new(),
            draft_username: String::new(),
            draft_password: String::new(),
            draft_local_port: String::new(),
            draft_enabled: true,
            edit_error: String::new(),
            confirm_delete: None,
            show_password: false,
        };

        // Start all enabled proxies on launch
        let enabled: Vec<usize> = app
            .config
            .proxies
            .iter()
            .enumerate()
            .filter(|(_, p)| p.enabled)
            .map(|(i, _)| i)
            .collect();
        for i in enabled {
            app.start_relay(i);
        }

        app
    }

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
        // Shift relay indices above the removed one
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

    fn open_add_dialog(&mut self) {
        self.editing_index = None;
        self.draft_name.clear();
        self.draft_remote_host.clear();
        self.draft_remote_port = "1080".into();
        self.draft_username.clear();
        self.draft_password.clear();
        self.draft_local_port.clear();
        self.draft_enabled = true;
        self.edit_error.clear();
        self.show_password = false;
        self.show_edit_dialog = true;
    }

    fn open_edit_dialog(&mut self, index: usize) {
        if let Some(proxy) = self.config.proxies.get(index) {
            self.editing_index = Some(index);
            self.draft_name = proxy.name.clone();
            self.draft_remote_host = proxy.remote_host.clone();
            self.draft_remote_port = proxy.remote_port.to_string();
            self.draft_username = proxy.username.clone();
            self.draft_password = proxy.password.clone();
            self.draft_local_port = proxy.local_port.to_string();
            self.draft_enabled = proxy.enabled;
            self.edit_error.clear();
            self.show_password = false;
            self.show_edit_dialog = true;
        }
    }

    fn save_draft(&mut self) {
        let remote_port: u16 = match self.draft_remote_port.parse() {
            Ok(p) => p,
            Err(_) => {
                self.edit_error = "Invalid remote port".into();
                return;
            }
        };
        let local_port: u16 = match self.draft_local_port.parse() {
            Ok(p) => p,
            Err(_) => {
                self.edit_error = "Invalid local port".into();
                return;
            }
        };
        if self.draft_name.trim().is_empty() {
            self.edit_error = "Name is required".into();
            return;
        }
        if self.draft_remote_host.trim().is_empty() {
            self.edit_error = "Remote host is required".into();
            return;
        }

        let entry = ProxyEntry {
            name: self.draft_name.trim().to_string(),
            remote_host: self.draft_remote_host.trim().to_string(),
            remote_port,
            username: self.draft_username.clone(),
            password: self.draft_password.clone(),
            local_port,
            enabled: self.draft_enabled,
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
        self.show_edit_dialog = false;
    }
}

impl eframe::App for TropaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Top bar
            ui.horizontal(|ui| {
                ui.heading("Tropa Relay");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("+ Add Proxy").size(16.0),
                            )
                            .min_size(egui::vec2(130.0, 36.0)),
                        )
                        .clicked()
                    {
                        self.open_add_dialog();
                    }
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            if self.config.proxies.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(
                        egui::RichText::new("No proxies configured")
                            .size(18.0)
                            .weak(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Click \"+ Add Proxy\" to get started.")
                            .size(14.0)
                            .weak(),
                    );
                });
            } else {
                // Snapshot proxy data for cards (avoids borrow conflicts)
                let proxies: Vec<(usize, String, String, u16, u16, bool)> = self
                    .config
                    .proxies
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        (
                            i,
                            p.name.clone(),
                            p.remote_host.clone(),
                            p.remote_port,
                            p.local_port,
                            p.enabled,
                        )
                    })
                    .collect();

                let toggle_action = Cell::new(None);
                let edit_action = Cell::new(None);
                let delete_action = Cell::new(None);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, name, host, remote_port, local_port, enabled) in &proxies {
                        let mut enabled_copy = *enabled;
                        let card_fill = if ui.visuals().dark_mode {
                            egui::Color32::from_gray(35)
                        } else {
                            egui::Color32::from_gray(245)
                        };

                        egui::Frame::new()
                            .fill(card_fill)
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(name).strong().size(16.0),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if toggle_switch(ui, &mut enabled_copy).changed() {
                                                toggle_action.set(Some(*i));
                                            }
                                        },
                                    );
                                });

                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}:{} \u{2192} local {}",
                                        host, remote_port, local_port
                                    ))
                                    .weak()
                                    .size(13.0),
                                );

                                ui.horizontal(|ui| {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(egui::Button::new("Delete").frame(false))
                                                .clicked()
                                            {
                                                delete_action.set(Some(*i));
                                            }
                                            if ui
                                                .add(egui::Button::new("Edit").frame(false))
                                                .clicked()
                                            {
                                                edit_action.set(Some(*i));
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(4.0);
                    }
                });

                // Apply deferred actions
                if let Some(i) = toggle_action.get() {
                    self.config.proxies[i].enabled = !self.config.proxies[i].enabled;
                    if self.config.proxies[i].enabled {
                        self.start_relay(i);
                    } else {
                        self.stop_relay(i);
                    }
                    let _ = self.config.save();
                }
                if let Some(i) = edit_action.get() {
                    self.open_edit_dialog(i);
                }
                if let Some(i) = delete_action.get() {
                    self.confirm_delete = Some(i);
                }
            }
        });

        // Edit/Add dialog
        if self.show_edit_dialog {
            let title = if self.editing_index.is_some() {
                "Edit Proxy"
            } else {
                "Add Proxy"
            };
            let mut open = true;
            let mut do_save = false;
            let mut do_cancel = false;

            egui::Window::new(title)
                .id(egui::Id::new("edit_dialog"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::Grid::new("edit_form")
                        .num_columns(2)
                        .spacing([12.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft_name)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            ui.label("Remote host:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft_remote_host)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            ui.label("Remote port:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft_remote_port)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            ui.label("Username:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft_username)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            ui.label("Password:");
                            ui.horizontal(|ui| {
                                let show = self.show_password;
                                let mut edit =
                                    egui::TextEdit::singleline(&mut self.draft_password)
                                        .desired_width(220.0);
                                if !show {
                                    edit = edit.password(true);
                                }
                                ui.add(edit);
                                if ui
                                    .small_button(if show { "Hide" } else { "Show" })
                                    .clicked()
                                {
                                    self.show_password = !self.show_password;
                                }
                            });
                            ui.end_row();

                            ui.label("Local port:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.draft_local_port)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            ui.label("Enabled:");
                            ui.checkbox(&mut self.draft_enabled, "");
                            ui.end_row();
                        });

                    if !self.edit_error.is_empty() {
                        ui.colored_label(egui::Color32::RED, &self.edit_error);
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Save")
                                        .size(16.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(59, 130, 246))
                                .min_size(egui::vec2(90.0, 32.0)),
                            )
                            .clicked()
                        {
                            do_save = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("Cancel").size(16.0))
                                    .min_size(egui::vec2(90.0, 32.0)),
                            )
                            .clicked()
                        {
                            do_cancel = true;
                        }
                    });
                });

            if !open || do_cancel {
                self.show_edit_dialog = false;
            }
            if do_save {
                self.save_draft();
            }
        }

        // Delete confirmation dialog
        if let Some(index) = self.confirm_delete {
            let name = self
                .config
                .proxies
                .get(index)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let mut open = true;
            let mut do_delete = false;
            let mut do_cancel = false;

            egui::Window::new("Confirm Delete")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Delete proxy \"{}\"?", name));
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Delete")
                                        .size(16.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(220, 53, 69))
                                .min_size(egui::vec2(90.0, 32.0)),
                            )
                            .clicked()
                        {
                            do_delete = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("Cancel").size(16.0))
                                    .min_size(egui::vec2(90.0, 32.0)),
                            )
                            .clicked()
                        {
                            do_cancel = true;
                        }
                    });
                });

            if !open || do_cancel {
                self.confirm_delete = None;
            }
            if do_delete {
                self.remove_proxy(index);
                self.confirm_delete = None;
            }
        }
    }
}

impl Drop for TropaApp {
    fn drop(&mut self) {
        for (_, handle) in self.relays.drain() {
            let _ = handle.shutdown_tx.send(true);
        }
    }
}

pub fn run_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([550.0, 380.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tropa Relay",
        options,
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;
            let mut style = (*ctx.style()).clone();

            // Font sizes
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));

            // Spacing
            style.spacing.item_spacing = egui::vec2(10.0, 8.0);
            style.spacing.button_padding = egui::vec2(14.0, 6.0);

            // Rounded widgets
            let cr = egui::CornerRadius::same(6);
            style.visuals.widgets.noninteractive.corner_radius = cr;
            style.visuals.widgets.inactive.corner_radius = cr;
            style.visuals.widgets.hovered.corner_radius = cr;
            style.visuals.widgets.active.corner_radius = cr;
            style.visuals.widgets.open.corner_radius = cr;

            // Rounded windows
            style.visuals.window_corner_radius = egui::CornerRadius::same(10);

            ctx.set_style(style);

            Ok(Box::new(TropaApp::new()))
        }),
    )
    .expect("failed to launch GUI");
}
