use clap::{arg, command, Parser};
use pii_masker::masker::PIIMaskable;
use pii_masker::rust::Rust;
use std::fs;

#[derive(Parser, Debug)]
#[command(name = "pii-masker", about = "Masks PII within source files")]
struct PIIMaskerArgs {
    #[arg(long)]
    source_path: std::path::PathBuf,

    #[arg(long)]
    fixture_path: std::path::PathBuf,

    #[arg(long)]
    word_pool_size: Option<usize>,
}

fn main() {
    let args = PIIMaskerArgs::parse();

    let source_content = match fs::read_to_string(&args.source_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading source file: {}", err);
            std::process::exit(1);
        }
    };
    let fixture_content = match fs::read_to_string(&args.fixture_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading fixture file: {}", err);
            std::process::exit(1);
        }
    };

    let masked = Rust::mask_tests(
        Rust::Rs,
        &source_content,
        &fixture_content,
        args.word_pool_size.unwrap_or(10000),
    );

    // write the masked content to a file
    match masked {
        Ok((masked_content, masked_fixture)) => {
            let source_file_ext: String = args
                .source_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_string())
                .unwrap();
            let masked_source_file = args
                .source_path
                .with_extension(format!("{}.masked", source_file_ext));
            let fixture_file_ext: String = args
                .fixture_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_string())
                .unwrap();
            let masked_fixture_file = args
                .fixture_path
                .with_extension(format!("{}.masked", fixture_file_ext));
            fs::write(masked_source_file, masked_content)
                .expect("Unable to write masked content to file");
            fs::write(masked_fixture_file, masked_fixture)
                .expect("Unable to write masked fixture content to file");
        }
        Err(err) => {
            eprintln!("Error masking PII: {:?}", err);
            std::process::exit(1);
        }
    }
}
