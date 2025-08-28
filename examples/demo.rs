use eframe::NativeOptions;
use egui_logger::{IntoCategories, TimeFormat, TimePrecision};
use std::fmt;

fn main() {
    // Initialize the logger
    let mut logger = egui_logger::EguiLogger::new();
    logger.show_categories = false;
    logger.show_level = true;
    logger.time_format = TimeFormat::LocalTime;
    logger.time_precision = TimePrecision::Milliseconds;
    logger.input_text_prefix = "User: ".to_string();
    logger.max_records_per_level = 5;
    logger.set_input_categories(vec![MyLogCategory::Input, MyLogCategory::Dialogue]);

    let app = MyApp::new(logger);

    eframe::run_native(
        "egui_logger",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .unwrap();
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum MyLogCategory {
    Unknown,
    Dialogue,
    Input,
    Combat,
    Network,
}
impl fmt::Display for MyLogCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl IntoCategories for MyLogCategory {
    fn into_categories(self) -> Vec<String> {
        vec![self.to_string()]
    }
}

struct MyApp {
    logger: egui_logger::EguiLogger,
}
impl MyApp {
    pub fn new(logger: egui_logger::EguiLogger) -> Self {
        Self { logger }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::widgets::global_theme_preference_buttons(ui);

            if ui.button("This produces Debug Info").clicked() {
                self.logger.log_debug(MyLogCategory::Network, "Connecting...")
            }
            if ui.button("This produces an Info").clicked() {
                self.logger.log_info(MyLogCategory::Dialogue, "Hello World")
            }
            if ui.button("This produces an Error").clicked() {
                self.logger.log_error(MyLogCategory::Network, "Disconnected unexpectedly!");
            }
            if ui.button("This produces a Warning").clicked() {
                self.logger.log_warn(MyLogCategory::Unknown, "Be warned")
            }
            ui.separator();
            if ui.button("Focus on text input area").clicked() {
                self.logger.should_focus_input = true;
            }
            ui.separator();
            ui.checkbox(&mut self.logger.show_settings, "Toggle settings");
            ui.separator();
            ui.label("When in the text input area, CTRL+F goes to search. You can tab between interactive things.");
            ui.label("Pressing Enter gets back to the input area, except if in the search input area, where it just exits that.");
            ui.label("In a real application, how to get users into the text area is up to you. It is exposed through setting `logger.should_focus_input` to true. This demo handles it by watching for for presses of the Enter key.");
            // Only handle Enter if nothing has focus
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if ctx.memory(|mem| mem.focused()).is_none() {
                    self.logger.should_focus_input = true;
                }
            }
        });
        egui::TopBottomPanel::bottom("chat area")
            .resizable(true)
            .min_height(200.0)
            .show(ctx, |ui| {
                self.logger.show(ui);
            });
    }
}
