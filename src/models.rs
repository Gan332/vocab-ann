use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bank {
    pub name: String,
    pub count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Word {
    pub id: i64,
    pub bank_name: String,
    pub word: String,
    pub definition: String,
    pub is_starred: bool,
    pub wrong_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub bank_name: String,
    pub mode: String,
    pub total: i32,
    pub remembered: i32,
    pub forgotten: i32,
    pub accuracy: i32,
    pub duration: i32,
    pub date: i64,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub id: i64,
    pub front: String,
    pub back: String,
    pub is_starred: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LearnMode {
    Flashcard,
    Quiz,
}

impl LearnMode {
    pub fn to_str(&self) -> &str {
        match self {
            LearnMode::Flashcard => "flashcard",
            LearnMode::Quiz => "quiz",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "quiz" => LearnMode::Quiz,
            _ => LearnMode::Flashcard,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    WordFirst,
    DefFirst,
}

#[derive(Debug, Clone)]
pub struct QuizOption {
    pub text: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone)]
pub struct SessionResult {
    pub bank_name: String,
    pub mode: LearnMode,
    pub total: i32,
    pub remembered: i32,
    pub forgotten: i32,
    pub accuracy: i32,
    pub duration: i64,
    pub avg_time: i64,
}

#[derive(Debug, Clone)]
pub struct StatsSummary {
    pub session_count: i64,
    pub avg_accuracy: f64,
    pub total_minutes: i64,
}

#[derive(Debug, Clone)]
pub struct ParsedWord {
    pub word: String,
    pub definition: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub dark_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { dark_mode: false }
    }
}

#[derive(Debug, Clone)]
pub enum Tab {
    Banks,
    Learn,
    Stats,
}
