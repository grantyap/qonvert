//! This module contains helper functions for getting file paths.

use clean_path::Clean;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct InvalidDirectoryError {
    pub path: PathBuf,
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
pub struct MultipleInputDirectoriesError {}

impl std::error::Error for MultipleInputDirectoriesError {}

impl Display for MultipleInputDirectoriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Either a single directory or multiple files can be used as input"
        )
    }
}

pub async fn get_output_file_paths<T, U>(
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

    let output_file_paths = input_file_paths
        .iter()
        .map(|path| {
            output_directory
                .as_ref()
                .join(path.as_ref().with_extension(output_file_type))
                .clean()
        })
        .collect();

    Ok(output_file_paths)
}

pub async fn get_input_file_paths<T>(
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
