use clap::{App};
use dialoguer::Select;
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    // Initialize the CLI app
    let _matches = App::new("MP3 Player")
        .version("0.1.0")
        .author("J. Gavin Ray")
        .about("Plays an MP3 file")
        .get_matches();

    loop {
        // List all MP3 files in the current directory
        let mut mp3_files: Vec<_> = WalkDir::new(".")
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "mp3"))
            .map(|e| e.path().display().to_string())
            .collect();

        // Add an option to exit the program
        mp3_files.push("Exit".into());

        // Let the user select an MP3 file to play or exit
        let selection = Select::new()
            .with_prompt("Select an MP3 file to play or 'Exit' to quit")
            .default(0)
            .items(&mp3_files)
            .interact();

        match selection {
            Ok(index) if index < mp3_files.len() - 1 => {
                // Play the selected MP3 file
                let file_path = &mp3_files[index];
                play_mp3(file_path).expect("Error playing MP3 file");
            }
            _ => break, // Exit if 'Exit' is selected or on error
        }
    }
}

fn play_mp3(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create an output stream and stream handle
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let file = File::open(Path::new(file_path))?;
    let source = Decoder::new_mp3(BufReader::new(file))?;

    // Play the audio and block until it finishes
    let sink = rodio::Sink::try_new(&stream_handle)?;
    sink.append(source);
    println!("Playing: {}", file_path);

    sink.sleep_until_end();

    Ok(())
}
