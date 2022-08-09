use std::{fmt::Display, path::Path, process::ExitStatus, str};

use tokio::process::Command;

#[derive(Debug)]
pub struct FFmpegError {
    pub status: ExitStatus,
    pub stderr: String,
}

impl std::error::Error for FFmpegError {}

impl Display for FFmpegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FFmpeg execution failed with status {}:\n{}",
            self.status, self.stderr
        )
    }
}

pub async fn execute_ffmpeg_encoding(
    input: &Path,
    output: &Path,
    codec: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("ffmpeg");
    command.arg("-y").args(&["-i", &input.to_string_lossy()]);
    if let Some(codec) = codec {
        command.args(&["-c:v", codec]);
    }
    command
        .args(&["-movflags", "+faststart"])
        // Ensure that the dimensions are divisible by 2.
        .args(&["-vf", "crop=trunc(iw/2)*2:trunc(ih/2)*2"])
        .arg(&output);

    let result = command.output().await?;
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
