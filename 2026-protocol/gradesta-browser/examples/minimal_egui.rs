//! Minimal bevy_egui test - just a text box and button
//! Run with: cargo run --example minimal_egui

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

fn main() {
    eprintln!("Starting minimal egui test...");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Minimal egui test".to_string(),
                resolution: bevy::window::WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .init_resource::<TestState>()
        .add_systems(Startup, setup)
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    eprintln!("Camera2d spawned");
}

#[derive(Resource, Default)]
struct TestState {
    text_input: String,
    click_count: u32,
}

fn ui_system(mut contexts: EguiContexts, mut state: ResMut<TestState>) -> Result {
    let ctx = contexts.ctx_mut()?;

    static FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if frame == 0 {
        eprintln!("First UI frame");
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Minimal bevy_egui Test");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.label("Text input:");
            ui.text_edit_singleline(&mut state.text_input);
        });

        ui.add_space(10.0);

        if ui.button("Click me!").clicked() {
            state.click_count += 1;
            eprintln!("Button clicked! Count: {}", state.click_count);
        }

        ui.add_space(10.0);
        ui.label(format!("Click count: {}", state.click_count));
    });

    Ok(())
}
