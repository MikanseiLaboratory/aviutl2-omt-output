use serde::{Deserialize, Serialize};

use openmediatransport::{ColorSpace, Quality};

pub const PLUGIN_DISPLAY_NAME: &str = "AviUtl2 OMT Live Output";
pub const PLUGIN_AUTHOR_JA: &str = "未完成成果物研究所";
pub const PLUGIN_AUTHOR_EN: &str = "Mikansei Laboratory";
pub const PROJECT_CONFIG_KEY: &str = "omt_live_config";
pub const DEFAULT_QUEUE_DEPTH: usize = 4;
pub const MIN_QUEUE_DEPTH: usize = 1;
pub const MAX_QUEUE_DEPTH: usize = 16;
pub const MAX_SOURCE_NAME_LEN: usize = 63;
pub const MAX_PREVIEW_WIDTH: u32 = 320;
pub const SCENE_HINT: &str =
    "出したいシーンを開いてから描画開始してください。描画中はそのシーンを編集しないでください。";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QualitySetting {
    #[default]
    Default,
    Low,
    Medium,
    High,
}

impl QualitySetting {
    pub const ALL: [Self; 4] = [Self::Default, Self::Low, Self::Medium, Self::High];

    pub fn as_label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }

    pub fn to_omt(self) -> Quality {
        match self {
            Self::Default => Quality::Default,
            Self::Low => Quality::Low,
            Self::Medium => Quality::Medium,
            Self::High => Quality::High,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColorSpaceSetting {
    Bt601,
    #[default]
    Bt709,
}

impl ColorSpaceSetting {
    pub const ALL: [Self; 2] = [Self::Bt601, Self::Bt709];

    pub fn as_label(self) -> &'static str {
        match self {
            Self::Bt601 => "BT.601",
            Self::Bt709 => "BT.709",
        }
    }

    pub fn to_omt(self) -> ColorSpace {
        match self {
            Self::Bt601 => ColorSpace::Bt601,
            Self::Bt709 => ColorSpace::Bt709,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_source_name")]
    pub source_name: String,
    #[serde(default = "default_true")]
    pub send_video: bool,
    #[serde(default = "default_true")]
    pub send_audio: bool,
    #[serde(default)]
    pub quality: QualitySetting,
    #[serde(default)]
    pub color_space: ColorSpaceSetting,
    #[serde(default = "default_queue_depth")]
    pub send_queue_depth: usize,
}

fn default_source_name() -> String {
    "AviUtl2".to_string()
}

fn default_true() -> bool {
    true
}

fn default_queue_depth() -> usize {
    DEFAULT_QUEUE_DEPTH
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            source_name: default_source_name(),
            send_video: true,
            send_audio: true,
            quality: QualitySetting::Default,
            color_space: ColorSpaceSetting::Bt709,
            send_queue_depth: DEFAULT_QUEUE_DEPTH,
        }
    }
}

impl PluginConfig {
    pub fn clamped(mut self) -> Self {
        self.send_queue_depth = self
            .send_queue_depth
            .clamp(MIN_QUEUE_DEPTH, MAX_QUEUE_DEPTH);
        if self.source_name.trim().is_empty() {
            self.source_name = default_source_name();
        }
        if self.source_name.chars().count() > MAX_SOURCE_NAME_LEN {
            self.source_name = self.source_name.chars().take(MAX_SOURCE_NAME_LEN).collect();
        }
        if !self.send_video && !self.send_audio {
            self.send_video = true;
        }
        self
    }

    pub fn frame_types(&self) -> openmediatransport::FrameType {
        let mut types = openmediatransport::FrameType::METADATA;
        if self.send_video {
            types |= openmediatransport::FrameType::VIDEO;
        }
        if self.send_audio {
            types |= openmediatransport::FrameType::AUDIO;
        }
        types
    }
}
