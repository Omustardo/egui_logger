#![doc = include_str!("../README.md")]

use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use egui::{text::LayoutJob, Align, Color32, FontSelection, RichText, Style};
use regex::{Regex, RegexBuilder};

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
    records: HashMap<LogLevel, Vec<LogRecord>>,

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
    /// Whether search should be case sensitive.
    pub search_with_case_sensitive: bool,

    #[serde(serialize_with = "serialize_color32", deserialize_with = "deserialize_color32")]
    pub warn_color: Color32,
    #[serde(serialize_with = "serialize_color32", deserialize_with = "deserialize_color32")]
    pub error_color: Color32,
    #[serde(serialize_with = "serialize_color32", deserialize_with = "deserialize_color32")]
    pub highlight_color: Color32,
}

impl Default for EguiLogger {
    fn default() -> Self {
        Self::new()
    }
}

fn default_records() -> HashMap<LogLevel, Vec<LogRecord>> {
    let mut records = HashMap::new();
    records.insert(LogLevel::Error, Vec::new());
    records.insert(LogLevel::Warn, Vec::new());
    records.insert(LogLevel::Info, Vec::new());
    records.insert(LogLevel::Debug, Vec::new());
    records
}
impl EguiLogger {
    pub fn new() -> Self {
        Self {
            records: default_records(),
            category_counts: Default::default(),
            min_display_level: LogLevel::Debug,
            hidden_categories: HashSet::new(), // Empty means show all
            time_format: TimeFormat::LocalTime,
            time_precision: TimePrecision::Seconds,
            show_categories: true,
            show_level: true,
            show_search: false,
            max_message_length: 2000,
            max_records_per_level: 2000,
            search_term: String::new(),
            search_regex: None,
            search_with_regex: false,
            search_with_case_sensitive: false,
            warn_color: Color32::YELLOW,
            error_color: Color32::RED,
            highlight_color: Color32::LIGHT_GRAY,
        }
    }

    pub fn log_error<T: ToString>(&mut self, categories: Vec<T>, message: &str) {
        self.log(LogLevel::Error, categories, message);
    }
    pub fn log_warn<T: ToString>(&mut self, categories: Vec<T>, message: &str) {
        self.log(LogLevel::Warn, categories, message);
    }
    pub fn log_info<T: ToString>(&mut self, categories: Vec<T>, message: &str) {
        self.log(LogLevel::Info, categories, message);
    }
    pub fn log_debug<T: ToString>(&mut self, categories: Vec<T>, message: &str) {
        self.log(LogLevel::Debug, categories, message);
    }

    /// Log a message with the given level and category
    pub fn log<T: ToString>(&mut self, level: LogLevel, categories: Vec<T>, message: &str) {
        let category_strs: Vec<String> = categories.into_iter().map(|c| c.to_string()).collect();

        let truncated_message = if message.len() > self.max_message_length {
            format!("{}...", &message[..self.max_message_length.saturating_sub(3)])
        } else {
            message.parse().unwrap()
        };

        category_strs.iter().for_each(
            |category| {
                self.category_counts.entry(category.to_string())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        );
        let record = LogRecord {
            timestamp: Local::now(),
            level,
            categories: category_strs,
            message: truncated_message,
        };
        self.records.get_mut(&level).unwrap().push(record);
        self.enforce_limits();
    }

    /// Enforce the maximum record limits for a single log level.
    fn enforce_limit(&mut self, level: &LogLevel) {
        let records = self.records.get_mut(level).unwrap();
        if records.len() <= self.max_records_per_level {
            return
        }
        for r in records.drain(&self.max_records_per_level..) {
            r.categories.iter().for_each(
                |category| {
                    self.category_counts.entry(category.to_string()).and_modify(|count| *count -= 1);
                }
            )
        }
    }

    /// Enforce the maximum record limits per level
    fn enforce_limits(&mut self) {
        self.enforce_limit(&LogLevel::Error);
        self.enforce_limit(&LogLevel::Warn);
        self.enforce_limit(&LogLevel::Info);
        self.enforce_limit(&LogLevel::Debug);
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
            let formatted = self.format_record(record, self.get_time_format_padding()).text;
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

        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                self.clear();
            }

            // TODO: add a "copy recent" button based on timestamp? Maybe 2 minutes of logs?
            if ui.button("Copy").clicked() {
                let mut out_string = String::new();
                // TODO: How can I efficiently interleave logs based on timestamp? I shouldn't need to sort if each individual `Vec<LogRecord>` is already sorted. Just need to pop off of each one in turn.
                let time_padding = self.get_time_format_padding();
                self.records.values().into_iter().flatten()
                    .filter(|record| self.matches_filters(record))
                    .for_each(|record| {
                        out_string.push_str(
                            self.format_record(record, time_padding).text.clone().as_str(),
                        );
                        out_string.push_str(" \n");
                    });
                ui.ctx().copy_text(out_string);
            };

            ui.menu_button("Filter", |ui| {

                ui.menu_button("Log Levels", |ui| {
                    for level in vec![LogLevel::Error, LogLevel::Warn, LogLevel::Info, LogLevel::Debug] {
                        if ui
                            .selectable_label(self.min_display_level <= level, level.as_str())
                            .clicked()
                        {
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
                    let categories = self.category_counts.keys();
                    for category in categories {
                        let enabled = !self.hidden_categories.contains(category);
                        if ui.selectable_label(enabled, category.as_str()).clicked() {
                            if self.hidden_categories.contains(category) {
                                self.hidden_categories.remove(category);
                            } else {
                                self.hidden_categories.insert(category.to_string());
                            }
                        }
                    }
                });
            });

            if ui.button("Search").clicked() {
                self.show_search = !self.show_search;
            }

            ui.menu_button("Format", |ui| {
                ui.menu_button("Time", |ui| {
                    ui.radio_value(&mut self.time_format, TimeFormat::Utc, "UTC");
                    ui.radio_value(
                        &mut self.time_format,
                        TimeFormat::LocalTime,
                        "Local Time",
                    );
                    ui.radio_value(&mut self.time_format, TimeFormat::Hide, "Hide");

                    ui.separator();

                    ui.radio_value(
                        &mut self.time_precision,
                        TimePrecision::Seconds,
                        "Seconds",
                    );
                    ui.radio_value(
                        &mut self.time_precision,
                        TimePrecision::Milliseconds,
                        "Milliseconds",
                    );
                });

                if ui.selectable_label(self.show_categories, "Show Categories").clicked() {
                    self.show_categories = !self.show_categories;
                }
                if ui.selectable_label(self.show_level, "Show Log Level").clicked() {
                    self.show_level = !self.show_level;
                }
                // TODO: support changing text colors.
            });
        });

        if self.show_search {
            ui.horizontal(|ui| {
                ui.label("Search: ");
                let response = ui.text_edit_singleline(&mut self.search_term);

                let mut config_changed = false;

                if ui
                    .selectable_label(self.search_with_case_sensitive, "Aa")
                    .on_hover_text("Case sensitive")
                    .clicked()
                {
                    self.search_with_case_sensitive = !self.search_with_case_sensitive;
                    config_changed = true;
                }

                if ui
                    .selectable_label(self.search_with_regex, ".*")
                    .on_hover_text("Use regex")
                    .clicked()
                {
                    self.search_with_regex = !self.search_with_regex;
                    config_changed = true;
                }

                if self.search_with_regex
                    && (response.changed() || config_changed)
                {
                    self.search_regex = RegexBuilder::new(&self.search_term)
                        .case_insensitive(!self.search_with_case_sensitive)
                        .build()
                        .ok()
                }
            });
        }

        let mut logs_displayed: usize = 0;

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(ui.available_height() - 30.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let mut all_records: Vec<&LogRecord> = self.records.values().flatten().collect();
                all_records.sort_by_key(|r| r.timestamp);
                all_records.into_iter().for_each(|record| {
                    // Filter based on log level and categories.
                    if !self.matches_filters(&record) {
                        return
                    }

                    let layout_job = self.format_record(&record, time_padding);
                    let raw_text = layout_job.text.clone();

                    // Filter out logs that are disabled via search options.
                    if !self.search_term.is_empty() && !self.match_string(&raw_text) {
                        return;
                    }

                    let response = ui.label(layout_job);

                    response.clone().context_menu(|ui| {
                        if self.show_categories {
                            ui.label(&record.categories.join(","));
                        }
                        response.highlight();
                        let string_format = format!("[{:?}]: {}", record.level, record.message);

                        // the vertical layout is because otherwise text spacing gets weird
                        ui.vertical(|ui| {
                            ui.monospace(string_format);
                        });

                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(raw_text);
                        }
                    });

                    logs_displayed += 1;
                });
            });
    }

    // match_string determines if the given search terms match the provided string.
    fn match_string(&self, string: &str) -> bool {
        if self.search_with_regex {
            if let Some(matcher) = &self.search_regex {
                matcher.is_match(string)
            } else {
                false
            }
        } else if self.search_with_case_sensitive {
            string.contains(&self.search_term)
        } else {
            string
                .to_lowercase()
                .contains(&self.search_term.to_lowercase())
        }
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

    fn format_record(&self, record: &LogRecord, time_padding: usize) -> LayoutJob {
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

        let mut date_str = RichText::new(format!(
            "{: >width$}",
            self.format_time(record.timestamp),
            width = time_padding
        ))
            .monospace();
        match record.level {
            LogLevel::Warn => date_str = date_str.color(self.warn_color),
            LogLevel::Error => date_str = date_str.color(self.error_color),
            _ => {}
        }

        date_str.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

        let highlight_color = match record.level {
            LogLevel::Warn => self.warn_color,
            LogLevel::Error => self.error_color,
            _ => self.highlight_color,
        };

        RichText::new(level_str + &category_str)
            .monospace()
            .color(highlight_color)
            .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

        let mut message = RichText::new(&record.message).monospace();
        match record.level {
            LogLevel::Warn => message = message.color(self.warn_color),
            LogLevel::Error => message = message.color(self.error_color),
            _ => {}
        }

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
