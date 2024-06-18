use log::info;
use regex::Regex;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use indicatif::{ProgressBar, ProgressStyle};

use crate::util::config::{Config, Settings};

fn parse_ffmpeg_progress(line: &str) -> Option<f64> {
    let re = Regex::new(r"time=(\d+):(\d+):(\d+\.\d+)").unwrap();
    re.captures(line).and_then(|caps| {
        let hours: f64 = caps[1].parse().ok()?;
        let minutes: f64 = caps[2].parse().ok()?;
        let seconds: f64 = caps[3].parse().ok()?;
        Some(hours * 3600.0 + minutes * 60.0 + seconds)
    })
}

fn run_ffmpeg_command(ffmpeg_command: &[String], duration: f64, advanced_log: bool) -> Result<(), String> {
    let mut command = Command::new(&ffmpeg_command[0]);
    command.args(&ffmpeg_command[1..]);

    if advanced_log {
        let status = command.status().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("FFmpeg command failed with status: {}", status));
        }
    } else {
        command.stderr(Stdio::piped()).stdout(Stdio::null());
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
        let reader = BufReader::new(stderr);

        let progress = ProgressBar::new(duration as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% ({pos}/{len}, {eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        for line in reader.lines() {
            if let Ok(line) = line {
                if let Some(elapsed) = parse_ffmpeg_progress(&line) {
                    progress.set_position(elapsed as u64);
                }
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("FFmpeg command failed with status: {}", status));
        }
    }

    Ok(())
}

pub fn add_fade_effects(config: &Config) -> Result<(), String> {
    config.validate()?;

    let Settings {
        input_video_path,
        output_video_path,
        ffmpeg_path,
        use_gpu,
        video_bitrate,
        crf,
        upscale_resolution,
        background_audio_path,
        audio_start_time,
        replace_audio,
        original_audio_volume,
        background_audio_volume,
        clip_start_time,
        clip_end_time,
        video_speed,
        advanced_log,
        fade_in_duration,
        fade_out_duration,
    } = &config.settings;

    let probe_command = Command::new(ffmpeg_path)
        .arg("-i")
        .arg(input_video_path)
        .arg("-hide_banner")
        .output()
        .map_err(|e| e.to_string())?;
    let output = String::from_utf8_lossy(&probe_command.stderr);

    let duration = output
        .lines()
        .find(|line| line.contains("Duration"))
        .and_then(|line| {
            let duration_str = line.split("Duration: ").nth(1)?.split(',').next()?;
            let mut parts = duration_str.split(':');
            let h: f64 = parts.next()?.parse().ok()?;
            let m: f64 = parts.next()?.parse().ok()?;
            let s: f64 = parts.next()?.parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        })
        .ok_or("Could not determine video duration")?;

    let framerate = output
        .lines()
        .find(|line| line.contains("Stream") && line.contains("Video"))
        .and_then(|line| {
            let fps_str = line.split("fps").next()?.split_whitespace().last()?;
            fps_str.parse::<f64>().ok()
        })
        .ok_or("Could not determine video framerate")?;

    let fade_in_duration = fade_in_duration.unwrap_or(3.0);
    let fade_out_duration = fade_out_duration.unwrap_or(3.0);

    let clip_start_time_float = if let Some(ref clip_start_time) = clip_start_time {
        if clip_start_time.to_lowercase() == "none" {
            0.0
        } else {
            clip_start_time.parse::<f64>().map_err(|_| "Invalid clip_start_time")?
        }
    } else {
        0.0
    };

    let clip_end_time_float = if let Some(ref clip_end_time) = clip_end_time {
        if clip_end_time.to_lowercase() == "none" {
            duration
        } else {
            clip_end_time.parse::<f64>().map_err(|_| "Invalid clip_end_time")?
        }
    } else {
        duration
    };

    let fade_out_start_time = clip_end_time_float - fade_out_duration;

    let mut video_filters = vec![format!(
        "fade=t=in:st=0:d={},fade=t=out:st={}:d={}",
        fade_in_duration, fade_out_start_time, fade_out_duration
    )];

    if let Some(ref resolution) = upscale_resolution {
        if resolution.to_lowercase() != "none" {
            video_filters.push(format!("scale={}", resolution));
        }
    }
    if *video_speed != 1.0 {
        video_filters.push(format!("setpts={}*PTS", 1.0 / video_speed));
    }

    let video_filter_str = video_filters.join(",");

    let mut audio_filters = vec![format!(
        "afade=t=in:st=0:d={},afade=t=out:st={}:d={}",
        fade_in_duration, fade_out_start_time, fade_out_duration
    )];
    if *video_speed != 1.0 {
        audio_filters.push(format!("atempo={}", video_speed));
    }

    let audio_filter_str = audio_filters.join(",");

    let video_codec = if *use_gpu { "hevc_nvenc" } else { "libx265" };

    let mut ffmpeg_command = vec![ffmpeg_path.clone(), "-i".to_string(), input_video_path.clone()];

    if clip_start_time_float > 0.0 {
        ffmpeg_command.extend(vec!["-ss".to_string(), clip_start_time_float.to_string()]);
    }
    if clip_end_time_float < duration {
        ffmpeg_command.extend(vec!["-to".to_string(), clip_end_time_float.to_string()]);
    }

    if let Some(ref audio_path) = background_audio_path {
        if audio_path.to_lowercase() != "none" {
            ffmpeg_command.extend(vec![
                "-ss".to_string(),
                audio_start_time.to_string(),
                "-i".to_string(),
                audio_path.clone(),
            ]);
        }
    }

    if video_filter_str.is_empty() {
        ffmpeg_command.extend(vec![
            "-c:v".to_string(),
            "copy".to_string()
        ]);
    } else {
        ffmpeg_command.extend(vec![
            "-filter_complex".to_string(),
            format!("[0:v]{}[v]", video_filter_str),
            "-map".to_string(),
            "[v]".to_string(),
        ]);

        if *video_speed != 1.0 {
            ffmpeg_command.extend(vec![
                "-r".to_string(),
                (framerate * video_speed).to_string(),
            ]);
        }

        ffmpeg_command.extend(vec![
            "-c:v".to_string(),
            video_codec.to_string(),
        ]);

        if let Some(ref crf_value) = crf {
            if crf_value.to_lowercase() != "none" && !use_gpu {
                ffmpeg_command.extend(vec![
                    "-crf".to_string(),
                    crf_value.to_string(),
                ]);
            } else {
                ffmpeg_command.extend(vec![
                    "-b:v".to_string(),
                    video_bitrate.clone(),
                ]);
            }
        } else {
            ffmpeg_command.extend(vec![
                "-b:v".to_string(),
                video_bitrate.clone(),
            ]);
        }
    }

    if let Some(ref audio_path) = background_audio_path {
        if audio_path.to_lowercase() != "none" {
            if *replace_audio {
                ffmpeg_command.extend(vec![
                    "-filter_complex".to_string(),
                    format!(
                        "[1:a]volume={},{}[a]",
                        background_audio_volume, audio_filter_str
                    ),
                    "-map".to_string(),
                    "[a]".to_string(),
                ]);
            } else {
                let normalize_filter = format!(
                    "[0:a]volume={}[a0];[1:a]volume={},{}[a1];[a0][a1]amix=inputs=2:duration=first:dropout_transition=3[a]",
                    original_audio_volume, background_audio_volume, audio_filter_str
                );
                ffmpeg_command.extend(vec![
                    "-filter_complex".to_string(),
                    normalize_filter,
                    "-map".to_string(),
                    "[a]".to_string(),
                ]);
            }
        } else {
            ffmpeg_command.extend(vec![
                "-filter_complex".to_string(),
                format!("[0:a]volume={}{}", original_audio_volume, audio_filter_str),
                "-map".to_string(),
                "[a]".to_string(),
            ]);
        }
    } else {
        ffmpeg_command.extend(vec![
            "-filter_complex".to_string(),
            format!("[0:a]volume={}{}", original_audio_volume, audio_filter_str),
            "-map".to_string(),
            "[a]".to_string(),
        ]);
    }

    ffmpeg_command.extend(vec![
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "192k".to_string(),
        "-y".to_string(),
        output_video_path.clone(),
    ]);

    info!("Starting the video processing...");
    run_ffmpeg_command(&ffmpeg_command, duration, *advanced_log)?;

    info!("All done! Your video has been processed successfully.");

    Ok(())
}
