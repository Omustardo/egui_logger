use eframe::NativeOptions;
use egui_logger::{TimeFormat, TimePrecision};

fn main() {
    // Initialize the logger
    let mut logger = egui_logger::EguiLogger::new();
    logger.show_categories = false;
    logger.show_level = true;
    logger.time_format = TimeFormat::LocalTime;
    logger.time_precision = TimePrecision::Milliseconds;

    let app = MyApp::new(logger);

    eframe::run_native("egui_logger", NativeOptions::default(), Box::new(|_cc| Ok(Box::new(app)))).unwrap();
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
            if ui.button("This produces Debug Info").clicked() {
                self.logger.log_debug(vec!["Category1"],"Very verbose Debug Info")
            }
            if ui.button("This produces an Info").clicked() {
                self.logger.log_info(vec!["Category1"],"Some Info")
            }
            if ui.button("This produces an Error").clicked() {
                self.logger.log_error( vec!["Serious", "OMG"],"Error doing Something");
            }
            if ui.button("This produces a Warning").clicked() {
                self.logger.log_warn(vec![""], "Warn about something")
            }
        });
        egui::TopBottomPanel::bottom("chat area").min_height(200_f32).show(ctx, |ui| {
            self.logger.show(ui);
        });
    }
}
