//! Bag (clipboard) panel - displays yanked vertices

use bevy_egui::egui;

/// Actions returned from the bag UI
pub enum BagAction {
    None,
    Close,
    ClearAll,
    JumpTo(u64),
    Remove(usize),
}

/// Render the bag (clipboard) panel
///
/// # Arguments
/// * `ui` - egui UI context
/// * `bag` - List of vertex IDs in the bag (newest last)
/// * `get_label` - Function to get a display label for a vertex ID
///
/// # Returns
/// Action to take (close, clear, jump, remove)
pub fn render_bag<F>(ui: &mut egui::Ui, bag: &[u64], get_label: F) -> BagAction
where
    F: Fn(u64) -> String,
{
    let mut action = BagAction::None;

    ui.heading("Bag (Clipboard)");
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Close (Escape)").clicked() {
            action = BagAction::Close;
        }
        if ui.button("Clear All").clicked() {
            action = BagAction::ClearAll;
        }
    });

    ui.separator();

    ui.label("Keyboard shortcuts:");
    ui.label("  Y = Yank current vertex to bag");
    ui.label("  Ctrl+Y = Pop from bag");
    ui.label("  G = Go to bag top");

    ui.separator();

    if bag.is_empty() {
        ui.label("Bag is empty. Press Y to yank current vertex.");
    } else {
        ui.label(format!("{} item(s) in bag:", bag.len()));
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                // Show items with newest (top of stack) first
                for (i, &vertex_id) in bag.iter().rev().enumerate() {
                    let stack_idx = bag.len() - 1 - i;
                    ui.horizontal(|ui| {
                        let is_top = i == 0;
                        let prefix = if is_top { "→ " } else { "  " };

                        let label = get_label(vertex_id);
                        ui.label(format!("{}{}", prefix, label));

                        if ui.small_button("Go").clicked() {
                            action = BagAction::JumpTo(vertex_id);
                        }
                        if ui.small_button("×").clicked() {
                            action = BagAction::Remove(stack_idx);
                        }
                    });
                }
            });
    }

    action
}
