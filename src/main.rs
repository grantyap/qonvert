use argparse::{ArgumentParser, Store, StoreTrue};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use qonvert::execute_ffmpeg_encoding;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct Args {
    input_directory: String,
    input_file_type: String,
    output_directory: String,
    output_file_type: String,
    codec: String,
    verbose: bool,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            input_directory: ".".to_string(),
            input_file_type: "webm".to_string(),
            output_directory: ".".to_string(),
            output_file_type: "mp4".to_string(),
            codec: String::default(),
            verbose: false,
        }
    }
}

fn parse_args() -> Args {
    let mut args = Args::default();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("A tool to easily batch convert files using FFmpeg");
        ap.refer(&mut args.input_file_type).add_argument(
            "input file type",
            Store,
            "The file type of the files in the input directory to convert. Default: webm",
        );
        ap.refer(&mut args.output_file_type).add_argument(
            "output file type",
            Store,
            "The output file type of the converted files. Default: mp4",
        );
        ap.refer(&mut args.codec).add_option(
            &["-c"],
            Store,
            "The FFmpeg video codec to use. Default: libx265",
        );
        ap.refer(&mut args.input_directory).add_option(
            &["-i", "--input-dir"],
            Store,
            "The directory containing the files to convert",
        );
        ap.refer(&mut args.output_directory).add_option(
            &["-o", "--output-dir"],
            Store,
            "The directory to place the converted files in",
        );
        ap.refer(&mut args.verbose).add_option(
            &["-v", "--v"],
            StoreTrue,
            "Display extra information",
        );
        ap.parse_args_or_exit();
    }

    args.codec = match (
        args.input_file_type.as_str(),
        args.output_file_type.as_str(),
    ) {
        ("webm", "mp4") => "libx265".to_string(),
        _ => String::default(),
    };

    args
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

async fn get_file_paths_of_type_in_directory<T>(
    directory: T,
    file_type: &str,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>
where
    T: AsRef<Path>,
{
    if !directory.as_ref().is_dir() {
        return Err(Box::new(InvalidDirectoryError {
            path: directory.as_ref().to_path_buf(),
        }));
    }

    let mut results: Vec<PathBuf> = vec![];
    let mut directory = tokio::fs::read_dir(directory).await?;
    while let Some(path) = directory.next_entry().await? {
        let path = path.path();
        let path_file_type = path.extension();
        if let Some(path_file_type) = path_file_type {
            if path_file_type == file_type {
                results.push(path);
            }
        }
    }

    Ok(results)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args();

    let input_directory = Path::new(&args.input_directory);
    let output_directory = Path::new(&args.output_directory);

    let input_file_paths =
        get_file_paths_of_type_in_directory(input_directory, &args.input_file_type).await?;
    let output_file_paths =
        get_output_file_paths(output_directory, &input_file_paths, &args.output_file_type).await?;

    println!("Converting files with {}:", &args.codec);
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

            execute_ffmpeg_encoding(input_path, output_path, &args.codec)
                .await
                .unwrap_or_else(|_| {
                    // TODO: Write the error to stderr?
                    progress_bar.println(format!(
                        "\x1b[0;31m  {} failed\x1b[0m",
                        input_path.to_string_lossy()
                    ));
                });

            progress_bar.inc(1);
            progress_bar.println(format!(
                "\x1b[0;32m  {}\x1b[0m",
                output_path.to_string_lossy()
            ));
        })
        .await;

    progress_bar.println("");
    progress_bar.finish();

    Ok(())
}
