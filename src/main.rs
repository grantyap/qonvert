mod path;

use std::path::{Path, PathBuf};

use clap::Parser;
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use path::{get_input_file_paths, get_output_file_paths};
use qonvert::progress::FFmpegProgress;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The files to be converted
    #[clap(value_parser)]
    input_paths: Vec<PathBuf>,

    /// The directory for the converted files
    #[clap(short, long, value_parser)]
    output_directory: Option<PathBuf>,

    /// The file extension of the converted files
    #[clap(short = 't', long, value_parser)]
    output_file_type: String,

    /// The FFmpeg codec to use for conversion
    #[clap(short, long, value_parser)]
    codec: Option<String>,

    /// Enable debugging information
    #[clap(short, long, value_parser, default_value_t = false)]
    verbose: bool,
}

struct EncodingProcess {
    input_file_path: PathBuf,
    output_file_path: PathBuf,
    frame_count: u64,
    progress_bar: ProgressBar,
    codec: Option<String>,
}

impl EncodingProcess {
    fn new<T, U>(
        input_file_path: T,
        output_file_path: U,
        frame_count: u64,
        progress_bar: ProgressBar,
        codec: Option<String>,
    ) -> Self
    where
        T: AsRef<Path>,
        U: AsRef<Path>,
    {
        return Self {
            input_file_path: input_file_path.as_ref().to_owned(),
            output_file_path: output_file_path.as_ref().to_owned(),
            frame_count,
            progress_bar,
            codec,
        };
    }
}

impl std::fmt::Debug for EncodingProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncodingProcess")
            .field("input_file_path", &self.input_file_path)
            .field("output_file_path", &self.output_file_path)
            .field("frame_count", &self.frame_count)
            .field("codec", &self.codec)
            .finish()
    }
}

fn new_progress_bar<S>(message: S, frame_count: u64, progress_bars: &MultiProgress) -> ProgressBar
where
    S: Into<String>,
{
    let template_string = concat!(
        "  \x1b[1;36m{wide_msg}\x1b[0m\n",
        "    {percent:>3}% {bar:40.green/cyan} \x1b[2m{pos}/{len} ({eta} left)\x1b[22m"
    );

    let pb = ProgressBar::new(frame_count)
        .with_style(ProgressStyle::with_template(template_string).unwrap())
        .with_message(message.into());

    progress_bars.add(pb)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Define default codecs for certain file types.
    let codec = match args.codec {
        None => match args.output_file_type.as_ref() {
            "mp4" => Some("libx265".to_string()),
            _ => None,
        },
        Some(c) => Some(c),
    };

    if let Some(codec) = &codec {
        println!("Converting file(s) with \x1b[36m{}\x1b[0m:", codec);
    } else {
        println!("Converting:");
    }

    let input_file_paths = get_input_file_paths(&args.input_paths).await?;
    let file_count = input_file_paths.len();

    let output_directory = args
        .output_directory
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    let output_file_paths =
        get_output_file_paths(&output_directory, &input_file_paths, &args.output_file_type).await?;

    let input_and_output_file_paths = input_file_paths
        .into_iter()
        .zip(output_file_paths.into_iter());
    let progress_bars = MultiProgress::new();

    futures::stream::iter(input_and_output_file_paths)
        // Create an `EncodingProcess` for each input and output file pair.
        .then(|(input_file_path, output_file_path)| async {
            let frame_count = qonvert::get_frame_count_from_file_path(&input_file_path)
                .await
                .expect("Could not get frame count");

            let progress_bar = new_progress_bar(
                input_file_path.to_string_lossy(),
                frame_count,
                &progress_bars,
            );

            EncodingProcess::new(
                input_file_path,
                output_file_path,
                frame_count,
                progress_bar,
                codec.clone(),
            )
        })
        // Concurrently execute an FFmpeg encoding command for each encoding process.
        .for_each_concurrent(None, |encoding_process| async move {
            qonvert::execute_ffmpeg_encoding(
                &encoding_process.input_file_path,
                &encoding_process.output_file_path,
                encoding_process.codec.as_deref(),
                |progress: FFmpegProgress| {
                    encoding_process.progress_bar.set_position(progress.frame);
                },
            )
            .await
            .expect("Could not execute FFmpeg command");

            encoding_process.progress_bar.finish();
        })
        .await;

    println!(
        "Successfully converted \x1b[32m{}\x1b[0m file(s)!",
        file_count
    );

    Ok(())
}
