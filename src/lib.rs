pub mod progress;

use std::{
    fmt::Display,
    path::Path,
    process::{ExitStatus, Stdio},
    str,
};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use progress::FFmpegProgressUpdate;

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

fn ffmpeg_command(input: &Path, output: &Path, codec: Option<&str>) -> Command {
    let mut command = Command::new("ffmpeg");

    // Emit progress to `stdout`.
    command.args(["-progress", "pipe:1"]);

    // Overwrite the output file.
    // TODO: Maybe provide an option for overriding?
    command.arg("-y");

    command.args(["-i", &input.to_string_lossy()]);

    // Use the given codec, else use FFmpeg's default codec for the output extension.
    if let Some(codec) = codec {
        command.args(["-c:v", codec]);

        match codec {
            // Support h.265 thumbnail previews on Apple devices.
            "libx265" | "hevc_videotoolbox" => {
                command.args(["-tag:v", "hvc1"]);
            }
            _ => (),
        };
    }

    command
        .args(["-movflags", "faststart"])
        // Ensure that `.gif` colors are correctly converted.
        .args(["-pix_fmt", "yuv420p"])
        // Ensure that the dimensions are divisible by 2.
        .args(["-vf", "crop=trunc(iw/2)*2:trunc(ih/2)*2"])
        .arg(output);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

pub async fn execute_ffmpeg_encoding<F>(
    input: &Path,
    output: &Path,
    codec: Option<&str>,
    on_progress_update: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FFmpegProgressUpdate,
{
    let mut command = ffmpeg_command(input, output, codec);
    let mut process = command.spawn()?;

    // Set up reading from `stdout` and `stderr`.

    let stdout = process.stdout.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout).lines();

    let mut error_string = String::new();
    let stderr = process.stderr.take().unwrap();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Execute the command.
    let result_handle = tokio::spawn(async move { process.wait().await });

    // Read from `stdout` and `stderr`.

    progress::parse(&mut stdout_reader, on_progress_update).await?;

    while let Some(line) = stderr_reader.next_line().await? {
        error_string = format!("{}\n{}", error_string, line).trim().to_string();
    }

    // Wait for the command to finish.
    let status = result_handle.await??;

    if !status.success() {
        return Err(Box::new(FFmpegError {
            status,
            stderr: error_string,
        }));
    }

    Ok(())
}

pub async fn get_frame_count_from_file_path(
    input: &Path,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut command = Command::new("ffprobe");
    command.args(["-v", "error"]);

    // TODO: Handle audio streams as well.
    command.args(["-select_streams", "v:0"]);

    command
        .arg("-count_packets")
        .args(["-show_entries", "stream=nb_read_packets"])
        .args(["-of", "csv=p=0"])
        .arg(input);

    // Parse from `stdout` into a `u64`.
    let stdout = command.output().await?.stdout;
    let output = str::from_utf8(&stdout)?.trim();
    output.parse::<u64>().map_err(|err| {
        let dyn_err: Box<dyn std::error::Error> = Box::new(err);
        dyn_err
    })
}
