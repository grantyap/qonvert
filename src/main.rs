use clap::Parser;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use qonvert::execute_ffmpeg_encoding;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

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

#[derive(Debug)]
struct InvalidDirectoryError {
    path: PathBuf,
}

impl std::error::Error for InvalidDirectoryError {}

impl Display for InvalidDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{}' is not a valid directory",
            self.path.to_string_lossy()
        )
    }
}

#[derive(Debug)]
struct MultipleInputDirectoriesError {}

impl std::error::Error for MultipleInputDirectoriesError {}

impl Display for MultipleInputDirectoriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Either a single directory or multiple files can be used as input"
        )
    }
}

async fn get_output_file_paths<T, U>(
    output_directory: T,
    input_file_paths: &[U],
    output_file_type: &str,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>
where
    T: AsRef<Path>,
    U: AsRef<Path>,
{
    if !output_directory.as_ref().is_dir() {
        return Err(Box::new(InvalidDirectoryError {
            path: output_directory.as_ref().to_path_buf(),
        }));
    }

    let mut output_file_paths: Vec<PathBuf> = vec![];
    for path in input_file_paths {
        let output_file_path = {
            let path_with_extension = path.as_ref().with_extension(output_file_type);
            output_directory
                .as_ref()
                .strip_prefix(".")?
                .join(path_with_extension)
        };
        output_file_paths.push(output_file_path);
    }

    Ok(output_file_paths)
}

async fn get_input_file_paths<T>(
    input_file_paths: &[T],
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>
where
    T: AsRef<Path>,
{
    // If there's only one path and it is a directory,
    // return all the file paths inside it.
    if input_file_paths.len() == 1 {
        let input_file_path = input_file_paths[0].as_ref();
        if input_file_path.is_dir() {
            let mut results: Vec<PathBuf> = vec![];

            let mut directory = tokio::fs::read_dir(input_file_path).await?;
            while let Some(path) = directory.next_entry().await? {
                let path = path.path();
                if path.is_dir() {
                    continue;
                }
                results.push(path);
            }

            return Ok(results);
        }
    }

    let mut results: Vec<PathBuf> = vec![];
    for input_file_path in input_file_paths {
        // If there are multiple input paths, none of the paths can be directories.
        if input_file_path.as_ref().is_dir() {
            return Err(Box::new(MultipleInputDirectoriesError {}));
        }

        results.push(input_file_path.as_ref().to_path_buf());
    }

    Ok(results)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input_file_paths = get_input_file_paths(&args.input_paths).await?;
    let output_directory = args
        .output_directory
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    let output_file_paths =
        get_output_file_paths(&output_directory, &input_file_paths, &args.output_file_type).await?;

    // Define default codecs for certain file types.
    let codec = match args.codec {
        None => match args.output_file_type.as_ref() {
            "mp4" => Some("libx265".to_string()),
            _ => None,
        },
        Some(c) => Some(c),
    };

    if let Some(codec) = &codec {
        println!("Converting files with {}:", codec);
    } else {
        println!("Converting:");
    }

    for input_file_path in &input_file_paths {
        println!("  {}", input_file_path.to_string_lossy());
    }

    println!("\nDone:");

    let progress_bar = ProgressBar::new(input_file_paths.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {wide_msg}"),
    );
    progress_bar.enable_steady_tick(1_000);

    futures::stream::iter(input_file_paths.iter().zip(output_file_paths.iter()))
        .for_each_concurrent(None, |(input_path, output_path)| async {
            progress_bar.set_message(format!(
                "{}",
                input_path.file_name().unwrap().to_string_lossy()
            ));

            let encoding_result =
                execute_ffmpeg_encoding(input_path, output_path, codec.as_deref()).await;

            progress_bar.inc(1);
            match encoding_result {
                Ok(_) => {
                    progress_bar.println(format!(
                        "\x1b[0;32m  {}\x1b[0m",
                        output_path.to_string_lossy()
                    ));
                }
                Err(e) => match &args.verbose {
                    true => {
                        progress_bar.println(format!(
                            "\x1b[0;31m  {} failed:\n{}\x1b[0m",
                            input_path.to_string_lossy(),
                            e
                        ));
                    }
                    false => {
                        progress_bar.println(format!(
                            "\x1b[0;31m  {} failed\x1b[0m",
                            input_path.to_string_lossy()
                        ));
                    }
                },
            }
        })
        .await;

    progress_bar.finish();

    Ok(())
}
