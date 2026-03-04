//! Voice command overlay UI
//!
//! Renders the voice command interface including:
//! - Recording indicator (pulsing red dot)
//! - Processing spinner
//! - Interpretation menu with selection highlight
//! - Permission prompt display
//! - Joystick hint at bottom
//! - Voice settings dialog

use bevy_egui::egui;
use std::time::Instant;

use crate::voice_command::{
    AgentAction, AgentInterpretation, VoiceCommandConfig,
    VoiceCommandState,
};

/// Render the voice command overlay
/// Returns true if the overlay consumed input (modal behavior)
pub fn render_voice_command_overlay(
    ctx: &egui::Context,
    state: &VoiceCommandState,
    start_time: Option<Instant>,
) -> bool {
    // Semi-transparent background overlay
    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();

    egui::Area::new("voice_command_overlay".into())
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Dark overlay background
            let painter = ui.painter();
            painter.rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            // Center the content
            let center = screen_rect.center();
            let panel_width = 500.0;
            let panel_height = match state {
                VoiceCommandState::Recording { ref live_transcript, .. } => {
                    // Base height plus extra for transcript display
                    let transcript_len = live_transcript
                        .lock()
                        .map(|t| t.len())
                        .unwrap_or(0);
                    if transcript_len > 0 { 220.0 } else { 150.0 }
                }
                VoiceCommandState::Transcribing => 120.0,
                VoiceCommandState::Interpreting { .. } => 120.0,
                VoiceCommandState::AwaitingPermission { .. } => 200.0,
                VoiceCommandState::Selecting { interpretations, .. } => {
                    // Calculate height based on script lines
                    let max_lines: usize = interpretations.iter()
                        .map(|i| match &i.action {
                            AgentAction::Script(s) => s.lines().count().max(1),
                            AgentAction::Cancel => 1,
                        })
                        .max()
                        .unwrap_or(1);
                    100.0 + (interpretations.len() as f32 * (30.0 + max_lines as f32 * 18.0)).min(400.0)
                }
            };

            let panel_rect = egui::Rect::from_center_size(
                center,
                egui::vec2(panel_width, panel_height),
            );

            // Panel background
            painter.rect_filled(
                panel_rect,
                12.0,
                egui::Color32::from_rgb(30, 30, 40),
            );
            painter.rect_stroke(
                panel_rect,
                12.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 120)),
                egui::StrokeKind::Outside,
            );

            // Content area
            let content_rect = panel_rect.shrink(20.0);
            let ui_builder = egui::UiBuilder::new().max_rect(content_rect);

            #[allow(deprecated)]
            ui.allocate_new_ui(ui_builder, |ui| {
                ui.vertical_centered(|ui| {
                    match state {
                        VoiceCommandState::Recording { live_transcript, audio_level, .. } => {
                            let transcript_text = live_transcript
                                .lock()
                                .map(|t| t.clone())
                                .unwrap_or_default();
                            let level = audio_level
                                .lock()
                                .map(|l| *l)
                                .unwrap_or(0.0);
                            render_recording_indicator(ui, start_time, &transcript_text, level);
                        }
                        VoiceCommandState::Transcribing => {
                            render_transcribing_indicator(ui);
                        }
                        VoiceCommandState::Interpreting { transcript, .. } => {
                            render_interpreting_indicator(ui, transcript);
                        }
                        VoiceCommandState::AwaitingPermission { transcript, requested_targets, reason } => {
                            render_permission_prompt(ui, transcript, requested_targets, reason);
                        }
                        VoiceCommandState::Selecting { transcript, interpretations, selected } => {
                            render_selection_menu(ui, transcript, interpretations, *selected);
                        }
                    }
                });
            });
        });

    // Request repaint for animations
    ctx.request_repaint();

    true // Overlay is modal
}

fn render_recording_indicator(ui: &mut egui::Ui, start_time: Option<Instant>, live_transcript: &str, audio_level: f32) {
    ui.add_space(10.0);

    // Title
    ui.heading(egui::RichText::new("Voice Command").color(egui::Color32::WHITE));
    ui.add_space(10.0);

    // Pulsing red recording dot and duration
    let elapsed = start_time
        .map(|t| t.elapsed().as_secs_f32())
        .unwrap_or(0.0);
    let pulse = ((elapsed * 3.0).sin() * 0.5 + 0.5) as f32;
    let dot_color = egui::Color32::from_rgba_unmultiplied(
        220 + (pulse * 35.0) as u8,
        (50.0 + pulse * 50.0) as u8,
        (50.0 + pulse * 50.0) as u8,
        255,
    );

    ui.horizontal(|ui| {
        let (response, painter) = ui.allocate_painter(egui::vec2(24.0, 24.0), egui::Sense::hover());
        let center = response.rect.center();
        let radius = 10.0 + pulse * 2.0;
        painter.circle_filled(center, radius, dot_color);

        ui.add_space(8.0);

        // Duration display
        if let Some(start) = start_time {
            let duration = start.elapsed().as_secs_f32();
            ui.label(
                egui::RichText::new(format!("{:.1}s", duration))
                    .size(16.0)
                    .color(egui::Color32::LIGHT_GRAY),
            );
        }
    });

    ui.add_space(10.0);

    // Audio level meter
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Mic:")
                .size(14.0)
                .color(egui::Color32::GRAY),
        );
        ui.add_space(8.0);

        // Draw level bar
        let bar_width = 200.0;
        let bar_height = 16.0;
        let (response, painter) = ui.allocate_painter(egui::vec2(bar_width, bar_height), egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 40, 50));

        // Level fill
        let fill_width = rect.width() * audio_level;
        if fill_width > 0.0 {
            let fill_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(fill_width, rect.height()),
            );
            // Color based on level: green -> yellow -> red
            let color = if audio_level < 0.5 {
                egui::Color32::from_rgb(80, 200, 80)
            } else if audio_level < 0.8 {
                egui::Color32::from_rgb(200, 200, 80)
            } else {
                egui::Color32::from_rgb(200, 80, 80)
            };
            painter.rect_filled(fill_rect, 4.0, color);
        }

        // Border
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 100)), egui::StrokeKind::Inside);
    });

    ui.add_space(10.0);

    // Live transcript display
    if !live_transcript.is_empty() {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(20, 25, 35))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(live_transcript)
                        .size(18.0)
                        .color(egui::Color32::WHITE),
                );
            });
        ui.add_space(10.0);
    } else {
        // Show placeholder when no transcript yet
        ui.label(
            egui::RichText::new("Listening...")
                .size(16.0)
                .italics()
                .color(egui::Color32::from_rgb(100, 100, 120)),
        );
        ui.add_space(10.0);
    }

    ui.label(
        egui::RichText::new("Release trigger to stop")
            .size(14.0)
            .color(egui::Color32::GRAY),
    );
}

fn render_transcribing_indicator(ui: &mut egui::Ui) {
    ui.add_space(15.0);
    ui.heading(egui::RichText::new("Voice Command").color(egui::Color32::WHITE));
    ui.add_space(20.0);

    ui.spinner();
    ui.add_space(10.0);
    ui.label(
        egui::RichText::new("Transcribing...")
            .size(16.0)
            .color(egui::Color32::LIGHT_GRAY),
    );
}

fn render_interpreting_indicator(ui: &mut egui::Ui, transcript: &str) {
    ui.add_space(10.0);
    ui.heading(egui::RichText::new("Voice Command").color(egui::Color32::WHITE));
    ui.add_space(10.0);

    // Show transcript
    ui.label(
        egui::RichText::new(format!("\"{}\"", transcript))
            .size(14.0)
            .italics()
            .color(egui::Color32::LIGHT_GRAY),
    );
    ui.add_space(15.0);

    ui.spinner();
    ui.add_space(5.0);
    ui.label(
        egui::RichText::new("Interpreting...")
            .size(16.0)
            .color(egui::Color32::LIGHT_GRAY),
    );
}

fn render_permission_prompt(
    ui: &mut egui::Ui,
    transcript: &str,
    requested_targets: &[String],
    reason: &str,
) {
    ui.add_space(5.0);
    ui.heading(egui::RichText::new("Permission Required").color(egui::Color32::YELLOW));
    ui.add_space(10.0);

    // Show transcript
    ui.label(
        egui::RichText::new(format!("\"{}\"", transcript))
            .size(14.0)
            .italics()
            .color(egui::Color32::LIGHT_GRAY),
    );
    ui.add_space(10.0);

    // Permission request
    let targets_str = requested_targets.join(", ");
    ui.label(
        egui::RichText::new(format!("Agent wants to view: {}", targets_str))
            .size(15.0)
            .color(egui::Color32::WHITE),
    );

    if !reason.is_empty() {
        ui.label(
            egui::RichText::new(format!("Reason: {}", reason))
                .size(13.0)
                .color(egui::Color32::GRAY),
        );
    }

    ui.add_space(15.0);

    // Joystick hints
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("→ Allow").color(egui::Color32::GREEN));
        ui.add_space(30.0);
        ui.label(egui::RichText::new("← Deny").color(egui::Color32::from_rgb(255, 100, 100)));
    });
}

fn render_selection_menu(
    ui: &mut egui::Ui,
    transcript: &str,
    interpretations: &[AgentInterpretation],
    selected: usize,
) {
    ui.add_space(5.0);
    ui.heading(egui::RichText::new("Voice Command").color(egui::Color32::WHITE));
    ui.add_space(5.0);

    // Show transcript
    ui.label(
        egui::RichText::new(format!("\"{}\"", transcript))
            .size(14.0)
            .italics()
            .color(egui::Color32::LIGHT_GRAY),
    );
    ui.add_space(10.0);

    // Interpretation options
    egui::ScrollArea::vertical()
        .max_height(350.0)
        .show(ui, |ui| {
            for (i, interp) in interpretations.iter().enumerate() {
                let is_selected = i == selected;

                let bg_color = if is_selected {
                    egui::Color32::from_rgb(60, 80, 120)
                } else {
                    egui::Color32::from_rgb(40, 40, 50)
                };

                let text_color = if is_selected {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::LIGHT_GRAY
                };

                egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        // Header row with selection indicator, confidence, and explanation
                        ui.horizontal(|ui| {
                            // Selection indicator
                            if is_selected {
                                ui.label(egui::RichText::new("▶").color(egui::Color32::WHITE));
                            } else {
                                ui.add_space(14.0);
                            }

                            // Confidence percentage
                            let confidence_pct = (interp.confidence * 100.0) as u32;
                            let confidence_color = confidence_to_color(interp.confidence);
                            ui.label(
                                egui::RichText::new(format!("[{}%]", confidence_pct))
                                    .size(13.0)
                                    .color(confidence_color),
                            );

                            // Explanation as header
                            if !interp.explanation.is_empty() {
                                ui.label(
                                    egui::RichText::new(&interp.explanation)
                                        .size(14.0)
                                        .color(text_color),
                                );
                            }
                        });

                        // Script content (monospace, indented)
                        match &interp.action {
                            AgentAction::Script(script) => {
                                // Use indent + vertical layout for multiline script
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(28.0);
                                    // Show script in a code-style frame
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgb(25, 25, 35))
                                        .corner_radius(4.0)
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                for line in script.lines() {
                                                    ui.label(
                                                        egui::RichText::new(line)
                                                            .size(12.0)
                                                            .family(egui::FontFamily::Monospace)
                                                            .color(egui::Color32::from_rgb(180, 200, 180)),
                                                    );
                                                }
                                            });
                                        });
                                });
                            }
                            AgentAction::Cancel => {
                                ui.horizontal(|ui| {
                                    ui.add_space(28.0);
                                    ui.label(
                                        egui::RichText::new("(cancel)")
                                            .size(12.0)
                                            .italics()
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                            }
                        }
                    });

                ui.add_space(4.0);
            }
        });

    ui.add_space(10.0);

    // Joystick hints
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("↑↓ Navigate").color(egui::Color32::GRAY));
        ui.add_space(15.0);
        ui.label(egui::RichText::new("R3/A Select").color(egui::Color32::GREEN));
    });
}

fn confidence_to_color(confidence: f32) -> egui::Color32 {
    if confidence >= 0.9 {
        egui::Color32::from_rgb(100, 200, 100) // Green
    } else if confidence >= 0.7 {
        egui::Color32::from_rgb(200, 200, 100) // Yellow
    } else if confidence >= 0.5 {
        egui::Color32::from_rgb(200, 150, 100) // Orange
    } else {
        egui::Color32::from_rgb(200, 100, 100) // Red
    }
}


// ============================================================================
// Voice Settings Dialog
// ============================================================================

use crate::voice_command::{LlmModelInfo, ModelFetchState};

/// Actions returned from the voice settings dialog
#[derive(Debug, Clone)]
pub enum VoiceSettingsAction {
    None,
    Close,
    Save(VoiceCommandConfig),
    FetchModels,
}

impl PartialEq for VoiceSettingsAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VoiceSettingsAction::None, VoiceSettingsAction::None) => true,
            (VoiceSettingsAction::Close, VoiceSettingsAction::Close) => true,
            (VoiceSettingsAction::FetchModels, VoiceSettingsAction::FetchModels) => true,
            (VoiceSettingsAction::Save(a), VoiceSettingsAction::Save(b)) => a == b,
            _ => false,
        }
    }
}

/// Render the settings dialog
/// Returns the action to take
pub fn render_voice_settings_dialog(
    ctx: &egui::Context,
    config: &mut VoiceCommandConfig,
    model_state: &ModelFetchState,
    filter: &mut String,
) -> VoiceSettingsAction {
    let mut action = VoiceSettingsAction::None;

    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();

    egui::Area::new("settings_dialog".into())
        .fixed_pos(egui::pos2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Dark overlay background
            let painter = ui.painter();
            painter.rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            // Center the dialog
            let center = screen_rect.center();
            let panel_width = 600.0;
            let panel_height = 600.0;

            let panel_rect = egui::Rect::from_center_size(
                center,
                egui::vec2(panel_width, panel_height),
            );

            // Panel background
            painter.rect_filled(
                panel_rect,
                12.0,
                egui::Color32::from_rgb(30, 30, 40),
            );
            painter.rect_stroke(
                panel_rect,
                12.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 120)),
                egui::StrokeKind::Outside,
            );

            // Content area with header, scrollable body, and fixed footer
            let content_rect = panel_rect.shrink(20.0);
            let header_height = 50.0;
            let footer_height = 50.0;

            // Header area
            let header_rect = egui::Rect::from_min_size(
                content_rect.min,
                egui::vec2(content_rect.width(), header_height),
            );
            let header_builder = egui::UiBuilder::new().max_rect(header_rect);

            #[allow(deprecated)]
            ui.allocate_new_ui(header_builder, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("Settings")
                            .color(egui::Color32::WHITE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            action = VoiceSettingsAction::Close;
                        }
                    });
                });
                ui.separator();
            });

            // Footer area (fixed at bottom)
            let footer_rect = egui::Rect::from_min_size(
                egui::pos2(content_rect.min.x, content_rect.max.y - footer_height),
                egui::vec2(content_rect.width(), footer_height),
            );
            let footer_builder = egui::UiBuilder::new().max_rect(footer_rect);

            #[allow(deprecated)]
            ui.allocate_new_ui(footer_builder, |ui| {
                ui.separator();
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("Save")
                                .color(egui::Color32::WHITE),
                        )
                        .clicked()
                    {
                        action = VoiceSettingsAction::Save(config.clone());
                    }
                    ui.add_space(10.0);
                    if ui.button("Cancel").clicked() {
                        action = VoiceSettingsAction::Close;
                    }
                });
            });

            // Scrollable body area (between header and footer)
            let body_rect = egui::Rect::from_min_max(
                egui::pos2(content_rect.min.x, content_rect.min.y + header_height),
                egui::pos2(content_rect.max.x, content_rect.max.y - footer_height),
            );
            let body_builder = egui::UiBuilder::new().max_rect(body_rect);

            #[allow(deprecated)]
            ui.allocate_new_ui(body_builder, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(10.0);

                    // API Keys Section
                    ui.label(
                        egui::RichText::new("API Keys")
                            .color(egui::Color32::WHITE)
                            .strong()
                            .size(16.0),
                    );
                    ui.add_space(10.0);

                    // Requesty API Key
                    ui.label(
                        egui::RichText::new("Requesty.ai:")
                            .color(egui::Color32::GRAY),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut config.requesty_api_key)
                            .password(true)
                            .desired_width(400.0)
                            .hint_text("Enter Requesty API key...")
                    );
                    ui.label(
                        egui::RichText::new("Used for LLM voice command interpretation")
                            .size(11.0)
                            .color(egui::Color32::DARK_GRAY),
                    );
                    ui.add_space(10.0);

                    // Soniox API Key
                    ui.label(
                        egui::RichText::new("Soniox:")
                            .color(egui::Color32::GRAY),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut config.soniox_api_key)
                            .password(true)
                            .desired_width(400.0)
                            .hint_text("Enter Soniox API key...")
                    );
                    ui.label(
                        egui::RichText::new("Used for real-time speech-to-text")
                            .size(11.0)
                            .color(egui::Color32::DARK_GRAY),
                    );
                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // LLM Model Section
                    ui.label(
                        egui::RichText::new("LLM Model")
                            .color(egui::Color32::WHITE)
                            .strong()
                            .size(16.0),
                    );
                    ui.add_space(5.0);

                    // Current model display
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Current:")
                                .color(egui::Color32::GRAY),
                        );
                        ui.label(
                            egui::RichText::new(&config.model)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        );
                    });
                    ui.add_space(10.0);

                    // Search filter and refresh
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Filter:").color(egui::Color32::GRAY));
                        ui.add(
                            egui::TextEdit::singleline(filter)
                                .desired_width(200.0)
                                .hint_text("Search models...")
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Refresh").clicked() {
                                action = VoiceSettingsAction::FetchModels;
                            }
                        });
                    });
                    ui.add_space(5.0);

                    // Model list
                    if model_state.loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new("Loading models from Requesty.ai...")
                                    .color(egui::Color32::LIGHT_GRAY),
                            );
                        });
                    } else if let Some(ref error) = model_state.error {
                        ui.label(
                            egui::RichText::new(format!("Error: {}", error))
                                .color(egui::Color32::from_rgb(255, 100, 100)),
                        );
                        if ui.button("Retry").clicked() {
                            action = VoiceSettingsAction::FetchModels;
                        }
                    } else if model_state.models.is_empty() {
                        ui.label(
                            egui::RichText::new("No models loaded. Click Refresh to fetch.")
                                .color(egui::Color32::YELLOW),
                        );
                    } else {
                        // Filter models
                        let filter_lower = filter.to_lowercase();
                        let filtered_models: Vec<&LlmModelInfo> = model_state.models.iter()
                            .filter(|m| {
                                filter_lower.is_empty() ||
                                m.id.to_lowercase().contains(&filter_lower) ||
                                m.owned_by.to_lowercase().contains(&filter_lower)
                            })
                            .collect();

                        ui.label(
                            egui::RichText::new(format!("{} models", filtered_models.len()))
                                .size(12.0)
                                .color(egui::Color32::GRAY),
                        );

                        for model in filtered_models {
                            let is_selected = config.model == model.id;

                            // Create a selectable row
                            let text = if model.owned_by.is_empty() {
                                model.id.clone()
                            } else {
                                format!("{} ({})", model.id, model.owned_by)
                            };

                            if ui.selectable_label(is_selected, &text).clicked() {
                                config.model = model.id.clone();
                            }
                        }
                    }
                    ui.add_space(10.0);
                });
            });
        });

    action
}
