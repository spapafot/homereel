//! The ffmpeg sidecar: frames for AI titles, and thumbnails for videos Drive has none for. Input
//! is the local media proxy URL, so the OAuth token never appears in ffmpeg's arguments.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use futures_util::future::join_all;
use tokio::process::Command;

use crate::recorded_date;

/// Sampled at 10%, 30%, 50%, 70% and 90% of the video.
pub const FRAME_FRACTIONS: [f64; 5] = [0.1, 0.3, 0.5, 0.7, 0.9];
/// Small frames keep Gemini's image tokens low.
pub const TITLE_FRAME_WIDTH: u32 = 512;
/// Long edge of previews, sharp enough for the full-height Shorts player on a HiDPI screen.
const PREVIEW_LONG_EDGE: u32 = 1280;
/// JPEG quality on ffmpeg's 2–31 scale, lower is better. Previews are shown large; frames for
/// Gemini and the photo cropper are not.
const PREVIEW_QUALITY: u32 = 2;
const FRAME_QUALITY: u32 = 5;
/// Maps HDR (HLG or PQ, as iPhones record) to SDR, which otherwise comes out grey and flat.
const TONEMAP_FILTER: &str = "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,\
tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p";
/// How far before the end a frame is taken from a video that played to the end.
const END_MARGIN_MS: u64 = 500;
/// Seeking over HTTP can be slow for files with their index at the end; give each frame time.
const FRAME_TIMEOUT: Duration = Duration::from_secs(45);
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// ffmpeg's network read timeout, in microseconds.
const READ_TIMEOUT_US: &str = "30000000";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The ffmpeg sidecar: next to the app executable once bundled, or the downloaded binary in
/// `src-tauri/binaries` when running from source.
pub fn locate_ffmpeg() -> Option<PathBuf> {
    let file_name = format!("ffmpeg{}", std::env::consts::EXE_SUFFIX);
    let bundled = std::env::current_exe().ok()?.parent()?.join(&file_name);
    if bundled.exists() {
        return Some(bundled);
    }
    #[cfg(debug_assertions)]
    {
        let sidecar = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(format!(
                "ffmpeg-{}{}",
                env!("HOMEREEL_TARGET_TRIPLE"),
                std::env::consts::EXE_SUFFIX
            ));
        if sidecar.exists() {
            return Some(sidecar);
        }
    }
    None
}

fn ffmpeg_command(ffmpeg: &Path) -> Command {
    let mut command = Command::new(ffmpeg);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// Extracts one JPEG frame at `at_seconds`, `width` pixels wide.
pub async fn extract_frame(
    ffmpeg: &Path,
    input: &str,
    at_seconds: f64,
    width: u32,
) -> Result<Vec<u8>, String> {
    let filter = format!("scale={width}:-2");
    run_frame_command(ffmpeg, input, at_seconds, &filter, FRAME_QUALITY).await
}

/// Extracts one high-quality JPEG frame for a video preview, fitted into
/// [`PREVIEW_LONG_EDGE`] without upscaling and tone-mapped when the video is HDR.
pub async fn extract_preview(
    ffmpeg: &Path,
    input: &str,
    at_seconds: f64,
    hdr: bool,
) -> Result<Vec<u8>, String> {
    run_frame_command(
        ffmpeg,
        input,
        at_seconds,
        &preview_filter(hdr),
        PREVIEW_QUALITY,
    )
    .await
}

fn preview_filter(hdr: bool) -> String {
    // ffmpeg rotates phone videos before filtering, so `iw` and `ih` are already upright.
    let scale = format!(
        "scale='min(iw,{PREVIEW_LONG_EDGE})':'min(ih,{PREVIEW_LONG_EDGE})'\
         :force_original_aspect_ratio=decrease:force_divisible_by=2"
    );
    if hdr {
        format!("{TONEMAP_FILTER},{scale}")
    } else {
        scale
    }
}

/// With `-ss` before `-i`, ffmpeg seeks with HTTP range requests instead of downloading the
/// whole file.
async fn run_frame_command(
    ffmpeg: &Path,
    input: &str,
    at_seconds: f64,
    filter: &str,
    quality: u32,
) -> Result<Vec<u8>, String> {
    let mut command = ffmpeg_command(ffmpeg);
    command.args(["-nostdin", "-hide_banner", "-loglevel", "error"]);
    if input.starts_with("http") {
        command.args(["-rw_timeout", READ_TIMEOUT_US]);
    }
    command
        .args([
            "-ss",
            &format!("{at_seconds:.3}"),
            "-i",
            input,
            "-frames:v",
            "1",
        ])
        .args(["-vf", filter, "-q:v", &quality.to_string()])
        .args(["-f", "image2pipe", "-c:v", "mjpeg", "pipe:1"]);

    let output = tokio::time::timeout(FRAME_TIMEOUT, command.output())
        .await
        .map_err(|_| "ffmpeg timed out".to_owned())?
        .map_err(|error| format!("ffmpeg could not start: {error}"))?;
    if output.status.success() && output.stdout.starts_with(&[0xFF, 0xD8]) {
        Ok(output.stdout)
    } else {
        Err("ffmpeg could not decode a frame".to_owned())
    }
}

/// What ffmpeg's input summary says about a file.
#[derive(Debug, Default, PartialEq)]
pub struct MediaInfo {
    pub duration_seconds: Option<f64>,
    /// The recording date in the file's metadata, `YYYY-MM-DD`; see [`recorded_date`].
    pub recorded_date: Option<String>,
    /// HLG or PQ video, which needs tone mapping to look right as a JPEG.
    pub is_hdr: bool,
    /// Picture size as the video plays, after any rotation.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Reads ffmpeg's input summary. Empty when the file can't be read.
pub async fn probe(ffmpeg: &Path, input: &str) -> MediaInfo {
    let mut command = ffmpeg_command(ffmpeg);
    command.args(["-nostdin", "-hide_banner", "-i", input]);
    // ffmpeg exits with an error because no output is given; the summary is still on stderr.
    match tokio::time::timeout(PROBE_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let size = parse_picture_size(&stderr);
            MediaInfo {
                duration_seconds: parse_duration(&stderr),
                recorded_date: recorded_date::from_ffmpeg_metadata(&stderr),
                is_hdr: is_hdr_video(&stderr),
                width: size.map(|(width, _)| width),
                height: size.map(|(_, height)| height),
            }
        }
        _ => MediaInfo::default(),
    }
}

/// Reads the duration from ffmpeg's input summary, for files Drive has no duration for.
pub async fn probe_duration(ffmpeg: &Path, input: &str) -> Option<f64> {
    probe(ffmpeg, input).await.duration_seconds
}

/// Parses `Duration: 00:03:00.03` from ffmpeg's stderr.
pub fn parse_duration(stderr: &str) -> Option<f64> {
    let value = stderr.split("Duration: ").nth(1)?.split(',').next()?.trim();
    let mut parts = value.split(':');
    let hours: f64 = parts.next()?.parse().ok()?;
    let minutes: f64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Reads the first video stream's size from ffmpeg's summary, such as `1920x1080`. Phones store
/// portrait video as landscape marked as rotated, so a quarter turn swaps the two.
pub fn parse_picture_size(stderr: &str) -> Option<(u32, u32)> {
    let line = stderr.lines().find(|line| line.contains("Video:"))?;
    let (width, height) = line.split(',').find_map(|part| {
        let (width, height) = part.split_whitespace().next()?.split_once('x')?;
        // Codec tags such as `0x31637661` have the same shape.
        if width.starts_with('0') {
            return None;
        }
        Some((width.parse::<u32>().ok()?, height.parse::<u32>().ok()?))
    })?;
    let quarter_turn =
        rotation_degrees(stderr).is_some_and(|degrees| (degrees.abs().round() as i64) % 180 == 90);
    Some(if quarter_turn {
        (height, width)
    } else {
        (width, height)
    })
}

/// `Display Matrix: rotation of -90.00 degrees`, or `rotate : 90` from older ffmpeg builds.
fn rotation_degrees(stderr: &str) -> Option<f64> {
    let lower = stderr.to_ascii_lowercase();
    if let Some(rest) = lower.split("rotation of ").nth(1) {
        return rest.split_whitespace().next()?.parse().ok();
    }
    lower.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim() == "rotate" {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

/// Whether a video stream in ffmpeg's summary uses an HDR transfer: `arib-std-b67` (HLG, what
/// iPhones record) or `smpte2084` (PQ).
pub fn is_hdr_video(stderr: &str) -> bool {
    stderr
        .lines()
        .filter(|line| line.contains("Video:"))
        .any(|line| line.contains("arib-std-b67") || line.contains("smpte2084"))
}

/// Where to take a frame for `position_ms`. ffmpeg finds no frame when asked for the very end,
/// which is where a finished video stops.
pub fn capture_position(position_ms: u64, duration_ms: Option<u64>) -> u64 {
    match duration_ms {
        Some(duration_ms) => position_ms.min(duration_ms.saturating_sub(END_MARGIN_MS)),
        None => position_ms,
    }
}

/// Extracts up to five frames spread over the video, skipping any that fail.
pub async fn extract_frames(ffmpeg: &Path, input: &str, duration_ms: Option<u64>) -> Vec<Vec<u8>> {
    let duration_seconds = match duration_ms {
        Some(ms) if ms > 0 => Some(ms as f64 / 1000.0),
        _ => probe_duration(ffmpeg, input).await,
    };
    let timestamps: Vec<f64> = match duration_seconds {
        Some(duration) => FRAME_FRACTIONS
            .iter()
            .map(|fraction| duration * fraction)
            .collect(),
        None => vec![1.0],
    };
    // Each seek costs a few Drive round trips, so the frames are fetched in parallel.
    join_all(
        timestamps
            .into_iter()
            .map(|at_seconds| extract_frame(ffmpeg, input, at_seconds, TITLE_FRAME_WIDTH)),
    )
    .await
    .into_iter()
    .filter_map(Result::ok)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_durations() {
        let stderr = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'x':\n  Duration: 00:03:00.03, start: 0.000000, bitrate: 2711 kb/s";

        assert_eq!(parse_duration(stderr), Some(180.03));
        assert_eq!(
            parse_duration("  Duration: 01:02:03.50, start"),
            Some(3723.5)
        );
        assert_eq!(parse_duration("  Duration: N/A, bitrate: N/A"), None);
        assert_eq!(parse_duration("no summary"), None);
    }

    #[test]
    fn detects_hlg_and_pq_videos() {
        let hlg = "  Stream #0:0[0x1](und): Video: hevc (Main 10) (hvc1 / 0x31637668), \
                   yuv420p10le(tv, bt2020nc/bt2020/arib-std-b67), 1080x1920";
        let pq = "  Stream #0:0: Video: hevc (Main 10), yuv420p10le(tv, bt2020nc/bt2020/smpte2084)";
        let sdr = "  Stream #0:0: Video: h264 (High) (avc1), yuv420p(tv, bt709), 1920x1080\n  \
                   Stream #0:1: Data: none (arib-std-b67)";

        assert!(is_hdr_video(hlg));
        assert!(is_hdr_video(pq));
        assert!(!is_hdr_video(sdr));
        assert!(!is_hdr_video(""));
    }

    #[test]
    fn reads_the_upright_picture_size() {
        let landscape = "  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), \
                         yuv420p(progressive), 320x240 [SAR 1:1 DAR 4:3], 182 kb/s, 10 fps";
        let iphone = "  Stream #0:0[0x1](und): Video: hevc (Main 10) (hvc1 / 0x31637668), \
                      yuv420p10le(tv, bt2020nc/bt2020/arib-std-b67), 1920x1080, 9000 kb/s\n    \
                      Side data:\n      Display Matrix: rotation of -90.00 degrees";
        let old_build = "  Stream #0:0: Video: h264, yuv420p, 1280x720\n    Metadata:\n      \
                         rotate          : 270";
        let upside_down = "  Stream #0:0: Video: h264, yuv420p, 1280x720\n      \
                           displaymatrix: rotation of 180.00 degrees";

        assert_eq!(parse_picture_size(landscape), Some((320, 240)));
        assert_eq!(parse_picture_size(iphone), Some((1080, 1920)));
        assert_eq!(parse_picture_size(old_build), Some((720, 1280)));
        assert_eq!(parse_picture_size(upside_down), Some((1280, 720)));
        assert_eq!(
            parse_picture_size("  Stream #0:1: Audio: aac, 44100 Hz"),
            None
        );
    }

    #[test]
    fn captures_a_little_before_the_end_of_a_finished_video() {
        assert_eq!(capture_position(14_215, Some(14_233)), 13_733);
        assert_eq!(capture_position(7_000, Some(14_233)), 7_000);
        assert_eq!(capture_position(300, Some(400)), 0);
        assert_eq!(capture_position(14_215, None), 14_215);
    }

    /// Width and height from a baseline JPEG's SOF0 segment.
    fn jpeg_size(bytes: &[u8]) -> (u32, u32) {
        let mut index = 2;
        while index + 9 < bytes.len() {
            let marker = bytes[index + 1];
            let length = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]]) as usize;
            if marker == 0xC0 {
                let height = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]);
                let width = u16::from_be_bytes([bytes[index + 7], bytes[index + 8]]);
                return (width.into(), height.into());
            }
            index += 2 + length;
        }
        panic!("no SOF0 segment");
    }

    fn generate_clip(ffmpeg: &Path, path: &Path, size: &str) {
        let status = std::process::Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi"])
            .args(["-i", &format!("testsrc2=size={size}:rate=10")])
            .args(["-t", "2", "-pix_fmt", "yuv420p"])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn fits_previews_into_the_long_edge_without_upscaling() {
        let ffmpeg = locate_ffmpeg()
            .expect("run scripts/fetch-ffmpeg.ps1 or .sh to install the ffmpeg sidecar");
        let dir = tempfile::tempdir().unwrap();
        let large = dir.path().join("large.mp4");
        let small = dir.path().join("small.mp4");
        generate_clip(&ffmpeg, &large, "1080x1920");
        generate_clip(&ffmpeg, &small, "320x240");

        let large_preview = extract_preview(&ffmpeg, &large.to_string_lossy(), 1.0, false)
            .await
            .unwrap();
        let small_preview = extract_preview(&ffmpeg, &small.to_string_lossy(), 1.0, false)
            .await
            .unwrap();

        assert_eq!(jpeg_size(&large_preview), (720, 1280));
        assert_eq!(jpeg_size(&small_preview), (320, 240));
    }

    /// iPhones record 10-bit HEVC in HLG; the probe must spot it and the tone mapper must run.
    #[tokio::test]
    async fn tone_maps_hlg_previews() {
        let ffmpeg = locate_ffmpeg()
            .expect("run scripts/fetch-ffmpeg.ps1 or .sh to install the ffmpeg sidecar");
        let dir = tempfile::tempdir().unwrap();
        let clip = dir.path().join("hlg.mp4");
        let status = std::process::Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi"])
            .args(["-i", "testsrc2=size=360x640:rate=10", "-t", "2"])
            .args(["-pix_fmt", "yuv420p10le", "-c:v", "libx265", "-x265-params"])
            .arg("log-level=error:colorprim=bt2020:transfer=arib-std-b67:colormatrix=bt2020nc")
            .arg(&clip)
            .status()
            .unwrap();
        assert!(status.success());
        let input = clip.to_string_lossy();

        let info = probe(&ffmpeg, &input).await;
        let preview = extract_preview(&ffmpeg, &input, 1.0, info.is_hdr)
            .await
            .unwrap();

        assert!(info.is_hdr);
        assert_eq!(jpeg_size(&preview), (360, 640));
    }

    #[tokio::test]
    async fn probes_the_size_of_a_rotated_clip() {
        let ffmpeg = locate_ffmpeg()
            .expect("run scripts/fetch-ffmpeg.ps1 or .sh to install the ffmpeg sidecar");
        let dir = tempfile::tempdir().unwrap();
        let clip = dir.path().join("landscape.mp4");
        let rotated = dir.path().join("portrait.mp4");
        generate_clip(&ffmpeg, &clip, "320x240");
        let status = std::process::Command::new(&ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-display_rotation",
                "90",
                "-i",
            ])
            .arg(&clip)
            .args(["-c", "copy"])
            .arg(&rotated)
            .status()
            .unwrap();
        assert!(status.success());

        let info = probe(&ffmpeg, &rotated.to_string_lossy()).await;

        assert_eq!((info.width, info.height), (Some(240), Some(320)));
        assert!(info.duration_seconds.is_some_and(|seconds| seconds > 1.5));
    }

    /// Exercises the real sidecar on a generated clip, including a file whose index is at the end.
    #[tokio::test]
    async fn extracts_frames_from_a_generated_clip() {
        let ffmpeg = locate_ffmpeg()
            .expect("run scripts/fetch-ffmpeg.ps1 or .sh to install the ffmpeg sidecar");
        let dir = tempfile::tempdir().unwrap();
        let clip = dir.path().join("clip.mp4");
        let status = std::process::Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi"])
            .args([
                "-i",
                "testsrc2=size=320x240:rate=10",
                "-t",
                "6",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(status.success());
        let input = clip.to_string_lossy();

        let with_duration = extract_frames(&ffmpeg, &input, Some(6_000)).await;
        let probed = extract_frames(&ffmpeg, &input, None).await;

        assert_eq!(with_duration.len(), 5);
        assert_eq!(probed.len(), 5);
        assert!(
            with_duration
                .iter()
                .all(|frame| frame.starts_with(&[0xFF, 0xD8]))
        );
    }
}
