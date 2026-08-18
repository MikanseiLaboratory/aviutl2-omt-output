use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use aviutl2::AnyResult;
use aviutl2::tracing_subscriber::prelude::*;

use crate::config::{
    PLUGIN_AUTHOR_EN, PLUGIN_AUTHOR_JA, PLUGIN_DISPLAY_NAME, PROJECT_CONFIG_KEY, PluginConfig,
};
use crate::controller::{LiveController, SharedState};

pub mod bake;
pub mod config;
pub mod controller;
pub mod media;
mod omt_file_log;
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
    omt_file_log::init();
    let max_level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    let _ = aviutl2::tracing_subscriber::registry()
        .with(
            aviutl2::tracing_subscriber::fmt::layer()
                .event_format(aviutl2::logger::AviUtl2Formatter)
                .with_writer(aviutl2::logger::AviUtl2LogWriter)
                .with_filter(
                    aviutl2::tracing_subscriber::filter::LevelFilter::from_level(max_level),
                ),
        )
        .with(OmtFileLayer)
        .try_init();
}

struct OmtFileLayer;

impl<S> aviutl2::tracing_subscriber::layer::Layer<S> for OmtFileLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: aviutl2::tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut message = String::new();
        event.record(&mut MessageVisitor(&mut message));
        if message.is_empty() {
            message.push_str(event.metadata().name());
        }
        omt_file_log::write(&message, event.metadata().target());
    }
}

struct MessageVisitor<'a>(&'a mut String);

impl tracing::field::Visit for MessageVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        if field.name() == "message" {
            let _ = write!(self.0, "{value:?}");
        } else {
            let _ = write!(self.0, "{}={value:?}", field.name());
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            self.0.push_str(field.name());
            self.0.push('=');
            self.0.push_str(value);
        }
    }
}

aviutl2::register_generic_plugin!(OmtLivePlugin);
