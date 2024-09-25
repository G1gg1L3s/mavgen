use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

/// Generate Rust code from XML MAVLink definitions.
#[derive(Parser, Debug)]
struct Args {
    /// Path to definition files
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output file or directory
    #[arg(short, long)]
    output: PathBuf,
}

fn resolve_input(paths: Vec<PathBuf>) -> anyhow::Result<Vec<PathBuf>> {
    let mut result = Vec::with_capacity(paths.len());
    for path in paths {
        let meta = path
            .metadata()
            .with_context(|| format!("accessing {}", path.display()))?;

        if meta.is_dir() {
            let ls = std::fs::read_dir(&path)
                .with_context(|| format!("reading directory {}", path.display()))?;
            for entry in ls {
                let path = entry?.path();
                if path.is_file() {
                    result.push(path);
                }
            }
        } else {
            result.push(path);
        }
    }

    Ok(result)
}

fn print_and_format_mavgen_error(error: mavgen::Error) -> anyhow::Error {
    match error {
        mavgen::Error::CreateDir(error, path_buf) => anyhow::anyhow!(
            "failed to create directory {}: {}",
            path_buf.display(),
            error
        ),
        mavgen::Error::ParseXml(errors) => {
            eprintln!("Errors occured during xml parsing:");
            for error in errors {
                eprintln!("- {error:#}");
            }
            anyhow::anyhow!("failed to parse XML")
        }
        mavgen::Error::NormalisePath(error, path_buf) => {
            anyhow::anyhow!("failed to normalise path {}: {}", path_buf.display(), error)
        }
        mavgen::Error::Flattening(error) => anyhow::anyhow!("failed to flatten a module: {error}"),
        mavgen::Error::Normalisation(errors, path_buf) => {
            eprintln!(
                "Errors occured during model normalisation in {}",
                path_buf.display()
            );

            for error in errors {
                eprintln!("- {error:#}");
            }
            anyhow::anyhow!("failed to normalise mavlink model")
        }
        mavgen::Error::InvalidFilename(os_string) => {
            anyhow::anyhow!("unsupported filename: {:?}", os_string)
        }
        mavgen::Error::WritingToFile(error, path_buf) => anyhow::anyhow!(
            "failed to write to a file {}: {}",
            path_buf.display(),
            error
        ),
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let input_is_one_file = args.input.len() == 1 && args.input[0].is_file();
    let input = resolve_input(args.input)?;

    let result = if input_is_one_file {
        mavgen::generate_one(&input[0], &args.output)
    } else if args.output.is_file() {
        anyhow::bail!("for multiple input definitions the output should point to a directory to generate a tree of modules");
    } else {
        mavgen::generate_dir(&input, &args.output)
    };

    if let Err(err) = result {
        let err = print_and_format_mavgen_error(err);
        Err(err)
    } else {
        Ok(())
    }
}
