use std::fmt;
use eframe::NativeOptions;
use egui_logger::{TimeFormat, TimePrecision};

fn main() {
    // Initialize the logger
    let mut logger = egui_logger::EguiLogger::new();
    logger.show_categories = false;
    logger.show_level = true;
    logger.time_format = TimeFormat::LocalTime;
    logger.time_precision = TimePrecision::Milliseconds;
    logger.input_text_prefix = "User: ".to_string();
    logger.max_records_per_level = 5;
    logger.set_input_categories(vec![MyLogCategory::Input,MyLogCategory::Dialogue]);

    let app = MyApp::new(logger);

    eframe::run_native("egui_logger", NativeOptions::default(), Box::new(|_cc| Ok(Box::new(app)))).unwrap();
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

struct MyApp {
    logger: egui_logger::EguiLogger,
}
impl MyApp {
    pub fn new(logger: egui_logger::EguiLogger) -> Self {
        Self {
            logger
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::widgets::global_theme_preference_buttons(ui);

            if ui.button("This produces Debug Info").clicked() {
                self.logger.log_debug(vec![MyLogCategory::Network],"Connecting...")
            }
            if ui.button("This produces an Info").clicked() {
                self.logger.log_info(vec![MyLogCategory::Dialogue],"Hello World")
            }
            if ui.button("This produces an Error").clicked() {
                self.logger.log_error( vec![MyLogCategory::Network],"Disconnected unexpectedly!");
            }
            if ui.button("This produces a Warning").clicked() {
                self.logger.log_warn(vec![MyLogCategory::Unknown], "Be warned")
            }
        });
        egui::TopBottomPanel::bottom("chat area").resizable(true).min_height(200.0).show(ctx, |ui| {
            self.logger.show(ui);
        });
    }
}
