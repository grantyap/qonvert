use core::fmt;
use std::collections::HashMap;

use tokio::io::{AsyncBufRead, Lines};

#[derive(Debug, Clone)]
pub enum FFmpegProgressError<'a> {
    Parse(&'a str),
}

impl fmt::Display for FFmpegProgressError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(key) => write!(f, "Could not parse key '{}' from FFprobe progress logs", key),
        }
    }
}

impl std::error::Error for FFmpegProgressError<'_> {}

pub trait FFmpegProgressUpdate: FnOnce(FFmpegProgress) + Copy {}

impl<T> FFmpegProgressUpdate for T where T: FnOnce(FFmpegProgress) + Copy {}

#[derive(Debug)]
pub enum FFmpegProgressState {
    Continue,
    End,
}

impl FFmpegProgressState {
    fn from_str(s: &str) -> Self {
        match s {
            "continue" => Self::Continue,
            "end" => Self::End,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub struct FFmpegProgress {
    pub frame: u64,
    pub progress: FFmpegProgressState,
}

impl FFmpegProgress {
    fn try_from_parsed_progress(
        parsed_progress: &HashMap<String, String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            frame: parsed_progress
                .get("frame")
                .ok_or(FFmpegProgressError::Parse("frame"))?
                .parse::<u64>()?,
            progress: FFmpegProgressState::from_str(
                parsed_progress
                    .get("progress")
                    .ok_or(FFmpegProgressError::Parse("progress"))?,
            ),
        })
    }
}

fn parse_key_value_pair(input: &str) -> Option<(&str, &str)> {
    input
        .split_once('=')
        .map(|(key, value)| (key.trim(), value.trim()))
}

pub async fn parse<R, F>(
    reader: &mut Lines<R>,
    on_progress_update: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    R: AsyncBufRead + Unpin,
    F: FFmpegProgressUpdate,
{
    let mut parsed_progress = HashMap::<String, String>::new();
    while let Some(line) = reader.next_line().await? {
        let Some((key, value)) = parse_key_value_pair(&line) else {
            continue;
        };

        parsed_progress.insert(key.to_string(), value.to_string());

        if key == "progress" {
            let progress = FFmpegProgress::try_from_parsed_progress(&parsed_progress)?;
            on_progress_update(progress);

            parsed_progress.clear();
        }
    }

    Ok(())
}
