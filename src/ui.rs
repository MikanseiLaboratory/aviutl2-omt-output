use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aviutl2_eframe::eframe;
use aviutl2_eframe::egui::{self, Color32, ColorImage, Slider, TextureHandle, TextureOptions};

use crate::config::{
    ColorSpaceSetting, MAX_QUEUE_DEPTH, MIN_QUEUE_DEPTH, PLUGIN_DISPLAY_NAME, PluginConfig,
    QualitySetting, SCENE_HINT,
};
use crate::controller::LiveController;
use crate::media::format_bytes;
use crate::player::Transport;

pub struct UiApp {
    controller: Arc<LiveController>,
    draft: PluginConfig,
    config_epoch: u64,
    preview: Option<TextureHandle>,
    preview_generation: u64,
}

impl UiApp {
    pub fn new(cc: &eframe::CreationContext<'_>, controller: Arc<LiveController>) -> Self {
        cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());
        cc.egui_ctx.set_visuals(aviutl2_eframe::aviutl2_visuals());
        let draft = controller.shared().config_snapshot();
        Self {
            controller,
            draft,
            config_epoch: 0,
            preview: None,
            preview_generation: 0,
        }
    }
}

impl eframe::App for UiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        let player = Arc::clone(self.controller.player());
        let shared = Arc::clone(self.controller.shared());
        let transport = player.transport();
        let baking = transport == Transport::Baking;
        let sending = player.sending();
        if !sending && !baking {
            let epoch = shared.config_epoch.load(Ordering::Acquire);
            if epoch != self.config_epoch {
                self.draft = shared.config_snapshot();
                self.config_epoch = epoch;
            }
            self.controller.refresh_scene_name();
        }

        let scene = player.scene_snapshot();
        let status = shared.status_snapshot();
        let estimate = scene.estimated_bytes();
        let (baked_n, baked_total) = player.bake_progress();
        let play_index = player.current_index();
        let cue_enabled = player.cue_enabled();

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(PLUGIN_DISPLAY_NAME);
                ui.separator();
                ui.label(SCENE_HINT);
                ui.label(format!("現在シーン: {}", scene.name));
                ui.label(format!(
                    "解像度: {}x{}    fps: {}    フレーム数: {}",
                    scene.width,
                    scene.height,
                    scene.fps_label(),
                    scene.frame_count
                ));
                ui.label(format!("メモリ概算: {}", format_bytes(estimate)));

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!baking, egui::Button::new("描画開始"))
                        .clicked()
                    {
                        shared.store_draft(self.draft.clone().clamped());
                        self.controller.start_bake();
                    }
                    if ui
                        .add_enabled(baking, egui::Button::new("キャンセル"))
                        .clicked()
                    {
                        self.controller.cancel_bake();
                    }
                    let cue = if cue_enabled {
                        egui::Button::new("CUE").fill(Color32::from_rgb(196, 48, 48))
                    } else {
                        egui::Button::new("CUE").fill(Color32::from_rgb(70, 70, 70))
                    };
                    if ui.add_enabled(cue_enabled, cue).clicked() {
                        self.controller.cue();
                    }
                    if ui
                        .add_enabled(sending || baking, egui::Button::new("送出停止"))
                        .clicked()
                    {
                        self.controller.stop();
                    }
                });

                ui.label(match transport {
                    Transport::Idle => "状態: 待機".to_string(),
                    Transport::Baking => format!("状態: 描画中 {baked_n} / {baked_total}"),
                    Transport::HoldFirst => "状態: 先頭フレーム固定".to_string(),
                    Transport::Playing => format!("状態: 再生中 フレーム {play_index}"),
                    Transport::HoldLast => "状態: 最終フレーム固定".to_string(),
                });

                ui.separator();
                ui.add_enabled_ui(!sending && !baking, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("ソース名");
                        ui.text_edit_singleline(&mut self.draft.source_name);
                    });
                    ui.checkbox(&mut self.draft.send_video, "映像を送信");
                    ui.checkbox(&mut self.draft.send_audio, "音声を送信");
                    ui.horizontal(|ui| {
                        ui.label("品質");
                        egui::ComboBox::from_id_salt("omt_quality")
                            .selected_text(self.draft.quality.as_label())
                            .show_ui(ui, |ui| {
                                for item in QualitySetting::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.quality,
                                        item,
                                        item.as_label(),
                                    );
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("色空間");
                        egui::ComboBox::from_id_salt("omt_colorspace")
                            .selected_text(self.draft.color_space.as_label())
                            .show_ui(ui, |ui| {
                                for item in ColorSpaceSetting::ALL {
                                    ui.selectable_value(
                                        &mut self.draft.color_space,
                                        item,
                                        item.as_label(),
                                    );
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("送信キュー深度");
                        ui.add(Slider::new(
                            &mut self.draft.send_queue_depth,
                            MIN_QUEUE_DEPTH..=MAX_QUEUE_DEPTH,
                        ));
                    });
                });

                ui.separator();
                ui.label(format!(
                    "接続数: {}    ポート: {}",
                    status.connections, status.port
                ));
                ui.label(format!(
                    "映像購読: {}    音声購読: {}",
                    bool_jp(status.video_subscribed),
                    bool_jp(status.audio_subscribed)
                ));
                ui.label(format!(
                    "送信 fps: {:.1}    エンコード: {} µs",
                    status.send_fps, status.encode_us
                ));
                ui.label(format!(
                    "映像 drop: {}    音声 drop: {}    送信 drop: {}",
                    status.video_drops, status.audio_drops, status.sender_drops
                ));
                ui.label(format!(
                    "直近エラー: {}",
                    status.last_error.as_deref().unwrap_or("なし")
                ));

                ui.separator();
                ui.label("プレビュー");
                self.show_preview(ui, play_index);
            });
        });

        if !sending && !baking {
            shared.store_draft(self.draft.clone());
        }
    }
}

impl UiApp {
    fn show_preview(&mut self, ui: &mut egui::Ui, play_index: usize) {
        let Some(frame) = self.controller.player().preview_snapshot() else {
            self.preview = None;
            self.preview_generation = 0;
            ui.label("なし");
            return;
        };
        if self.preview_generation != frame.generation {
            let image = ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            self.preview = Some(ui.ctx().load_texture(
                "omt-preview",
                image,
                TextureOptions::NEAREST,
            ));
            self.preview_generation = frame.generation;
        }
        if let Some(texture) = &self.preview {
            let size = texture.size_vec2();
            ui.image((texture.id(), size));
            ui.label(format!("フレーム {play_index}"));
        } else {
            ui.label("なし");
        }
    }
}

fn bool_jp(value: bool) -> &'static str {
    if value { "はい" } else { "いいえ" }
}
