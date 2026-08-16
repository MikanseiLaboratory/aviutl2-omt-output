use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use aviutl2::AnyResult;

use crate::config::{
    PLUGIN_AUTHOR_EN, PLUGIN_AUTHOR_JA, PLUGIN_DISPLAY_NAME, PROJECT_CONFIG_KEY, PluginConfig,
};
use crate::controller::{LiveController, SharedState};

pub mod bake;
pub mod config;
pub mod controller;
pub mod media;
pub mod player;
pub mod sender;
mod ui;

#[aviutl2::plugin(GenericPlugin)]
pub struct OmtLivePlugin {
    window: aviutl2_eframe::EframeWindow,
    controller: Arc<LiveController>,
}

impl aviutl2::generic::GenericPlugin for OmtLivePlugin {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        init_logging();
        tracing::info!("Initializing {PLUGIN_DISPLAY_NAME} ({PLUGIN_AUTHOR_EN})");

        let shared = Arc::new(SharedState::new());
        let controller = Arc::new(LiveController::new(shared));
        let ui_controller = Arc::clone(&controller);
        let window =
            aviutl2_eframe::EframeWindow::new("AviUtl2OmtLiveOutput", move |cc, _handle| {
                Ok(Box::new(ui::UiApp::new(cc, ui_controller)))
            })?;

        Ok(Self { window, controller })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: PLUGIN_DISPLAY_NAME.to_string(),
            information: format!(
                "今開いているシーンを OMT で送出 / {PLUGIN_AUTHOR_JA} / v{version}",
                version = env!("CARGO_PKG_VERSION")
            ),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        let handle = Arc::new(registry.create_edit_handle());
        self.controller.set_edit_handle(handle);
        if let Ok(handle) = self.window.handle() {
            let _ = registry.register_window_client(PLUGIN_DISPLAY_NAME, &handle);
        }
    }

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile<'_>) {
        self.controller.stop();
        if let Ok(config) = project.deserialize::<PluginConfig>(PROJECT_CONFIG_KEY) {
            self.controller.shared().set_config(config.clamped());
        }
        self.controller.mark_scene_dirty();
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile<'_>) {
        let config = self.controller.shared().config_snapshot();
        if let Err(e) = project.serialize(PROJECT_CONFIG_KEY, &config) {
            tracing::warn!("failed to save live config: {e}");
        }
    }

    fn event_change_edit_frame(&mut self) {
        request_ui_repaint(&self.window);
    }

    fn event_change_scene_info(&mut self) {
        self.controller.mark_scene_dirty();
        request_ui_repaint(&self.window);
    }
}

impl Drop for OmtLivePlugin {
    fn drop(&mut self) {
        self.controller.stop();
    }
}

fn request_ui_repaint(window: &aviutl2_eframe::EframeWindow) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Ok(ctx) = window.egui_ctx() {
            ctx.request_repaint();
        }
    }));
}

fn init_logging() {
    let _ = aviutl2::tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .event_format(aviutl2::logger::AviUtl2Formatter)
        .with_writer(aviutl2::logger::AviUtl2LogWriter)
        .try_init();
}

aviutl2::register_generic_plugin!(OmtLivePlugin);
