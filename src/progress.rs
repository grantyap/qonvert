use std::collections::HashMap;

use tokio::io::{AsyncBufRead, Lines};

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
    fn from_parsed_progress(parsed_progress: &HashMap<String, String>) -> Self {
        Self {
            frame: parsed_progress
                .get("frame")
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            progress: FFmpegProgressState::from_str(parsed_progress.get("progress").unwrap()),
        }
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
            let progress = FFmpegProgress::from_parsed_progress(&parsed_progress);
            on_progress_update(progress);

            parsed_progress.clear();
        }
    }

    Ok(())
}
