use egui::{Align, Color32, FontSelection, PopupCloseBehavior, RichText, Style, text::LayoutJob, UiBuilder};

use crate::logger::EguiLogger;
use crate::record::LogRecord;
use crate::types::{LogLevel, TimeFormat, TimePrecision};

pub fn render_logger_ui(logger: &mut EguiLogger, ui: &mut egui::Ui) {
    let time_padding = logger.get_time_format_padding();

    // --- Top Controls ---
    ui.horizontal(|ui| {
        if ui.button("Clear").clicked() {
            logger.clear();
        }

        if ui.button("Copy").clicked() {
            // Collect, filter, then sort records for a chronological copy.
            let mut records_to_copy: Vec<&LogRecord> = logger
                .records()
                .values()
                .flatten()
                .filter(|record| logger.matches_filters(record))
                .collect();
            records_to_copy.sort_by_key(|r| r.timestamp);

            let mut out_string = String::new();
            for record in records_to_copy {
                out_string.push_str(
                    format_record(logger, record, time_padding, ui).text.as_str(), // Use existing time_padding
                );
                out_string.push_str("\n"); // Use newline for better copy-paste
            }
            ui.ctx().copy_text(out_string);
        };

        egui::Popup::menu(&ui.button("Filter"))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.menu_button("Log Levels", |ui| {
                    for level in vec![
                        LogLevel::Error,
                        LogLevel::Warn,
                        LogLevel::Info,
                        LogLevel::Debug,
                    ] {
                        if ui
                            .selectable_label(logger.min_display_level <= level, level.as_str())
                            .clicked()
                        {
                            logger.min_display_level = level;
                        }
                    }
                });

                ui.menu_button("Categories", |ui| {
                    if ui.button("Select All").clicked() {
                        logger.hidden_categories_mut().clear();
                    }
                    if ui.button("Unselect All").clicked() {
                        for category in logger.get_all_categories() {
                            logger.hidden_categories_mut().insert(category);
                        }
                    }
                    // Iterate over category names (&String) from category_counts
                    let categories_to_display: Vec<String> =
                        logger.category_counts().keys().cloned().collect();
                    for cat_str in categories_to_display {
                        let is_currently_shown = !logger.hidden_categories().contains(&cat_str);

                        if ui.selectable_label(is_currently_shown, &cat_str).clicked() {
                            // Toggle state
                            if is_currently_shown {
                                logger.hidden_categories_mut().insert(cat_str.to_string()); // Hide it
                            } else {
                                logger.hidden_categories_mut().remove(&cat_str); // Show it
                            }
                        }
                    }
                });
            });

        if ui.button("Search").clicked() {
            logger.show_search = !logger.show_search;
            if logger.show_search {
                logger.set_should_focus_search(true); // Request focus when opening search
            }
        }

        egui::Popup::menu(&ui.button("Format"))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.menu_button("Time", |ui| {
                    ui.radio_value(&mut logger.time_format, TimeFormat::Utc, "UTC");
                    ui.radio_value(&mut logger.time_format, TimeFormat::LocalTime, "Local Time");
                    ui.radio_value(&mut logger.time_format, TimeFormat::Hide, "Hide");
                    ui.separator();
                    ui.radio_value(&mut logger.time_precision, TimePrecision::Seconds, "Seconds");
                    ui.radio_value(
                        &mut logger.time_precision,
                        TimePrecision::Milliseconds,
                        "Milliseconds",
                    );
                });
                if ui
                    .selectable_label(logger.show_categories, "Show Categories")
                    .clicked()
                {
                    logger.show_categories = !logger.show_categories;
                }
                if ui
                    .selectable_label(logger.show_level, "Show Log Level")
                    .clicked()
                {
                    logger.show_level = !logger.show_level;
                }
                if ui
                    .selectable_label(logger.show_input_area, "Show Input Area")
                    .clicked()
                {
                    logger.show_input_area = !logger.show_input_area;
                }
                ui.horizontal(|ui| {
                    ui.label("Input area hint: ");
                    ui.text_edit_singleline(&mut logger.input_hint);
                    if logger.input_hint.len() > 256 {
                        logger.input_hint.truncate(256);
                    }
                });
            });
    });
    ui.separator();

    // --- Search Bar (if visible) ---
    if logger.show_search {
        ui.horizontal(|ui| {
            ui.label("Search: ");
            let response = ui.text_edit_singleline(&mut logger.search_term);
            if logger.should_focus_search() {
                response.request_focus();
                logger.set_should_focus_search(false); // Reset the flag
            }
            // Limit the length of the search term to avoid absurdly long strings from being
            // compiled to regex and potentially causing performance issues. There are
            // still probably bad edge cases, but people would need to be trying to abuse it.
            // TODO: should too-small strings also not cause regex searches? For example, "a" will match many lines.
            // TODO: Make this behavior clearer to users.
            logger.search_term = logger
                .search_term
                .chars()
                .filter(|c| !c.eq(&'\n') && !c.is_control())
                .take(512)
                .collect();
            let mut config_changed = false;
            if ui
                .selectable_label(logger.search_with_case_sensitive, "Aa")
                .on_hover_text("Case sensitive")
                .clicked()
            {
                logger.search_with_case_sensitive = !logger.search_with_case_sensitive;
                config_changed = true;
            }
            if ui
                .selectable_label(logger.search_with_regex, ".*")
                .on_hover_text("Use regex")
                .clicked()
            {
                logger.search_with_regex = !logger.search_with_regex;
                config_changed = true;
            }
            if logger.search_with_regex && (response.changed() || config_changed) {
                logger.update_search_regex();
            }
        });
        ui.separator(); // Separator after search bar
    }

    // Calculate the height needed for the input area
    let input_height = if logger.show_input_area {
        ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 2.0 // Approximate height
    } else {
        0.0
    };

    // Reserve space for input area at the bottom if enabled
    let available_rect = ui.available_rect_before_wrap();
    let log_area_height = available_rect.height() - input_height;

    // Create log display area with calculated height
    if log_area_height > 0.0 {
        let log_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::Vec2::new(available_rect.width(), log_area_height),
        );

        let mut log_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(log_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT))
        );
        // --- Log Display Area (Central Scroll Area) ---
        // This `ScrollArea` will use the space remaining in `ui` after the top controls
        // and the bottom input panel have been laid out.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false]) // Fill available width and height. Crucial.
            .stick_to_bottom(true)
            .show(&mut log_ui, |scroll_ui| {
                let mut all_records: Vec<&LogRecord> =
                    logger.records().values().flatten().collect();
                all_records.sort_by_key(|r| r.timestamp);

                if all_records.is_empty() && !logger.show_input_area {
                    scroll_ui.label("No logs to display.");
                }

                all_records.into_iter().for_each(|record| {
                    if !logger.matches_filters(&record) {
                        return;
                    }

                    let layout_job = format_record(logger, &record, time_padding, scroll_ui);
                    let raw_text = layout_job.text.clone(); // Still needed for copy in context menu

                    let response = scroll_ui.label(layout_job);

                    response.clone().context_menu(|menu_ui| {
                        if logger.show_categories {
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

    // Add input area at the bottom if enabled
    if logger.show_input_area {
        // Move to the bottom of the available area
        let input_rect = egui::Rect::from_min_size(
            egui::Pos2::new(available_rect.min.x, available_rect.min.y + log_area_height),
            egui::Vec2::new(available_rect.width(), input_height),
        );

        let mut input_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(input_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT))
        );

        input_ui.separator();
        input_ui.horizontal(|ui| {
            // Extract values we need before creating the mutable borrow
            let max_message_length = logger.max_message_length;
            let input_hint = logger.input_hint.clone();

            let input_edit = egui::TextEdit::singleline(logger.input_text_mut())
                .char_limit(max_message_length)
                .cursor_at_end(true)
                .hint_text(input_hint)
                // Unique ID for focusing with ctrl+F.
                .id(egui::Id::new("egui_logger_input_field"))
                .desired_width(f32::INFINITY);

            let response = ui.add(input_edit);

            // Check for Ctrl+F to open search
            if response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.ctrl)
            {
                logger.show_search = true;
                logger.set_should_focus_search(true);
            }

            // Check for Enter key press to submit
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !logger.input_text().trim().is_empty() {
                    let prefix_text: String =
                        logger.input_text_prefix.chars().take(128).collect();
                    let current_input = logger.take_input_text();
                    let submitted_text = format!("{}{}", prefix_text, current_input);
                    logger.log_info(logger.input_categories().to_vec(), submitted_text.as_str());
                    response.request_focus(); // Keep focus on the input field after submit.
                }
                // If input_text was empty and Enter was pressed, focus is lost, no log, no refocus. This allows "escaping" the input field.
            }

            if logger.should_focus_input {
                response.request_focus();
                logger.should_focus_input = false;
            }
        });
    }
}

fn get_level_color(level: LogLevel, ui: &egui::Ui) -> Color32 {
    let visuals = ui.visuals();
    match level {
        LogLevel::Error => visuals.error_fg_color,
        LogLevel::Warn => visuals.warn_fg_color,
        LogLevel::Info => visuals.text_color(),
        LogLevel::Debug => visuals.weak_text_color(),
    }
}

fn format_record(logger: &EguiLogger, record: &LogRecord, time_padding: usize, ui: &egui::Ui) -> LayoutJob {
    let level_str = if logger.show_level {
        format!("[{:}] ", record.level.as_str())
    } else {
        String::new()
    };
    let category_str = if logger.show_categories {
        format!("[{:}] ", record.categories.join(","),)
    } else {
        String::new()
    };
    let mut layout_job = LayoutJob::default();
    let style = Style::default();

    let level_color = get_level_color(record.level, ui);

    let date_str = RichText::new(format!(
        "{: >width$}",
        logger.format_time(record.timestamp),
        width = time_padding
    ))
        .monospace()
        .color(level_color);
    date_str.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    RichText::new(level_str + &category_str)
        .monospace()
        .color(level_color)
        .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    let message = RichText::new(&record.message)
        .monospace()
        .color(level_color);
    message.append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

    layout_job
}