use config::{Config as ConfigLoader, File, FileFormat};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub input_video_path: String,
    pub output_video_path: String,
    pub ffmpeg_path: String,
    pub use_gpu: bool,
    pub video_bitrate: String,
    pub crf: Option<String>,
    pub upscale_resolution: Option<String>,
    pub background_audio_path: Option<String>,
    pub audio_start_time: f64,
    pub replace_audio: bool,
    pub original_audio_volume: f64,
    pub background_audio_volume: f64,
    pub clip_start_time: Option<String>,
    pub clip_end_time: Option<String>,
    pub video_speed: f64,
    pub advanced_log: bool,
    pub fade_in_duration: Option<f64>,
    pub fade_out_duration: Option<f64>,
}

impl Config {
    pub fn from_file(file_path: &str) -> Result<Self, config::ConfigError> {
        let config_loader = ConfigLoader::builder().add_source(File::new(file_path, FileFormat::Toml)).build()?;
        config_loader.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        let required_keys = vec![
            &self.settings.input_video_path,
            &self.settings.output_video_path,
            &self.settings.ffmpeg_path,
            &self.settings.video_bitrate,
        ];

        for key in required_keys {
            if key.is_empty() {
                return Err(format!("Missing required config key: {}", key));
            }
        }

        Ok(())
    }
}
