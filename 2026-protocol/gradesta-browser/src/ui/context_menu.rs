//! Context menu system for gamepad
//!
//! Shows unbound gamepad commands organized by context in a sprawling 2D grid.
//! All commands are visible at once - categories sprawl out from the center.
//! Square button opens the menu, right stick navigates, R3 selects.

use std::time::{Duration, Instant};

use bevy_egui::egui::{self, Color32, FontId, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};

use crate::commands::{Command, Context as CmdContext};
use crate::keybindings::KeybindingResolver;
use crate::state::{AppState, ContextMenuGrid, ContextMenuItem, ContextMenuItemType};

/// Navigation cooldown to prevent rapid movement
const NAV_COOLDOWN: Duration = Duration::from_millis(200);

/// Cell size for menu items
const CELL_WIDTH: f32 = 180.0;
const CELL_HEIGHT: f32 = 50.0;
const CELL_PADDING: f32 = 8.0;

/// Get unbound commands for a context (commands without gamepad bindings)
fn get_unbound_commands(context: CmdContext, keybindings: &KeybindingResolver) -> Vec<Command> {
    Command::all()
        .into_iter()
        .filter(|cmd| cmd.context() == context)
        .filter(|cmd| keybindings.get_gamepad_bindings(cmd).is_empty())
        .collect()
}

/// Place a category's commands sprawling out from the category position
fn place_category_with_commands(
    grid: &mut ContextMenuGrid,
    commands: &[Command],
    start_pos: (i32, i32),
    direction: (i32, i32),
) {
    for (i, cmd) in commands.iter().enumerate() {
        let pos = (
            start_pos.0 + direction.0 * i as i32,
            start_pos.1 + direction.1 * i as i32,
        );
        grid.items.insert(pos, ContextMenuItem {
            label: cmd.description().to_string(),
            item_type: ContextMenuItemType::Command(cmd.clone()),
        });
    }
}

/// Build the sprawling context menu grid
/// Categories stacked vertically, commands extend horizontally to the right
/// All navigation is horizontal/vertical only
pub fn build_sprawling_menu_grid(keybindings: &KeybindingResolver) -> ContextMenuGrid {
    let mut grid = ContextMenuGrid::default();

    // Layout: Categories in column 0, commands extend right (positive x)
    // Row 0: Help (center)
    // Rows above (negative y): Global, Graph, Recording, Elf
    // Rows below (positive y): Bag, TextInput, Export, Auth

    // Row 0: Controller Help
    grid.items.insert((0, 0), ContextMenuItem {
        label: "Controller Help".to_string(),
        item_type: ContextMenuItemType::Help,
    });

    // Row -1: Global commands
    let global_cmds = get_unbound_commands(CmdContext::Global, keybindings);
    place_category_with_commands(&mut grid, &global_cmds, (0, -1), (1, 0));

    // Row -2: Graph commands
    let graph_cmds = get_unbound_commands(CmdContext::Graph, keybindings);
    place_category_with_commands(&mut grid, &graph_cmds, (0, -2), (1, 0));

    // Row -3: Recording commands
    let recording_cmds = get_unbound_commands(CmdContext::Recording, keybindings);
    place_category_with_commands(&mut grid, &recording_cmds, (0, -3), (1, 0));

    // Row -4: Elf commands
    let elf_cmds = get_unbound_commands(CmdContext::Elf, keybindings);
    place_category_with_commands(&mut grid, &elf_cmds, (0, -4), (1, 0));

    // Row 1: Bag commands
    let bag_cmds = get_unbound_commands(CmdContext::Bag, keybindings);
    place_category_with_commands(&mut grid, &bag_cmds, (0, 1), (1, 0));

    // Row 2: TextInput commands
    let text_cmds = get_unbound_commands(CmdContext::TextInput, keybindings);
    place_category_with_commands(&mut grid, &text_cmds, (0, 2), (1, 0));

    // Row 3: Export commands
    let export_cmds = get_unbound_commands(CmdContext::Export, keybindings);
    place_category_with_commands(&mut grid, &export_cmds, (0, 3), (1, 0));

    // Row 4: Auth commands
    let auth_cmds = get_unbound_commands(CmdContext::Authentication, keybindings);
    place_category_with_commands(&mut grid, &auth_cmds, (0, 4), (1, 0));

    grid.recalculate_bounds();
    grid
}

/// Handle context menu navigation
/// Returns Some(Command) if a command was selected
pub fn handle_context_menu_navigation(
    app_state: &mut AppState,
    nav_up: bool,
    nav_down: bool,
    nav_left: bool,
    nav_right: bool,
    select: bool,
    back: bool,
) -> Option<Command> {
    if !app_state.context_menu.open {
        return None;
    }

    // Check navigation cooldown
    let can_navigate = app_state.context_menu.last_nav_time
        .map(|t| t.elapsed() >= NAV_COOLDOWN)
        .unwrap_or(true);

    // Copy grid bounds to avoid borrow issues
    let (x, y) = app_state.context_menu.current_position;
    let (min_x, max_x) = (app_state.context_menu.grid.min_x, app_state.context_menu.grid.max_x);
    let (min_y, max_y) = (app_state.context_menu.grid.min_y, app_state.context_menu.grid.max_y);

    // Handle navigation - simple grid movement, no submenu logic
    if can_navigate {
        let mut moved = false;

        if nav_up && y > min_y {
            // Find valid cell moving up
            for new_y in (min_y..y).rev() {
                if app_state.context_menu.grid.items.contains_key(&(x, new_y)) {
                    app_state.context_menu.current_position.1 = new_y;
                    moved = true;
                    break;
                }
            }
        }
        if nav_down && y < max_y {
            // Find valid cell moving down
            for new_y in (y + 1)..=max_y {
                if app_state.context_menu.grid.items.contains_key(&(x, new_y)) {
                    app_state.context_menu.current_position.1 = new_y;
                    moved = true;
                    break;
                }
            }
        }
        if nav_left && x > min_x {
            // Find valid cell moving left
            for new_x in (min_x..x).rev() {
                if app_state.context_menu.grid.items.contains_key(&(new_x, y)) {
                    app_state.context_menu.current_position.0 = new_x;
                    moved = true;
                    break;
                }
            }
        }
        if nav_right && x < max_x {
            // Find valid cell moving right
            for new_x in (x + 1)..=max_x {
                if app_state.context_menu.grid.items.contains_key(&(new_x, y)) {
                    app_state.context_menu.current_position.0 = new_x;
                    moved = true;
                    break;
                }
            }
        }

        if moved {
            app_state.context_menu.last_nav_time = Some(Instant::now());
        }
    }

    // Handle selection (R3) - get item type without long-lived borrow
    if select {
        let (x, y) = app_state.context_menu.current_position;
        let item_type = app_state.context_menu.grid.items.get(&(x, y)).map(|item| item.item_type.clone());

        if let Some(item_type) = item_type {
            match item_type {
                ContextMenuItemType::Command(cmd) => {
                    // Execute command and close menu
                    app_state.context_menu.open = false;
                    return Some(cmd);
                }
                ContextMenuItemType::Submenu(_) => {
                    // Submenus are no longer used - this case shouldn't happen
                }
                ContextMenuItemType::Help => {
                    // Toggle gamepad help and close menu
                    app_state.show_gamepad_help = !app_state.show_gamepad_help;
                    app_state.context_menu.open = false;
                }
            }
        }
    }

    // Handle back - just close menu (no submenu stack)
    if back {
        app_state.context_menu.open = false;
    }

    None
}

/// Open the context menu
pub fn open_context_menu(app_state: &mut AppState) {
    app_state.context_menu.open = true;
    app_state.context_menu.grid = build_sprawling_menu_grid(&app_state.keybindings);
    app_state.context_menu.current_position = (0, 0);
    app_state.context_menu.last_nav_time = None;
}

/// Get cell rect centered on the current selection
fn get_cell_rect_centered(center: Pos2, current: (i32, i32), x: i32, y: i32) -> Rect {
    // Calculate offset from current selection (current is at center)
    let offset_x = (x - current.0) as f32 * (CELL_WIDTH + CELL_PADDING);
    let offset_y = (y - current.1) as f32 * (CELL_HEIGHT + CELL_PADDING);

    Rect::from_center_size(
        Pos2::new(center.x + offset_x, center.y + offset_y),
        Vec2::new(CELL_WIDTH, CELL_HEIGHT),
    )
}

/// Draw edges connecting adjacent cells (centered view)
fn draw_edges_centered(painter: &egui::Painter, grid: &ContextMenuGrid, center: Pos2, current: (i32, i32)) {
    let edge_color = Color32::from_rgb(60, 80, 100);

    for ((x, y), _) in &grid.items {
        let cell_rect = get_cell_rect_centered(center, current, *x, *y);

        // Check for neighbor to the right (horizontal)
        if grid.items.contains_key(&(x + 1, *y)) {
            let neighbor_rect = get_cell_rect_centered(center, current, x + 1, *y);
            painter.line_segment(
                [cell_rect.right_center(), neighbor_rect.left_center()],
                Stroke::new(2.0, edge_color),
            );
        }

        // Check for neighbor below (vertical)
        if grid.items.contains_key(&(*x, y + 1)) {
            let neighbor_rect = get_cell_rect_centered(center, current, *x, y + 1);
            painter.line_segment(
                [cell_rect.center_bottom(), neighbor_rect.center_top()],
                Stroke::new(2.0, edge_color),
            );
        }
    }
}

/// Render the context menu overlay
pub fn render_context_menu(ctx: &egui::Context, app_state: &AppState) {
    if !app_state.context_menu.open {
        return;
    }

    let grid = &app_state.context_menu.grid;
    let current = app_state.context_menu.current_position;

    // Semi-transparent background overlay
    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();

    egui::Area::new(egui::Id::new("context_menu_overlay"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Draw dark overlay background
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            );

            let painter = ui.painter().clone();
            let screen_center = screen_rect.center();

            // The grid center point (where current selection will be drawn)
            let grid_center = screen_center;

            // Draw edges first (behind cells)
            draw_edges_centered(&painter, grid, grid_center, current);

            // Draw cells on top
            for ((x, y), item) in &grid.items {
                let cell_rect = get_cell_rect_centered(grid_center, current, *x, *y);

                let is_selected = (*x, *y) == current;
                let bg_color = if is_selected {
                    Color32::from_rgb(60, 100, 180)
                } else {
                    Color32::from_rgb(45, 45, 50)
                };
                let border_color = if is_selected {
                    Color32::from_rgb(100, 160, 255)
                } else {
                    Color32::from_rgb(60, 60, 65)
                };

                // Cell background
                painter.rect_filled(cell_rect, 4.0, bg_color);
                painter.rect_stroke(cell_rect, 4.0, Stroke::new(2.0, border_color), StrokeKind::Outside);

                // Item icon/indicator
                let icon = match &item.item_type {
                    ContextMenuItemType::Command(_) => "⚡",
                    ContextMenuItemType::Submenu(_) => "▶",
                    ContextMenuItemType::Help => "?",
                };

                // Icon
                painter.text(
                    Pos2::new(cell_rect.left() + 10.0, cell_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    icon,
                    FontId::proportional(16.0),
                    Color32::WHITE,
                );

                // Label
                let label = truncate_label(&item.label, 18);
                painter.text(
                    Pos2::new(cell_rect.left() + 28.0, cell_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }

            // Instructions at bottom of screen
            let instructions_rect = Rect::from_min_size(
                Pos2::new(screen_rect.left() + 20.0, screen_rect.bottom() - 40.0),
                Vec2::new(screen_rect.width() - 40.0, 30.0),
            );

            painter.text(
                instructions_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Right Stick: Navigate  •  R3: Select  •  ○: Close",
                FontId::proportional(12.0),
                Color32::GRAY,
            );
        });
}

/// Truncate a label to fit in the cell
fn truncate_label(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", &s[..max_chars - 3])
    }
}
