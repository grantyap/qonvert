use std::{fmt::Display, path::Path, process::ExitStatus, str};

use tokio::process::Command;

#[derive(Debug)]
struct FFmpegError {
    status: ExitStatus,
    stderr: String,
}

impl std::error::Error for FFmpegError {}

impl Display for FFmpegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FFmpeg execution failed: {}", self.status)
    }
}

pub async fn execute_ffmpeg_encoding(
    input: &Path,
    output: &Path,
    codec: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&input)
        .args(&["-c:v", codec])
        .args(&["-movflags", "+faststart"])
        // Ensure that the dimensions are divisible by 2.
        .args(&["-vf", "crop=trunc(iw/2)*2:trunc(ih/2)*2"])
        .arg(&output)
        .output()
        .await?;

    match result.status.success() {
        true => Ok(()),
        false => Err(Box::new(FFmpegError {
            status: result.status,
            stderr: str::from_utf8(&result.stderr)
                .map(|s| s.to_string())
                .unwrap_or_else(|e| format!("Could not parse stderr: {}", e)),
        })),
    }
}
