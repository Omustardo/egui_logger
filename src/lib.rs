#![doc = include_str!("../README.md")]

use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet, VecDeque};
use egui::{text::LayoutJob, Align, Color32, FontSelection, RichText, Style};
use regex::{Regex, RegexBuilder};

// Trait to handle different category input types
pub trait IntoCategories {
    fn into_categories(self) -> Vec<String>;
}

// Implementation for vectors
impl<T: ToString> IntoCategories for Vec<T> {
    fn into_categories(self) -> Vec<String> {
        self.into_iter().map(|c| c.to_string()).collect()
    }
}

// Implementation for arrays
impl<T: ToString, const N: usize> IntoCategories for [T; N] {
    fn into_categories(self) -> Vec<String> {
        self.into_iter().map(|c| c.to_string()).collect()
    }
}

// Implementation for slices
impl<T: ToString> IntoCategories for &[T] {
    fn into_categories(self) -> Vec<String> {
        self.iter().map(|c| c.to_string()).collect()
    }
}

// Implementation for string types
impl IntoCategories for &str {
    fn into_categories(self) -> Vec<String> {
        vec![self.to_string()]
    }
}
impl IntoCategories for String {
    fn into_categories(self) -> Vec<String> {
        vec![self]
    }
}
impl IntoCategories for &String {
    fn into_categories(self) -> Vec<String> {
        vec![self.clone()]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 3,
    Warn = 2,
    #[default]
    Info = 1,
    Debug = 0,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    Utc,
    LocalTime,
    Hide,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub categories: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TimePrecision {
    Seconds,
    Milliseconds,
}

pub fn serialize_color32<S>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let [r, g, b, a] = color.to_array();
    [r, g, b, a].serialize(serializer)
}

pub fn deserialize_color32<'de, D>(deserializer: D) -> Result<Color32, D::Error>
where
    D: Deserializer<'de>,
{
    let [r, g, b, a] = <[u8; 4]>::deserialize(deserializer)?;
    Ok(Color32::from_rgba_unmultiplied(r, g, b, a))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EguiLogger {
    // TODO: Switch to a BinaryHeap. This will be way more efficient when iterating over all records
    //   since it will have them sorted by timestamp for free. It should also be able to do a fixed max size? Try using itertools?
    records: HashMap<LogLevel, VecDeque<LogRecord>>,

    // Count of each category. Kept up to date as new records are added and old are removed.
    // Reports with multiple categories are counted once per category.
    // This is necessary to keep track of which categories exist, and which have been seen in the
    // past.
    category_counts: HashMap<String, u32>,

    /// Minimum log level to display (e.g. Info will display Info,Warn,Error but not Debug)
    pub min_display_level: LogLevel,
    /// Categories that should be hidden.
    /// New categories are shown by default. Note that categories may be saved here which
    /// aren't actually in the logger! This can happen if logs with those categories appear
    /// but are then either clear()'d or get pushed out of the buffer. In these cases, it's
    /// necessary to save categories here in order to save user preferences if case the logs
    /// show up again.
    hidden_categories: HashSet<String>,

    /// How to format timestamps
    pub time_format: TimeFormat,
    pub time_precision: TimePrecision,

    /// Whether to show a categories in the text.
    pub show_categories: bool,
    /// Whether to show log level in the text.
    pub show_level: bool,
    // Whether the search box is visible.
    pub show_search: bool,
    /// A trigger for the UI to focus on the search area. If true, the next rendered frame
    /// will set focus, and set this value to false.
    /// For example, if a user clicks the search button, it should immediately focus on the
    /// search input. That change is done through this trigger.
    #[serde(skip)]
    should_focus_search: bool,

    /// Maximum length per message (longer messages get truncated)
    /// Note that reducing this value won't truncate existing logs,
    /// increasing the value will not recover the truncated portion of logs.
    pub max_message_length: usize,
    /// Maximum number of records to keep per log level.
    /// Log levels are separated so that (for example) a bunch of Debug logs don't hide Error level
    /// logs. Note that in this example, the display might look odd at an older timestamp.
    /// There might appear to be many error logs and no debug logs, because the older debug logs
    /// have been removed. This is a natural consequence of this approach and is probably ok.
    /// TODO: Call this out in documentation, examples, and tests. Or find an alternative that's
    ///   more obvious to the user.
    /// TODO: Consider removing records after a time delay (a few hours? one game session?).
    pub max_records_per_level: usize,
    /// Current search term for filtering.
    pub search_term: String,
    // Storing this regex isn't important since it's generated from the search_term.
    // It only exists here to prevent the regex from being re-calculated on every frame.
    #[serde(skip)]
    search_regex: Option<Regex>,
    // Whether regex based searching is enabled.
    pub search_with_regex: bool,
    /// Whether search should be case sensitive. This also applies to regex search.
    pub search_with_case_sensitive: bool,

    // Fields related to the text box and user input.

    // Whether to show the entire text input section.
    pub show_input_area: bool,
    /// A trigger for the UI to focus on the text input area. If true, the next rendered frame
    /// will set focus to the input area, and set this value to false.
    #[serde(skip)]
    pub should_focus_input: bool,
    // The current user input.
    input_text: String,
    // When input_text is empty, this text is displayed to indicate where the input box is.
    pub input_hint: String,
    // A message to prepend to all user inputs.
    pub input_text_prefix: String,
    // Categories to apply to LogRecords triggered by user input.
    input_categories: Vec<String>,
    // The log level to apply to LogRecords triggered by user input.
    pub input_level: LogLevel,
}

impl Default for EguiLogger {
    fn default() -> Self {
        Self::new()
    }
}

fn default_records() -> HashMap<LogLevel, VecDeque<LogRecord>> {
    let mut records = HashMap::new();
    records.insert(LogLevel::Error, VecDeque::new());
    records.insert(LogLevel::Warn, VecDeque::new());
    records.insert(LogLevel::Info, VecDeque::new());
    records.insert(LogLevel::Debug, VecDeque::new());
    records
}
impl EguiLogger {
    pub fn new() -> Self {
        Self {
            records: default_records(),
            category_counts: Default::default(),
            min_display_level: LogLevel::Debug,
            hidden_categories: HashSet::new(),
            time_format: TimeFormat::LocalTime,
            time_precision: TimePrecision::Seconds,
            show_categories: true,
            show_level: true,
            show_search: false,
            should_focus_search: false,
            max_message_length: 2000,
            max_records_per_level: 2000,
            search_term: String::new(),
            search_regex: None,
            search_with_regex: false,
            search_with_case_sensitive: false,
            show_input_area: true,
            should_focus_input: false,
            input_hint: "Type a message and press Enter...".to_string(),
            input_text: String::new(),
            input_text_prefix: String::new(),
            input_categories: vec!["Input".parse().unwrap()],
            input_level: LogLevel::Info,
        }
    }

    pub fn log_error<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Error, categories, message);
    }

    pub fn log_warn<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Warn, categories, message);
    }

    pub fn log_info<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Info, categories, message);
    }

    pub fn log_debug<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Debug, categories, message);
    }

    /// Log a message with the given level and category
    pub fn log<C: IntoCategories, M: std::fmt::Display>(
        &mut self,
        level: LogLevel,
        categories: C,
        message: M
    ) {
        let category_strs = categories.into_categories();

        // Convert message to string without requiring &format!
        let message_str = message.to_string();
        let cleaned_message: String = message_str.chars().filter(|c| !c.eq(&'\n')).collect();

        let truncated_message = if cleaned_message.len() > self.max_message_length {
            format!("{}...", &cleaned_message[..self.max_message_length.saturating_sub(3)])
        } else {
            cleaned_message
        };

        category_strs.iter().for_each(|category| {
            self.category_counts
                .entry(category.to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        });

        let record = LogRecord {
            timestamp: chrono::Local::now(),
            level,
            categories: category_strs,
            message: truncated_message,
        };

        self.records.get_mut(&level).unwrap().push_back(record);
        self.enforce_limits();
    }

    /// Enforce the maximum record limits for a single log level.
    fn enforce_limit(&mut self, level: &LogLevel) {
        let records = self.records.get_mut(level).unwrap();
        while records.len() > self.max_records_per_level {
             if let Some(r) = records.pop_front() {
                 r.categories.iter().for_each(
                     |category| {
                         self.category_counts.entry(category.to_string()).and_modify(|count| *count -= 1);
                     }
                 )
             }
        }
    }

    /// Enforce the maximum record limits per level
    fn enforce_limits(&mut self) {
        self.enforce_limit(&LogLevel::Error);
        self.enforce_limit(&LogLevel::Warn);
        self.enforce_limit(&LogLevel::Info);
        self.enforce_limit(&LogLevel::Debug);
    }

    // Categories to apply to LogRecords triggered by user input.
    pub fn set_input_categories<T: ToString>(&mut self, categories: Vec<T>) {
        self.input_categories = categories.into_iter().map(|c| c.to_string()).collect();
    }

    /// Clear all log records
    pub fn clear(&mut self) {
        self.records.iter_mut().for_each(|(_, r)| r.clear());
        self.category_counts.clear();
    }

    /// Get all records that match current filters
    pub fn filtered_records(&self) -> Vec<&LogRecord> {
        self.records.values().flatten()
            .filter(|record| self.matches_filters(record))
            .collect()
    }
    /// Get just the formatted text content without colors for search filtering
    fn format_record_text(&self, record: &LogRecord, time_padding: usize) -> String {
        let level_str = if self.show_level {
            format!("[{:}] ", record.level.as_str())
        } else {
            String::new()
        };
        let category_str = if self.show_categories {
            format!(
                "[{:}] ",
                record.categories.join(","),
            )
        } else {
            String::new()
        };

        let time_str = format!(
            "{: >width$}",
            self.format_time(record.timestamp),
            width = time_padding
        );

        format!("{}{}{}{}", time_str, level_str, category_str, record.message)
    }

    /// Check if a record matches current filters
    fn matches_filters(&self, record: &LogRecord) -> bool {
        // Level filtering (show this level and more severe)
        if record.level < self.min_display_level {
            return false;
        }

        // Category filtering
        if !self.hidden_categories.is_empty() {
            // TODO: Should it be this way, or based on all of the categories being hidden? What's more intuitive?
            // If any of a LogRecord's categories are marked as hidden, then hide the whole thing.
            if record.categories.iter()
                .any(|cat| self.hidden_categories.contains(cat)) {
                return false;
            }
        }

        // Search filtering
        if !self.search_term.is_empty() {
            let formatted = self.format_record_text(record, self.get_time_format_padding());
            let matches = if self.search_with_regex {
                // Note that the regex itself is generated to be case sensitive or not, so
                // that the regex + case check doesn't need to happen here.
                if self.search_regex.is_none() {
                    true
                } else {
                    self.search_regex.as_ref().unwrap().is_match(&formatted)
                }
            } else if self.search_with_case_sensitive {
                formatted.contains(&self.search_term)
            } else {
                formatted.to_lowercase().contains(&self.search_term.to_lowercase())
            };
            if !matches {
                return false;
            }
        }

        true
    }

    /// Get all unique categories that have been logged
    pub fn get_all_categories(&self) -> HashSet<String> {
        self.category_counts.keys().cloned().collect()
    }

    /// Get total number of records
    pub fn total_records(&self) -> usize {
        self.records.values().map(|r| r.len()).sum()
    }

    fn get_time_format_padding(&self) -> usize {
        // Format a time and use that to determine the padding to provide for all other rows.
        // TODO: Does this work? It feels like using a fixed timestamp would be better than
        //  non-determinism. Maybe I can get this value at compile-time, or find it myself
        //  and hard-code it in.
        self.format_time(Local::now()).len()
    }

    pub fn show_category<T: ToString>(&mut self, category: T) {
        self.hidden_categories.remove(&category.to_string());
    }

    pub fn hide_category<T: ToString>(&mut self, category: T) {
        self.hidden_categories.insert(category.to_string());
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let time_padding = self.get_time_format_padding();

        // --- Top Controls ---
        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                self.clear();
            }

            if ui.button("Copy").clicked() {
                // Collect, filter, then sort records for a chronological copy.
                let mut records_to_copy: Vec<&LogRecord> = self.records
                    .values()
                    .flatten()
                    .filter(|record| self.matches_filters(record))
                    .collect();
                records_to_copy.sort_by_key(|r| r.timestamp);

                let mut out_string = String::new();
                for record in records_to_copy {
                    out_string.push_str(
                        self.format_record(record, time_padding, ui).text.as_str(), // Use existing time_padding
                    );
                    out_string.push_str("\n"); // Use newline for better copy-paste
                }
                ui.ctx().copy_text(out_string);
            };

            ui.menu_button("Filter", |ui| {
                ui.menu_button("Log Levels", |ui| {
                    for level in vec![LogLevel::Error, LogLevel::Warn, LogLevel::Info, LogLevel::Debug] {
                        if ui.selectable_label(self.min_display_level <= level, level.as_str()).clicked() {
                            self.min_display_level = level;
                        }
                    }
                });

                ui.menu_button("Categories", |ui| {
                    if ui.button("Select All").clicked() {
                        self.hidden_categories.clear();
                    }
                    if ui.button("Unselect All").clicked() {
                        for category in self.get_all_categories() {
                            self.hidden_categories.insert(category);
                        }
                    }
                    // Iterate over category names (&String) from category_counts
                    let categories_to_display: Vec<String> = self.category_counts.keys().cloned().collect();
                    for cat_str in categories_to_display {
                        let is_currently_shown = !self.hidden_categories.contains(&cat_str);

                        if ui.selectable_label(is_currently_shown, &cat_str).clicked() {
                            // Toggle state
                            if is_currently_shown {
                                self.hidden_categories.insert(cat_str.to_string()); // Hide it
                            } else {
                                self.hidden_categories.remove(&cat_str); // Show it
                            }
                        }
                    }
                });
            });

            if ui.button("Search").clicked() {
                self.show_search = !self.show_search;
                if self.show_search {
                    self.should_focus_search = true; // Request focus when opening search
                }
            }

            ui.menu_button("Format", |ui| {
                ui.menu_button("Time", |ui| {
                    ui.radio_value(&mut self.time_format, TimeFormat::Utc, "UTC");
                    ui.radio_value(&mut self.time_format, TimeFormat::LocalTime, "Local Time");
                    ui.radio_value(&mut self.time_format, TimeFormat::Hide, "Hide");
                    ui.separator();
                    ui.radio_value(&mut self.time_precision, TimePrecision::Seconds, "Seconds");
                    ui.radio_value(&mut self.time_precision, TimePrecision::Milliseconds, "Milliseconds");
                });
                if ui.selectable_label(self.show_categories, "Show Categories").clicked() {
                    self.show_categories = !self.show_categories;
                }
                if ui.selectable_label(self.show_level, "Show Log Level").clicked() {
                    self.show_level = !self.show_level;
                }
                if ui.selectable_label(self.show_input_area, "Show Input Area").clicked() {
                    self.show_input_area = !self.show_input_area;
                }
                ui.horizontal(|ui| {
                    ui.label("Input area hint: ");
                    ui.text_edit_singleline(&mut self.input_hint);
                    if self.input_hint.len() > 256 {
                        self.input_hint.truncate(256);
                    }
                });
            });
        });
        ui.separator();

        // --- Search Bar (if visible) ---
        if self.show_search {
            ui.horizontal(|ui| {
                ui.label("Search: ");
                let response = ui.text_edit_singleline(&mut self.search_term);
                if self.should_focus_search {
                    response.request_focus();
                    self.should_focus_search = false; // Reset the flag
                }
                // Limit the length of the search term to avoid absurdly long strings from being
                // compiled to regex and potentially causing performance issues. There are
                // still probably bad edge cases, but people would need to be trying to abuse it.
                // TODO: should too-small strings also not cause regex searches? For example, "a" will match many lines.
                // TODO: Make this behavior clearer to users.
                self.search_term = self.search_term.chars().filter(|c| !c.eq(&'\n') && !c.is_control()).take(512).collect();
                let mut config_changed = false;
                if ui.selectable_label(self.search_with_case_sensitive, "Aa").on_hover_text("Case sensitive").clicked() {
                    self.search_with_case_sensitive = !self.search_with_case_sensitive;
                    config_changed = true;
                }
                if ui.selectable_label(self.search_with_regex, ".*").on_hover_text("Use regex").clicked() {
                    self.search_with_regex = !self.search_with_regex;
                    config_changed = true;
                }
                if self.search_with_regex && (response.changed() || config_changed) {
                    self.search_regex = RegexBuilder::new(&self.search_term)
                        .case_insensitive(!self.search_with_case_sensitive)
                        .build()
                        .ok();
                }
            });
            ui.separator(); // Separator after search bar
        }

        // --- Input Area (Bottom Panel) ---
        // This panel is defined before the central log area so it can reserve its space.
        if self.show_input_area {
            // Use a unique ID for the panel to avoid conflicts if this logger is used multiple times in same ui scope.
            let panel_id = ui.id().with("egui_logger_input_panel");
            egui::TopBottomPanel::bottom(panel_id)
                .resizable(false) // Input area usually has a fixed height
                .show_inside(ui, |panel_ui| { // `panel_ui` is the Ui for the bottom panel
                    panel_ui.horizontal(|input_ui| { // Use `input_ui` (which is `panel_ui` here)
                        let input_edit = egui::TextEdit::singleline(&mut self.input_text)
                            .char_limit(self.max_message_length)
                            .cursor_at_end(true)
                            .hint_text(self.input_hint.clone())
                            .id(egui::Id::new("egui_logger_input_field")) // Unique ID for focus
                            .desired_width(f32::INFINITY);

                        let response = input_ui.add(input_edit);

                        // Check for Ctrl+F to open search
                        if response.has_focus() && input_ui.input(|i| {
                            i.key_pressed(egui::Key::F) && i.modifiers.ctrl
                        }) {
                            self.show_search = true;
                            self.should_focus_search = true;
                        }

                        // Check for Enter key press to submit
                        if response.lost_focus() && input_ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if !self.input_text.trim().is_empty() {
                                let prefix_text: String = self.input_text_prefix.chars().take(16).collect();
                                let current_input = std::mem::take(&mut self.input_text); // Get text and clear field
                                let submitted_text = format!("{}{}", prefix_text, current_input);
                                self.log_info(self.input_categories.clone(), submitted_text.as_str());
                                response.request_focus(); // Keep focus on the input field after submit.
                            }
                            // If input_text was empty and Enter was pressed, focus is lost, no log, no refocus. This allows "escaping" the input field.
                        }

                        if self.should_focus_input {
                            response.request_focus();
                            self.should_focus_input = false;
                        }
                    });
                });

        }

        // --- Log Display Area (Central Scroll Area) ---
        // This `ScrollArea` will use the space remaining in `ui` after the top controls
        // and the bottom input panel have been laid out.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false]) // Fill available width and height. Crucial.
            .stick_to_bottom(true)
            .show(ui, |scroll_ui| {

                let mut all_records: Vec<&LogRecord> = self.records.values().flatten().collect();
                all_records.sort_by_key(|r| r.timestamp);

                if all_records.is_empty() && !self.show_input_area { // Only if truly nothing else might take vertical space
                    scroll_ui.label("No logs to display.");
                }

                all_records.into_iter().for_each(|record| {
                    if !self.matches_filters(&record) {
                        return;
                    }

                    let layout_job = self.format_record(&record, time_padding, scroll_ui);
                    let raw_text = layout_job.text.clone(); // Still needed for copy in context menu

                    let response = scroll_ui.label(layout_job);

                    response.clone().context_menu(|menu_ui| {
                        if self.show_categories {
                            menu_ui.label(&record.categories.join(","));
                        }
                        let string_format = format!("[{:?}]: {}", record.level, record.message);
                        menu_ui.vertical(|v_ui| {
                            v_ui.monospace(string_format);
                        });
                        if menu_ui.button("Copy").clicked() {
                            menu_ui.ctx().copy_text(raw_text);
                            menu_ui.close();
                        }
                    });
                });
            });
    }

    fn format_time(
        &self,
        time: DateTime<Local>,
    ) -> String {
        let time = match (self.time_format, self.time_precision) {
            (TimeFormat::Utc, TimePrecision::Seconds) => time
                .to_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            (TimeFormat::Utc, TimePrecision::Milliseconds) => time
                .to_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            (TimeFormat::LocalTime, TimePrecision::Seconds) => time.format("%T").to_string(),
            (TimeFormat::LocalTime, TimePrecision::Milliseconds) => time.format("%T%.3f").to_string(),
            (TimeFormat::Hide, _) => String::new(),
        };
        if self.time_format == TimeFormat::Hide {
            time
        } else {
            time + " "
        }
    }


    fn get_level_color(&self, level: LogLevel, ui: &egui::Ui) -> Color32 {
        let visuals = ui.visuals();
        match level {
            LogLevel::Error => visuals.error_fg_color,
            LogLevel::Warn => visuals.warn_fg_color,
            LogLevel::Info => visuals.text_color(),
            LogLevel::Debug => visuals.weak_text_color()
        }
    }

    fn format_record(&self, record: &LogRecord, time_padding: usize, ui: &egui::Ui) -> LayoutJob {
        let level_str = if self.show_level {
            format!("[{:}] ", record.level.as_str())
        } else {
            String::new()
        };
        let category_str = if self.show_categories {
            format!(
                "[{:}] ",
                record.categories.join(","),
            )
        } else {
            String::new()
        };
        let mut layout_job = LayoutJob::default();
        let style = Style::default();

        let level_color = self.get_level_color(record.level, ui);

        let date_str = RichText::new(format!(
            "{: >width$}",
            self.format_time(record.timestamp),
            width = time_padding
        )).monospace().color(level_color);
        date_str.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

        RichText::new(level_str + &category_str)
            .monospace()
            .color(level_color)
            .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

        let message = RichText::new(&record.message).monospace().color(level_color);
        message.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

        layout_job
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;
    use std::fmt::Formatter;
    use super::*;

    // Example usage and category enum
    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
    pub enum LogCategory {
        Unknown,
        Dialogue,
        Combat,
        UI,
        Network,
        Save,
        Load,
        Audio,
        Rendering,
    }

    impl fmt::Display for LogCategory {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

            match self {
                LogCategory::Unknown => write!(f, "Unknown"),
                LogCategory::Dialogue => write!(f, "Dialogue"),
                LogCategory::Combat => write!(f, "Combat"),
                LogCategory::UI => write!(f, "UI"),
                LogCategory::Network => write!(f, "Network"),
                LogCategory::Save => write!(f, "Save"),
                LogCategory::Load => write!(f, "Load"),
                LogCategory::Audio => write!(f, "Audio"),
                LogCategory::Rendering => write!(f, "Rendering"),
            }
        }
    }

    #[test]
    fn test_basic_logging() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Test message");

        assert_eq!(logger.total_records(), 1);
        let records = logger.filtered_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, LogLevel::Info);
        assert_eq!(records[0].message, "Test message");
    }

    #[test]
    fn test_level_filtering() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Debug, vec![LogCategory::Unknown], "Debug msg");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Info msg");
        logger.log(LogLevel::Warn, vec![LogCategory::Unknown], "Warn msg");
        logger.log(LogLevel::Error, vec![LogCategory::Unknown], "Error msg");

        // Set max level to Warn - should show Warn and Error only
        logger.min_display_level = LogLevel::Warn;
        let visible = logger.filtered_records();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|r| r.level >= LogLevel::Warn));
    }

    #[test]
    fn test_category_filtering() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Info,vec![LogCategory::Dialogue], "Dialogue msg");
        logger.log(LogLevel::Info, vec![LogCategory::Combat], "Combat msg");

        // Enable only Dialogue category
        logger.show_category(&LogCategory::Dialogue);
        logger.hide_category(&LogCategory::Combat);
        let visible = logger.filtered_records();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].categories[0], "Dialogue");
    }

    #[test]
    fn test_search_filtering() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Hello world");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Goodbye world");

        logger.search_term = "Hello".to_string();
        let visible = logger.filtered_records();
        assert_eq!(visible.len(), 1);
        assert!(visible[0].message.contains("Hello"));
    }

    #[test]
    fn test_message_truncation() {
        let mut logger = EguiLogger::new();
        logger.max_message_length = 10;

        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "This is a very long message that should be truncated");

        let records = logger.filtered_records();
        assert_eq!(records.len(), 1);
        assert!(records[0].message.len() <= 10);
        assert!(records[0].message.ends_with("..."));
    }
}
// TODO: Add a lot more tests.
