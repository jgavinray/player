use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::thread;
use std::path::Path;
use std::fs::File;
use std::io::{self, BufReader};
use rodio::{Decoder, OutputStream, Sink};
use crossterm::{
    execute,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    cursor::{MoveTo, MoveToNextLine},
    style::Print,
};
use clap::App;
use dialoguer::Select;
use walkdir::WalkDir;
use parking_lot::Mutex;

struct PlayerState {
    sink: Arc<Sink>,
    is_paused: AtomicBool,
    is_finished: AtomicBool,
    elapsed_seconds: AtomicU64,
    start_time: Instant,
    last_resume_time: Mutex<Instant>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

        mp3_files.push("Exit".into());

        let selection = Select::new()
            .with_prompt("Select an MP3 file to play or 'Exit' to quit")
            .default(0)
            .items(&mp3_files)
            .interact()?;

        if selection < mp3_files.len() - 1 {
            let file_path = &mp3_files[selection];
            play_mp3(file_path)?;
        } else {
            break;
        }
    }

    Ok(())
}

fn play_mp3(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = create_sink(&stream_handle, file_path)?;

    enable_raw_mode()?;
    setup_display(file_path)?;

    let state = create_player_state(sink);

    let player_thread = spawn_player_thread(state.clone());

    wait_for_playback_to_finish(&state.is_finished);

    cleanup_display()?;

    player_thread.join().unwrap();

    Ok(())
}

fn create_sink(stream_handle: &rodio::OutputStreamHandle, file_path: &str) -> Result<Arc<Sink>, Box<dyn std::error::Error>> {
    let file = File::open(Path::new(file_path))?;
    let source = Decoder::new_mp3(BufReader::new(file))?;
    let sink = Arc::new(Sink::try_new(stream_handle)?);
    sink.append(source);
    Ok(sink)
}

fn setup_display(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        MoveTo(0, 0),
        Print(format!("Playing: {}", file_path)),
        MoveToNextLine(1),
        Print("Press SPACE to pause/resume, 'q' to quit"),
        MoveToNextLine(1)
    )?;
    Ok(())
}

fn create_player_state(sink: Arc<Sink>) -> Arc<PlayerState> {
    let now = Instant::now();
    Arc::new(PlayerState {
        sink,
        is_paused: AtomicBool::new(false),
        is_finished: AtomicBool::new(false),
        elapsed_seconds: AtomicU64::new(0),
        start_time: now,
        last_resume_time: Mutex::new(now),
    })
}

fn spawn_player_thread(state: Arc<PlayerState>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            if handle_events(&state) {
                break;
            }
            update_elapsed_time(&state);
            if state.sink.empty() {
                state.is_finished.store(true, Ordering::SeqCst);
                break;
            }
        }
    })
}

fn handle_events(state: &PlayerState) -> bool {
    if event::poll(Duration::from_millis(100)).unwrap() {
        if let Ok(Event::Key(key_event)) = event::read() {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char(' ') => toggle_pause(state),
                    KeyCode::Char('q') => {
                        state.sink.stop();
                        state.is_finished.store(true, Ordering::SeqCst);
                        return true;
                    },
                    _ => {}
                }
            }
        }
    }
    false
}

fn toggle_pause(state: &PlayerState) {
    let was_paused = state.is_paused.fetch_xor(true, Ordering::SeqCst);
    let current_time = Instant::now();
    if was_paused {
        resume_playback(state, current_time);
    } else {
        pause_playback(state, current_time);
    }
}

fn resume_playback(state: &PlayerState, current_time: Instant) {
    state.sink.play();
    *state.last_resume_time.lock() = current_time;
    execute!(
        io::stdout(),
        MoveTo(0, 2),
        Clear(ClearType::CurrentLine),
        Print("Resumed")
    ).unwrap();
}

fn pause_playback(state: &PlayerState, current_time: Instant) {
    state.sink.pause();
    let last_resume = *state.last_resume_time.lock();
    let additional_elapsed = current_time.duration_since(last_resume).as_secs();
    state.elapsed_seconds.fetch_add(additional_elapsed, Ordering::SeqCst);
    let total_elapsed = state.elapsed_seconds.load(Ordering::SeqCst);
    let minutes = total_elapsed / 60;
    let seconds = total_elapsed % 60;
    execute!(
        io::stdout(),
        MoveTo(0, 2),
        Clear(ClearType::CurrentLine),
        Print(format!("Paused at {}:{:02}", minutes, seconds))
    ).unwrap();
}

fn update_elapsed_time(state: &PlayerState) {
    if !state.is_paused.load(Ordering::SeqCst) {
        let current_elapsed = state.start_time.elapsed().as_secs();
        state.elapsed_seconds.store(current_elapsed, Ordering::SeqCst);
    }
}

fn wait_for_playback_to_finish(is_finished: &AtomicBool) {
    while !is_finished.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }
}

fn cleanup_display() -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    Ok(())
}
