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
                "ffmpeg-x86_64-pc-windows-msvc{}",
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

/// Extracts one JPEG frame at `at_seconds`, `width` pixels wide. With `-ss` before `-i`, ffmpeg
/// seeks with HTTP range requests instead of downloading the whole file.
pub async fn extract_frame(
    ffmpeg: &Path,
    input: &str,
    at_seconds: f64,
    width: u32,
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
        .args(["-vf", &format!("scale={width}:-2"), "-q:v", "5"])
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
}

/// Reads ffmpeg's input summary. Empty when the file can't be read.
pub async fn probe(ffmpeg: &Path, input: &str) -> MediaInfo {
    let mut command = ffmpeg_command(ffmpeg);
    command.args(["-nostdin", "-hide_banner", "-i", input]);
    // ffmpeg exits with an error because no output is given; the summary is still on stderr.
    match tokio::time::timeout(PROBE_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            MediaInfo {
                duration_seconds: parse_duration(&stderr),
                recorded_date: recorded_date::from_ffmpeg_metadata(&stderr),
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

    /// Exercises the real sidecar on a generated clip, including a file whose index is at the end.
    #[tokio::test]
    async fn extracts_frames_from_a_generated_clip() {
        let ffmpeg =
            locate_ffmpeg().expect("run scripts/fetch-ffmpeg.ps1 to install the ffmpeg sidecar");
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
