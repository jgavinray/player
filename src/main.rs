use clap::{App, Arg};
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn main() {
    // Define and parse the command-line arguments
    let matches = App::new("MP3 Player")
        .version("0.1.0")
        .author("J. Gavin Ray")
        .about("Plays an MP3 file")
        .arg(
            Arg::with_name("FILE")
                .help("The MP3 file to play")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Get the file path from the command-line arguments
    let file_path = matches.value_of("FILE").unwrap();

    // Play the MP3 file
    play_mp3(file_path).expect("Error playing MP3 file");
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
