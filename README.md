# Vocab App (背单词) - Rust Rewrite

A vocabulary learning/flashcard app rewritten from Kotlin/Android to pure Rust using egui/eframe.

## Features

- **Import word banks** from TXT files (tab, " - ", " | ", ":", "：" delimited)
- **Two learning modes**: Flashcard (flip card + remember/forget) and Quiz (multiple choice)
- **Two directions**: Word → Definition, Definition → Word
- **Session persistence**: Paused sessions are saved and can be resumed
- **Statistics**: Learning history, accuracy tracking, wrong words review
- **Word management**: Search, star, add, edit, delete within word banks
- **Starred/Words review**: Quick review of starred words or frequently missed words

## Building

### Prerequisites

- Rust (install from https://rustup.rs)
- For Android cross-compilation: Android NDK + cargo-ndk

### Desktop Build

```bash
cd vocab-app-rust
cargo build --release
cargo run --release
```

The app database (`vocab_app.db`) is created automatically alongside the executable.

### Android Build

Building for Android requires additional setup:

1. Install Android NDK
2. Install cargo-ndk: `cargo install cargo-ndk`
3. Build: `cargo ndk -t arm64-v8a -o ../vocab-app-rust-android/jniLibs build --release`
4. Package with a minimal Android wrapper activity

## Project Structure

```
src/
  main.rs     - Entry point, eframe setup
  app.rs      - Main app struct, business logic, all UI rendering
  db.rs       - SQLite database layer (rusqlite)
  models.rs   - Data structures (Bank, Word, Session, Card, etc.)
  parser.rs   - TXT file parser
  theme.rs    - Colors and visual theme
```

## Data

The app uses SQLite via rusqlite with three tables:
- **banks**: Word bank metadata
- **words**: Individual word entries with starred/wrong tracking
- **sessions**: Learning session history

Session save data is stored as JSON in `saved_session.json` alongside the executable.

## License

MIT