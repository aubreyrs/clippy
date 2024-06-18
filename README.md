# clippy
 🎥 FFMPEG Wrapper written in Rust for quickly applying edits onto clips!

example config

```
[settings]
input_video_path = "C:\\Users\\Aubrey\\Documents\\Editing\\silly.mp4"
output_video_path = "C:\\Users\\Aubrey\\Documents\\Editing\\owo.mp4"
ffmpeg_path = "C:\\Users\\Aubrey\\Documents\\Temp\\ffmpeg-7.0.1-essentials_build\\ffmpeg-7.0.1-essentials_build\\bin\\ffmpeg.exe"
use_gpu = true
video_bitrate = "25M"
upscale_resolution = "None"
background_audio_path = "C:\\Users\\Aubrey\\Documents\\Editing\\background.mp3"
audio_start_time = 12.0
replace_audio = false
original_audio_volume = 5.0
background_audio_volume = 0.3
clip_start_time = "None"
clip_end_time = "None"
video_speed = 1.0
advanced_log = false
fade_in_duration = 0.8
fade_out_duration = 0.8
```

todo
- add better logging
- refactor shit (stuff is messy)
- fix audio related stuff
