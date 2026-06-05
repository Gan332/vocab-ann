use eframe::egui::{self, *};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};
use crate::models::*;
use crate::theme::{VocabColors, ThemeColors};
use crate::db::Database;
use crate::parser;

#[derive(Clone, Serialize, Deserialize)]
struct SavedSession {
    bank_name: String, remembered: i32, index: i32, total: i32,
    direction: String, mode: String, elapsed: i64,
    #[serde(default)]
    forgotten: i32,
}

struct LearnState {
    bank_name: String, cards: Vec<Card>, index: usize,
    remembered: i32, forgotten: i32, total_cards: usize,
    mode: LearnMode, direction: Direction,
    start_time: f64, active: bool, flipped: bool, answered: bool,
}

struct QuizState {
    question: String, options: Vec<QuizOption>,
    selected: i32, answered: bool, correct: bool,
}

pub struct VocabApp {
    db: Database,
    pub current_tab: Tab,
    pub banks: Vec<Bank>,
    sessions: Vec<Session>,
    pub bank_words: Vec<Word>,
    cur_bank: String,
    search: String, search_prev: String,
    pub starred: i64, pub wrong: i64,
    learn: Option<LearnState>,
    paused: Option<LearnState>,
    quiz: Option<QuizState>,
    result: Option<SessionResult>,
    saved: Option<SavedSession>,
    error: Option<String>,
    timer: String,
    pub show_wm: bool, pub wm_bank: String,
    pub del_bank: Option<String>, del_word: Option<i64>,
    quit_confirm: bool, show_add: bool, edit_word: Option<Word>,
    pub show_import: bool,
    pub sel_bank: String, mode: LearnMode, dir: Direction,
    add_w: String, add_d: String, import_txt: String,
    pub rename_bank: Option<String>, rename_to: String,
    config: AppConfig, theme: ThemeColors,
}

impl VocabApp {
    pub fn new(db_path: &str) -> Self {
        let db = Database::new(db_path).expect("DB");
        let config = load_config();
        let theme = VocabColors::for_theme(config.dark_mode);
        let banks = db.get_all_banks();
        let sel = banks.first().map(|b| b.name.clone()).unwrap_or_default();
        VocabApp {
            db, current_tab: Tab::Banks, banks, sessions: vec![],
            bank_words: vec![], cur_bank: String::new(),
            search: String::new(), search_prev: String::new(),
            starred: db.get_starred_count(), wrong: db.get_wrong_words_count(),
            learn: None, paused: None, quiz: None, result: None,
            saved: load_saved(), error: None, timer: String::new(),
            show_wm: false, wm_bank: String::new(),
            del_bank: None, del_word: None, quit_confirm: false,
            show_add: false, edit_word: None, show_import: false,
            sel_bank: sel, mode: LearnMode::Flashcard, dir: Direction::WordFirst,
            add_w: String::new(), add_d: String::new(), import_txt: String::new(),
            rename_bank: None, rename_to: String::new(),
            config, theme,
        }
    }

    fn now(&self) -> f64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() }

    fn refresh_banks(&mut self) {
        self.banks = self.db.get_all_banks();
        self.starred = self.db.get_starred_count();
        self.wrong = self.db.get_wrong_words_count();
    }

    fn refresh_all(&mut self) {
        self.banks = self.db.get_all_banks();
        self.sessions = self.db.get_all_sessions();
        self.starred = self.db.get_starred_count();
        self.wrong = self.db.get_wrong_words_count();
    }

    fn import(&mut self, name: &str, text: &str) {
        let parsed = parser::parse_txt(text);
        if parsed.is_empty() { self.error = Some("No valid entries found".into()); return; }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.db.upsert_bank(&Bank {
            name: name.into(), count: parsed.len() as i32, created_at: now, updated_at: now,
        });
        self.db.delete_words_by_bank(name);
        let ws: Vec<Word> = parsed.iter().map(|p| Word {
            id: 0, bank_name: name.into(), word: p.word.clone(),
            definition: p.definition.clone(), is_starred: false, wrong_count: 0,
        }).collect();
        self.db.insert_words(&ws);
        self.refresh_banks();
        if self.sel_bank.is_empty() { self.sel_bank = name.into(); }
    }

    fn del_bank_impl(&mut self, name: &str) {
        self.db.delete_bank(name);
        self.refresh_banks();
        if self.sel_bank == name {
            self.sel_bank = self.banks.first().map(|b| b.name.clone()).unwrap_or_default();
        }
    }

    fn load_words(&mut self, name: &str) {
        self.cur_bank = name.into();
        self.bank_words = self.db.get_words_by_bank(name);
        self.search.clear(); self.search_prev.clear();
    }

    fn search_words(&mut self, q: &str) {
        if q == self.search_prev { return; }
        self.search_prev = q.to_string();
        if self.cur_bank.is_empty() { self.bank_words.clear(); return; }
        self.bank_words = if q.is_empty() {
            self.db.get_words_by_bank(&self.cur_bank)
        } else {
            self.db.search_words(&self.cur_bank, q)
        };
    }

    pub fn toggle_star(&mut self, id: i64) {
        if let Some(mut w) = self.db.get_word_by_id(id) {
            w.is_starred = !w.is_starred; self.db.update_word(&w);
            self.starred = self.db.get_starred_count();
        }
    }

    fn recount(&mut self, name: &str) {
        let c = self.db.get_words_by_bank(name).len() as i32;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let created_at = self.db.get_bank(name).map(|b| b.created_at).unwrap_or(now);
        self.db.upsert_bank(&Bank { name: name.into(), count: c, created_at, updated_at: now });
        self.refresh_banks();
    }

    fn start_learn(&mut self, bank: &str, mode: LearnMode, dir: Direction) {
        let ws = self.db.get_words_by_bank(bank);
        if ws.is_empty() { self.error = Some("Bank is empty".into()); return; }
        let mut cards: Vec<Card> = ws.iter().map(|w| Card {
            id: w.id,
            front: if dir == Direction::WordFirst { w.word.clone() } else { w.definition.clone() },
            back: if dir == Direction::WordFirst { w.definition.clone() } else { w.word.clone() },
            is_starred: w.is_starred,
        }).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.learn = Some(LearnState {
            bank_name: bank.into(), cards, index: 0, remembered: 0, forgotten: 0,
            total_cards: cards.len(), mode: mode.clone(), direction: dir,
            start_time: self.now(), active: true, flipped: false, answered: false,
        });
        self.paused = None;
        if mode == LearnMode::Quiz { self.gen_quiz(); } else { self.quiz = None; }
    }

    pub fn start_starred(&mut self) {
        let ws = self.db.get_all_starred_words();
        if ws.is_empty() { self.error = Some("No starred words".into()); return; }
        let mut cards: Vec<Card> = ws.iter().map(|w| Card {
            id: w.id, front: w.word.clone(), back: w.definition.clone(), is_starred: true,
        }).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.learn = Some(LearnState {
            bank_name: "Starred".into(), cards, index: 0, remembered: 0, forgotten: 0,
            total_cards: cards.len(), mode: LearnMode::Flashcard,
            direction: Direction::WordFirst, start_time: self.now(), active: true,
            flipped: false, answered: false,
        });
        self.paused = None; self.quiz = None;
    }

    pub fn start_wrong(&mut self) {
        let ws = self.db.get_all_wrong_words();
        if ws.is_empty() { self.error = Some("No wrong words".into()); return; }
        let mut cards: Vec<Card> = ws.iter().map(|w| Card {
            id: w.id, front: w.word.clone(), back: w.definition.clone(), is_starred: w.is_starred,
        }).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.learn = Some(LearnState {
            bank_name: "Wrong".into(), cards, index: 0, remembered: 0, forgotten: 0,
            total_cards: cards.len(), mode: LearnMode::Flashcard,
            direction: Direction::WordFirst, start_time: self.now(), active: true,
            flipped: false, answered: false,
        });
        self.paused = None; self.quiz = None;
    }

    fn resume(&mut self) {
        let s = match self.saved.take() { Some(x) => x, None => return };
        let ws = self.db.get_words_by_bank(&s.bank_name);
        if ws.is_empty() { self.error = Some("Bank gone".into()); clear_saved(); return; }
        let dir = if s.direction == "DEF_FIRST" { Direction::DefFirst } else { Direction::WordFirst };
        let mode = if s.mode == "QUIZ" { LearnMode::Quiz } else { LearnMode::Flashcard };
        let cards: Vec<Card> = ws.iter().map(|w| Card {
            id: w.id,
            front: if dir == Direction::WordFirst { w.word.clone() } else { w.definition.clone() },
            back: if dir == Direction::WordFirst { w.definition.clone() } else { w.word.clone() },
            is_starred: w.is_starred,
        }).collect();
        clear_saved();
        self.learn = Some(LearnState {
            bank_name: s.bank_name, cards, index: s.index as usize,
            remembered: s.remembered, forgotten: s.forgotten, total_cards: s.total as usize,
            mode: mode.clone(), direction: dir,
            start_time: self.now() - s.elapsed as f64 / 1000.0,
            active: true, flipped: false, answered: false,
        });
        self.paused = None;
        if mode == LearnMode::Quiz { self.gen_quiz(); }
    }

    fn has_saved(&self) -> bool { self.saved.is_some() }
    fn saved_info(&self) -> String {
        self.saved.as_ref().map(|s| format!("{} ({}/{})", s.bank_name, s.index, s.total)).unwrap_or_default()
    }

    fn save_session(&self) {
        let l = match self.learn { Some(ref x) => x, None => return };
        if !l.active || l.index == 0 || l.bank_name == "Starred" || l.bank_name == "Wrong" { return; }
        let el = ((self.now() - l.start_time) * 1000.0) as i64;
        save_saved(&SavedSession {
            bank_name: l.bank_name.clone(), remembered: l.remembered,
            index: l.index as i32, total: l.total_cards as i32,
            direction: if l.direction == Direction::DefFirst { "DEF_FIRST".into() } else { "WORD_FIRST".into() },
            mode: if l.mode == LearnMode::Quiz { "QUIZ".into() } else { "FLASHCARD".into() },
            elapsed: el, forgotten: l.forgotten,
        });
    }

    fn answer_card(&mut self, remembered: bool) {
        let result = {
            let l = match self.learn { Some(ref x) => x, None => return };
            if !l.active { return; }
            (l.index, l.cards.get(l.index).map(|c| c.id), l.index + 1 >= l.total_cards)
        };
        if let Some(id) = result.1 { if !remembered { self.inc_wrong(id); } }
        let l = self.learn.as_mut().unwrap();
        if remembered { l.remembered += 1; } else { l.forgotten += 1; }
        if result.2 { self.finish(); }
        else { l.index = result.0 + 1; l.flipped = false; }
    }

    fn inc_wrong(&mut self, id: i64) {
        if let Some(mut w) = self.db.get_word_by_id(id) {
            w.wrong_count += 1; self.db.update_word(&w);
            self.wrong = self.db.get_wrong_words_count();
        }
    }

    fn gen_quiz(&mut self) {
        let l = match self.learn { Some(ref x) => x, None => return };
        if !l.active { return; }
        let card = match l.cards.get(l.index) { Some(c) => c, None => return };
        let mut dst: Vec<&str> = l.cards.iter()
            .enumerate().filter(|(i,_)| *i != l.index)
            .map(|(_,c)| c.back.as_str()).collect();
        dst.sort(); dst.dedup(); dst.shuffle(&mut rand::thread_rng());
        let opts: Vec<QuizOption> = dst.into_iter().take(3)
            .map(|t| QuizOption { text: t.to_string(), is_correct: false }).collect();
        let mut opts = opts;
        opts.push(QuizOption { text: card.back.clone(), is_correct: true });
        opts.shuffle(&mut rand::thread_rng());
        self.quiz = Some(QuizState {
            question: card.front.clone(), options: opts,
            selected: -1, answered: false, correct: false,
        });
    }

    fn answer_quiz(&mut self, idx: usize) {
        let (correct, wid, finish) = {
            let q = match self.quiz { Some(ref x) => x, None => return };
            if q.answered { return; }
            let l = match self.learn { Some(ref x) => x, None => return };
            if !l.active { return; }
            let c = q.options.get(idx).map(|o| o.is_correct).unwrap_or(false);
            (c, l.cards.get(l.index).map(|c| c.id), l.index + 1 >= l.total_cards)
        };
        if let Some(ref mut q) = self.quiz { q.selected = idx as i32; q.answered = true; q.correct = correct; }
        if let Some(ref mut l) = self.learn { if correct { l.remembered += 1; } else { l.forgotten += 1; } }
        if !correct { if let Some(id) = wid { self.inc_wrong(id); } }
        if finish { self.finish(); }
    }

    fn next_quiz(&mut self) {
        if let Some(ref mut l) = self.learn {
            if l.active && l.index + 1 < l.total_cards {
                l.index += 1; self.gen_quiz();
            }
        }
    }

    fn finish(&mut self) {
        let l = match self.learn.take() { Some(x) => x, None => return };
        let dur = (self.now() - l.start_time) as i64;
        let acc = if l.total_cards > 0 { l.remembered * 100 / l.total_cards as i32 } else { 0 };
        let avg = if l.total_cards > 0 { dur / l.total_cards as i64 } else { 0 };
        self.result = Some(SessionResult {
            bank_name: l.bank_name.clone(), mode: l.mode.clone(),
            total: l.total_cards as i32, remembered: l.remembered,
            forgotten: l.forgotten, accuracy: acc, duration: dur, avg_time: avg,
        });
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.db.insert_session(&Session {
            id: 0, bank_name: l.bank_name, mode: l.mode.to_str().into(),
            total: l.total_cards as i32, remembered: l.remembered,
            forgotten: l.forgotten, accuracy: acc, duration: dur as i32, date: now,
        });
        self.sessions = self.db.get_all_sessions();
    }

    pub fn quit(&mut self) { self.learn = None; self.paused = None; self.quiz = None; self.result = None; }

    fn pause(&mut self) { self.paused = self.learn.take(); self.save_session(); }
    fn cont(&mut self) { self.learn = self.paused.take(); }
    fn save_quit(&mut self) { self.save_session(); self.learn = None; self.paused = None; self.quiz = None; self.result = None; }

    fn summary(&self) -> StatsSummary {
        if self.sessions.is_empty() { return StatsSummary { session_count: 0, avg_accuracy: 0.0, total_minutes: 0 }; }
        let c = self.sessions.len() as i64;
        let ta: i32 = self.sessions.iter().map(|s| s.accuracy).sum();
        let td: i32 = self.sessions.iter().map(|s| s.duration).sum();
        StatsSummary { session_count: c, avg_accuracy: ta as f64 / c as f64, total_minutes: td as i64 / 60 }
    }

    fn clear_wrong(&mut self) {
        for w in self.db.get_all_wrong_words() {
            let mut x = w; x.wrong_count = 0; self.db.update_word(&x);
        }
        self.wrong = 0;
    }
}


// ============================================================================
//  UI RENDERING (eframe::App impl + screen methods)
// ============================================================================

impl eframe::App for VocabApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Timer update
        if let Some(ref l) = self.learn {
            if l.active {
                let e = (self.now() - l.start_time) as i64;
                let h = e / 3600; let m = (e % 3600) / 60; let s = e % 60;
                self.timer = if h > 0 { format!("{}:{:02}:{:02}", h, m, s) } else { format!("{:02}:{:02}", m, s) };
            }
        }

        // Style
        let mut st = (*ctx.style()).clone();
        st.visuals.dark_mode = self.config.dark_mode;
        if self.config.dark_mode {
            st.visuals.panel_fill = self.theme.bg;
            st.visuals.window_fill = self.theme.card_bg;
        }
        ctx.set_style(st);

        let is_active = self.learn.as_ref().map_or(false, |l| l.active);
        let has_paused = self.paused.is_some();
        let has_result = self.result.is_some();

        if is_active {
            self.render_learn_active(ctx);
        } else if has_paused {
            Area::new("pover").fixed_pos([0.0, 0.0]).interactable(false).show(ctx, |ui| {
                ui.painter().rect_filled(ui.max_rect(), Rounding::ZERO,
                    Color32::from_black_alpha(120));
            });
            let mut keep_paused = true;
            Window::new("Paused").id("pw".into())
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false).resizable(false).title_bar(false)
                .frame(Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(20.0)))
                .open(&mut keep_paused)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Paused").size(20.0).strong().color(self.theme.text_primary));
                        ui.label(RichText::new("Progress saved").size(13.0).color(self.theme.text_secondary));
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized(Vec2::new(100.0, 36.0),
                                Button::new("Continue").fill(self.theme.primary).text_color(Color32::WHITE).rounding(Rounding::same(8.0))).clicked()
                            { self.cont(); keep_paused = false; }
                            if ui.add_sized(Vec2::new(100.0, 36.0),
                                Button::new("Save & Quit").rounding(Rounding::same(8.0))).clicked()
                            { self.save_quit(); keep_paused = false; }
                        });
                    });
                });
            if !keep_paused { self.paused = None; }
        } else if has_result {
            self.render_result(ctx);
        } else {
            self.render_tabs(ctx);
            self.show_error(ctx);
            self.show_word_mgmt(ctx);
            self.show_import_dialog(ctx);
            self.show_add_edit_dialog(ctx);
            self.show_delete_confirm(ctx);
            self.show_quit_confirm(ctx);
            self.show_rename_dialog(ctx);
        }    fn render_learn_active(&mut self, ctx: &Context) {
        TopBottomPanel::top("hdr").show(ctx, |ui| {
            let l = self.learn.as_ref().unwrap();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} / {}", l.index + 1, l.total_cards))
                    .size(14.0).strong().color(self.theme.text_primary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(&self.timer).size(14.0).monospace()
                        .color(self.theme.text_primary));
                    if ui.add(Button::new(" Exit ").fill(self.theme.wrong_light)
                        .text_color(self.theme.wrong).rounding(Rounding::same(6.0))).clicked()
                    { self.quit_confirm = true; }
                    if ui.add(Button::new(" Pause ").fill(self.theme.surface)
                        .rounding(Rounding::same(6.0))).clicked()
                    { self.pause(); }
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| {
            let is_flashcard = self.learn.as_ref().map_or(true, |l| l.mode == LearnMode::Flashcard);
            if is_flashcard { self.flashcard_ui(ui); } else { self.quiz_ui(ui); }
        });
    }

    fn flashcard_ui(&mut self, ui: &mut Ui) {
        let (card, is_flipped) = {
            let l = self.learn.as_ref().unwrap();
            (l.cards.get(l.index).cloned(), l.flipped)
        };
        let card = match card { Some(c) => c, None => return };

        let card_rect = Frame::none().fill(self.theme.card_bg)
            .rounding(Rounding::same(20.0)).stroke(Stroke::new(1.0, self.theme.border))
            .inner_margin(Margin::symmetric(24.0, 24.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(RichText::new(&card.front).size(28.0).strong()
                        .color(self.theme.text_primary));
                    ui.add_space(16.0);
                    if is_flipped {
                        ui.label(RichText::new(&card.back).size(18.0)
                            .color(self.theme.text_secondary));
                    } else {
                        ui.label(RichText::new("Tap to reveal").size(13.0)
                            .color(self.theme.text_secondary.linear_multiply(0.6)));
                    }
                    ui.add_space(60.0);
                });
            }).response;

        if card_rect.clicked() {
            if let Some(ref mut l) = self.learn { l.flipped = true; }
        }

        if is_flipped {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.add_sized(Vec2::new(ui.available_width() / 2.0 - 4.0, 48.0),
                    Button::new("Forgot").fill(self.theme.wrong_light)
                        .text_color(self.theme.wrong).rounding(Rounding::same(12.0)))
                    .clicked()
                { self.answer_card(false); }
                if ui.add_sized(Vec2::new(ui.available_width() / 2.0 - 4.0, 48.0),
                    Button::new("Remembered").fill(self.theme.correct_light)
                        .text_color(self.theme.correct).rounding(Rounding::same(12.0)))
                    .clicked()
                { self.answer_card(true); }
            });
        }
    }

    fn quiz_ui(&mut self, ui: &mut Ui) {
        if self.quiz.is_none() { self.gen_quiz(); return; }
        let (total, next_avail) = {
            let l = self.learn.as_ref().unwrap();
            (l.total_cards, l.index + 1 < l.total_cards)
        };
        let q = self.quiz.as_ref().unwrap().clone();

        Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(20.0))
            .stroke(Stroke::new(1.0, self.theme.border))
            .inner_margin(Margin::symmetric(24.0, 24.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Choose the correct answer")
                        .size(13.0).color(self.theme.text_secondary));
                    ui.add_space(8.0);
                    ui.label(RichText::new(&q.question).size(24.0).strong()
                        .color(self.theme.text_primary));
                    ui.add_space(8.0);
                });
            });
        ui.add_space(16.0);

        for (i, opt) in q.options.iter().enumerate() {
            let sel = q.selected == i as i32;
            let cor = opt.is_correct;
            let bg = if q.answered && cor { self.theme.correct_light }
                else if q.answered && sel && !cor { self.theme.wrong_light }
                else { self.theme.card_bg };
            let tc = if q.answered && cor { self.theme.correct }
                else if q.answered && sel && !cor { self.theme.wrong }
                else { self.theme.text_primary };
            let bc = if q.answered && cor { self.theme.correct }
                else if q.answered && sel && !cor { self.theme.wrong }
                else { self.theme.border };
            if ui.add_sized(Vec2::new(ui.available_width(), 44.0),
                Button::new(RichText::new(&opt.text).color(tc))
                    .fill(bg).stroke(Stroke::new(2.0, bc))
                    .rounding(Rounding::same(12.0)))
                .clicked() && !q.answered
            { self.answer_quiz(i); }
        }

        if q.answered {
            ui.add_space(8.0);
            let fb_bg = if q.correct { self.theme.correct_light } else { self.theme.wrong_light };
            Frame::none().fill(fb_bg).rounding(Rounding::same(10.0))
                .inner_margin(Margin::symmetric(12.0, 12.0))
                .show(ui, |ui| {
                    let fb = if q.correct { self.theme.correct } else { self.theme.wrong };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if q.correct { "Correct!" } else { "Wrong!" }).color(fb).strong());
                        if !q.correct {
                            if let Some(ans) = q.options.iter().find(|o| o.is_correct) {
                                ui.label(RichText::new(format!("Answer: {}", ans.text)).size(13.0).color(fb));
                            }
                        }
                    });
                });
            if next_avail {
                ui.add_space(12.0);
                if ui.add_sized(Vec2::new(ui.available_width(), 44.0),
                    Button::new("Next").fill(self.theme.primary)
                        .text_color(Color32::WHITE).rounding(Rounding::same(12.0)))
                    .clicked()
                {
                    if q.correct { self.next_quiz(); }
                }
                // For wrong answers, still show next button
                if !q.correct {
                    if ui.add_sized(Vec2::new(ui.available_width(), 44.0),
                        Button::new("Next ->").rounding(Rounding::same(12.0)))
                        .clicked()
                    { self.next_quiz(); }
                }
            }
        }
    }

    }
    fn render_result(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            let r = self.result.as_ref().unwrap();
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                let title = if r.mode == LearnMode::Quiz { "Quiz Complete!" } else { "Session Complete!" };
                ui.label(RichText::new(title).size(24.0).strong().color(self.theme.text_primary));
                ui.label(RichText::new(format!("{} - {}",
                    r.bank_name,
                    if r.mode == LearnMode::Quiz { "Quiz" } else { "Flashcard" }))
                    .size(14.0).color(self.theme.text_secondary));
                ui.add_space(20.0);

                ui.horizontal(|ui| { stat_card(ui, "Total", &r.total.to_string(), &self.theme);
                    stat_card(ui, "Correct", &r.remembered.to_string(), &self.theme);
                    stat_card(ui, "Accuracy", &format!("{}%", r.accuracy), &self.theme);
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| { stat_card(ui, "Time", &format!("{}:{:02}", r.duration / 60, r.duration % 60), &self.theme);
                    stat_card(ui, "Avg", &format!("{}s", r.avg_time), &self.theme);
                    stat_card(ui, "Wrong", &r.forgotten.to_string(), &self.theme);
                });
                ui.add_space(24.0);
                // Capture result data before button interaction
                let bname = r.bank_name.clone();
                let mode = r.mode.clone();
                ui.horizontal(|ui| {
                    if ui.add_sized(Vec2::new(120.0, 40.0),
                        Button::new("Again").fill(self.theme.primary)
                            .text_color(Color32::WHITE).rounding(Rounding::same(8.0)))
                        .clicked()
                    {
                        self.result = None;
                        if bname == "Starred" { self.start_starred(); }
                        else if bname == "Wrong" { self.start_wrong(); }
                        else { self.start_learn(&bname, mode, self.dir.clone()); }
                    }
                    if ui.add_sized(Vec2::new(120.0, 40.0),
                        Button::new("Back").rounding(Rounding::same(8.0)))
                        .clicked()
                    { self.result = None; }
                });
            });
        });
    }

    // ---------------------------------------------------------------
    //  TABS + BOTTOM NAV
    // ---------------------------------------------------------------
    fn render_tabs(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let items = [(Tab::Banks, "Banks"), (Tab::Learn, "Learn"), (Tab::Stats, "Stats")];
                for (tab, label) in &items {
                    let sel = self.current_tab == *tab;
                    let b = Button::new(RichText::new(*label).size(14.0))
                        .fill(if sel { self.theme.primary } else { Color32::TRANSPARENT })
                        .text_color(if sel { Color32::WHITE } else { self.theme.text_primary })
                        .rounding(Rounding::same(8.0))
                        .min_size(Vec2::new(ui.available_width() / 3.0 - 6.0, 36.0));
                    if ui.add(b).clicked() { self.current_tab = tab.clone(); }
                }
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                match self.current_tab {
                    Tab::Banks => self.banks_ui(ui),
                    Tab::Learn => self.learn_setup_ui(ui),
                    Tab::Stats => self.stats_ui(ui),
                }
            });
        });
    }

    // ---------------------------------------------------------------
    //  BANKS TAB
    // ---------------------------------------------------------------
    fn banks_ui(&mut self, ui: &mut Ui) {
        if self.starred > 0 || self.wrong > 0 {
            ui.horizontal(|ui| {
                if self.starred > 0 {
                    let r = Frame::none().fill(self.theme.star_light).rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(12.0, 12.0)).show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label("Starred"); ui.label(RichText::new(self.starred.to_string())
                                    .size(18.0).strong().color(self.theme.star));
                            });
                        }).response;
                    if r.clicked() { self.start_starred(); }
                }
                if self.wrong > 0 {
                    let r = Frame::none().fill(self.theme.wrong_light).rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(12.0, 12.0)).show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label("Wrong"); ui.label(RichText::new(self.wrong.to_string())
                                    .size(18.0).strong().color(self.theme.wrong));
                            });
                        }).response;
                    if r.clicked() { self.start_wrong(); }
                }
            });
            ui.add_space(8.0);
        }

        ui.label(RichText::new("My Banks").size(18.0).strong().color(self.theme.text_primary));
        ui.add_space(8.0);

        // Import button
        Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(12.0))
            .stroke(Stroke::new(2.0, self.theme.border)).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if ui.add_sized(Vec2::new(ui.available_width(), 48.0),
                    Button::new("Import TXT").fill(Color32::TRANSPARENT).stroke(Stroke::NONE))
                    .clicked()
                { self.show_import = true; self.import_txt.clear(); }
            });
        ui.add_space(8.0);

        if self.banks.is_empty() {
            ui.vertical_centered(|ui| { ui.add_space(80.0); ui.label("No banks yet."); });
        } else {
            for bank in &self.banks {
                let n = bank.name.clone();
                let resp = Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(12.0))
                    .stroke(Stroke::new(1.0, self.theme.border))
                    .inner_margin(Margin::symmetric(12.0, 12.0)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&n).size(15.0).strong()
                                    .color(self.theme.text_primary));
                                ui.label(RichText::new(format!("{} words", bank.count)).size(12.0)
                                    .color(self.theme.text_secondary));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add_sized(Vec2::new(32.0, 28.0),
                                    Button::new("X").fill(self.theme.wrong_light)
                                        .text_color(self.theme.wrong).rounding(Rounding::same(6.0)))
                                    .clicked()
                                { self.del_bank = Some(n.clone()); }
                                if ui.add_sized(Vec2::new(56.0, 28.0),
                                    Button::new("Study").fill(self.theme.primary)
                                        .text_color(Color32::WHITE).rounding(Rounding::same(6.0)))
                                    .clicked()
                                { self.sel_bank = n; self.current_tab = Tab::Learn; }
                            });
                        });
                    }).response;
                resp.context_menu(|ui| {
                    if ui.button("Manage Words").clicked() {
                        self.wm_bank = bank.name.clone(); self.show_wm = true;
                        self.load_words(&bank.name); ui.close_menu();
                    }
                    if ui.button("Rename").clicked() {
                        self.rename_bank = Some(bank.name.clone());
                        self.rename_to = bank.name.clone();
                        ui.close_menu();
                    }
                });
                ui.add_space(4.0);
            }
        }
    }

    // ---------------------------------------------------------------
    //  LEARN SETUP TAB
    // ---------------------------------------------------------------
    fn learn_setup_ui(&mut self, ui: &mut Ui) {
        // Quick review cards
        if self.starred > 0 || self.wrong > 0 {
            ui.horizontal(|ui| {
                if self.starred > 0 {
                    let r = Frame::none().fill(self.theme.star_light).rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(12.0, 12.0)).show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label("Starred Review");
                                ui.label(RichText::new(self.starred.to_string()).size(16.0).strong()
                                    .color(self.theme.star));
                            });
                        }).response;
                    if r.clicked() { self.start_starred(); }
                }
                if self.wrong > 0 {
                    let r = Frame::none().fill(self.theme.wrong_light).rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(12.0, 12.0)).show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label("Wrong Review");
                                ui.label(RichText::new(self.wrong.to_string()).size(16.0).strong()
                                    .color(self.theme.wrong));
                            });
                        }).response;
                    if r.clicked() { self.start_wrong(); }
                }
            });
            ui.add_space(8.0);
        }

        // Resume
        if self.has_saved() {
            Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0, self.theme.border))
                .inner_margin(Margin::symmetric(16.0, 16.0)).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Saved session:").size(13.0)
                            .color(self.theme.text_secondary));
                        ui.label(RichText::new(self.saved_info()).size(14.0).strong()
                            .color(self.theme.text_primary));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized(Vec2::new(80.0, 32.0),
                                Button::new("Resume").fill(self.theme.primary)
                                    .text_color(Color32::WHITE).rounding(Rounding::same(8.0)))
                                .clicked()
                            { self.resume(); }
                            if ui.add_sized(Vec2::new(80.0, 32.0),
                                Button::new("Discard").rounding(Rounding::same(8.0)))
                                .clicked()
                            { self.saved = None; clear_saved(); }
                        });
                    });
                });
            ui.add_space(8.0);
        }

        // Setup
        Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(16.0))
            .inner_margin(Margin::symmetric(20.0, 20.0)).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Study Setup").size(18.0).strong()
                        .color(self.theme.text_primary));
                    ui.add_space(12.0);

                    if self.banks.is_empty() {
                        ui.label("No banks available.");
                    } else {
                        ComboBox::from_id_salt("bsel")
                            .selected_text(&self.sel_bank).width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for b in &self.banks {
                                    let n = b.name.clone();
                                    if ui.selectable_label(self.sel_bank == n, &n).clicked() {
                                        self.sel_bank = n;
                                    }
                                }
                            });
                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            let w = if ui.selectable_label(self.dir == Direction::WordFirst, "Word -> Def").clicked() { Direction::WordFirst } else { self.dir.clone() };
                            let d = if ui.selectable_label(self.dir == Direction::DefFirst, "Def -> Word").clicked() { Direction::DefFirst } else { self.dir.clone() };
                            self.dir = if w != self.dir { w } else if d != self.dir { d } else { self.dir.clone() };
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let f = if ui.selectable_label(self.mode == LearnMode::Flashcard, "Flashcard").clicked() { LearnMode::Flashcard } else { self.mode.clone() };
                            let q = if ui.selectable_label(self.mode == LearnMode::Quiz, "Quiz").clicked() { LearnMode::Quiz } else { self.mode.clone() };
                            self.mode = if f != self.mode { f } else if q != self.mode { q } else { self.mode.clone() };
                        });
                        ui.add_space(16.0);

                        if ui.add_sized(Vec2::new(ui.available_width(), 44.0),
                            Button::new("Start Studying").fill(self.theme.primary)
                                .text_color(Color32::WHITE).rounding(Rounding::same(10.0)))
                            .clicked()
                        { self.start_learn(&self.sel_bank, self.mode.clone(), self.dir.clone()); }
                    }
                });
            });
    }

    // ---------------------------------------------------------------
    //  STATS TAB
    // ---------------------------------------------------------------
    fn stats_ui(&mut self, ui: &mut Ui) {
        let s = self.summary();
        ui.label(RichText::new("Statistics").size(18.0).strong().color(self.theme.text_primary));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            stat_card(ui, "Sessions", &s.session_count.to_string(), &self.theme);
            stat_card(ui, "Avg Acc", &format!("{:.0}%", s.avg_accuracy), &self.theme);
            stat_card(ui, "Time", &format!("{}m", s.total_minutes), &self.theme);
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let _ = Frame::none().fill(self.theme.star_light).rounding(Rounding::same(12.0))
                .inner_margin(Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
                    ui.vertical_centered(|ui| { ui.label("Starred"); ui.label(RichText::new(self.starred.to_string()).size(18.0).strong().color(self.theme.star)); });
                });
            let _ = Frame::none().fill(self.theme.wrong_light).rounding(Rounding::same(12.0))
                .inner_margin(Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
                    ui.vertical_centered(|ui| { ui.label("Wrong"); ui.label(RichText::new(self.wrong.to_string()).size(18.0).strong().color(self.theme.wrong)); });
                });
        });

        if self.wrong > 0 {
            ui.add_space(8.0);
            if ui.add(Button::new("Clear Wrong").fill(self.theme.wrong_light)
                .text_color(self.theme.wrong).rounding(Rounding::same(8.0)))
                .clicked()
            { self.clear_wrong(); }
        }

        ui.add_space(12.0);
        ui.label(RichText::new("History").size(16.0).strong().color(self.theme.text_primary));
        ui.add_space(8.0);

        if self.sessions.is_empty() {
            ui.vertical_centered(|ui| { ui.add_space(40.0); ui.label("No sessions yet"); });
        } else {
            for ses in &self.sessions {
                Frame::none().fill(self.theme.card_bg).rounding(Rounding::same(10.0))
                    .stroke(Stroke::new(1.0, self.theme.border))
                    .inner_margin(Margin::symmetric(12.0, 10.0)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&ses.bank_name).size(14.0).strong()
                                    .color(self.theme.text_primary));
                                ui.label(RichText::new(format!("{} cards - {} correct", ses.total, ses.remembered))
                                    .size(12.0).color(self.theme.text_secondary));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(format!("{}%", ses.accuracy)).size(16.0).strong()
                                    .color(if ses.accuracy >= 80 { self.theme.correct } else { self.theme.wrong }));
                                ui.label(RichText::new(format!("{}:{:02}", ses.duration / 60, ses.duration % 60))
                                    .size(12.0).color(self.theme.text_secondary));
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        }
    }
}

// ============================================================================
//  HELPERS
// ============================================================================

fn stat_card(ui: &mut Ui, label: &str, value: &str, c: &ThemeColors) {
    Frame::none().fill(c.card_bg).rounding(Rounding::same(10.0))
        .stroke(Stroke::new(1.0, c.border))
        .inner_margin(Margin::symmetric(8.0, 10.0)).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(value).size(18.0).strong().color(c.text_primary));
                ui.label(RichText::new(label).size(11.0).color(c.text_secondary));
            });
        });
}

// ============================================================================
//  DIALOGS
// ============================================================================

impl VocabApp {
    fn show_error(&mut self, ctx: &Context) {
        if self.error.is_none() { return; }
        let err = self.error.take().unwrap();
        let mut open = true;
        Window::new("Error").id("err".into())
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false).resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(&err);
                if ui.button("OK").clicked() { open = false; }
            });
        if open { self.error = Some(err); } // still showing
    }

    fn show_word_mgmt(&mut self, ctx: &Context) {
        if !self.show_wm { return; }
        let bank_name = self.wm_bank.clone();
        let mut open = true;
        Window::new(&bank_name).id("wm".into())
            .open(&mut open).collapsible(false)
            .default_size([360.0, 500.0])
            .show(ctx, |ui| {
                // Search
                ui.horizontal(|ui| {
                    let resp = ui.add_sized(Vec2::new(ui.available_width() - 60.0, 0.0),
                        TextEdit::singleline(&mut self.search).hint_text("Search..."));
                    if resp.changed() { self.search_words(&self.search); }
                    if ui.button("Clear").clicked() {
                        self.search.clear(); self.search_prev.clear();
                        self.load_words(&bank_name);
                    }
                });
                ui.add_space(4.0);
                if ui.button("+ Add Word").clicked() {
                    self.show_add = true; self.edit_word = None;
                    self.add_w.clear(); self.add_d.clear();
                }
                ui.label(format!("{} words", self.bank_words.len()));
                ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                    for w in self.bank_words.clone() {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&w.word).size(14.0).strong()
                                    .color(self.theme.text_primary));
                                ui.label(RichText::new(&w.definition).size(12.0)
                                    .color(self.theme.text_secondary));
                            });
                            if ui.button(if w.is_starred { "Unstar" } else { "Star" })
                                .clicked()
                            { self.toggle_star(w.id); self.load_words(&bank_name); }
                            if ui.button("Edit").clicked() {
                                self.edit_word = Some(w.clone()); self.show_add = true;
                                self.add_w = w.word.clone(); self.add_d = w.definition.clone();
                            }
                            if ui.button("Del").clicked() { self.del_word = Some(w.id); }
                        });
                    }
                });
            });
        if !open { self.show_wm = false; self.search.clear(); self.search_prev.clear(); }
    }

    fn show_import_dialog(&mut self, ctx: &Context) {
        if !self.show_import { return; }
        let mut open = true;
        Window::new("Import TXT").id("imp".into())
            .open(&mut open).collapsible(false).default_size([360.0, 300.0])
            .show(ctx, |ui| {
                ui.label("Paste word list (word - definition per line):");
                ui.add_sized(Vec2::new(ui.available_width(), 120.0),
                    TextEdit::multiline(&mut self.import_txt).desired_width(f32::INFINITY));
                ui.add_space(8.0);
                if ui.add_sized(Vec2::new(ui.available_width(), 36.0),
                    Button::new("Import").fill(self.theme.primary)
                        .text_color(Color32::WHITE).rounding(Rounding::same(8.0)))
                    .clicked()
                {
                    let text = self.import_txt.clone();
                    if !text.is_empty() {
                        self.import("Imported", &text);
                        self.import_txt.clear();
                        open = false;
                    }
                }
            });
        if !open { self.show_import = false; }
    }

    fn show_add_edit_dialog(&mut self, ctx: &Context) {
        if !self.show_add { return; }
        let is_edit = self.edit_word.is_some();
        let title = if is_edit { "Edit Word" } else { "Add Word" };
        let mut open = true;
        Window::new(title).id("aw".into())
            .open(&mut open).collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Word:").strong();
                    ui.text_edit_singleline(&mut self.add_w);
                });
                ui.horizontal(|ui| {
                    ui.label("Def:").strong();
                    ui.text_edit_singleline(&mut self.add_d);
                });
                ui.add_space(8.0);
                if ui.add_sized(Vec2::new(ui.available_width(), 32.0),
                    Button::new("OK").fill(self.theme.primary)
                        .text_color(Color32::WHITE).rounding(Rounding::same(8.0)))
                    .clicked()
                {
                    let w = self.add_w.trim().to_string();
                    let d = self.add_d.trim().to_string();
                    if !w.is_empty() && !d.is_empty() {
                        if is_edit {
                            if let Some(ref e) = self.edit_word {
                                self.db.update_word(&Word {
                                    id: e.id, bank_name: e.bank_name.clone(),
                                    word: w, definition: d,
                                    is_starred: e.is_starred, wrong_count: e.wrong_count,
                                });
                                self.recount(&e.bank_name);
                                self.load_words(&e.bank_name);
                            }
                        } else {
                            let bank = self.wm_bank.clone();
                            self.db.insert_word(&Word {
                                id: 0, bank_name: bank.clone(),
                                word: w, definition: d, is_starred: false, wrong_count: 0,
                            });
                            self.recount(&bank);
                            self.load_words(&bank);
                        }
                        self.edit_word = None;
                        self.show_add = false;
                        open = false;
                    }
                }
            });
        if !open { self.edit_word = None; self.add_w.clear(); self.add_d.clear(); }
    }

    fn show_delete_confirm(&mut self, ctx: &Context) {
        // Word delete
        if let Some(id) = self.del_word {
            self.del_word = None;
            let mut confirm = true;
            Window::new("Delete Word").id("dw".into())
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false).resizable(false)
                .open(&mut confirm)
                .show(ctx, |ui| {
                    ui.label("Delete this word?");
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.db.delete_word(id);
                            if !self.cur_bank.is_empty() { self.load_words(&self.cur_bank); self.recount(&self.cur_bank); }
                            confirm = false;
                        }
                        if ui.button("Cancel").clicked() { confirm = false; }
                    });
                });
        }

        // Bank delete
        if let Some(ref name) = self.del_bank.clone() {
            let n = name.clone();
            let mut confirm = true;
            Window::new("Delete Bank").id("db".into())
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false).resizable(false)
                .open(&mut confirm)
                .show(ctx, |ui| {
                    ui.label(format!("Delete '{}' and all its words?", &n));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.del_bank_impl(&n);
                            confirm = false;
                        }
                        if ui.button("Cancel").clicked() { confirm = false; }
                    });
                });
            if !confirm { self.del_bank = None; }
        }
    }

    fn show_quit_confirm(&mut self, ctx: &Context) {
        if !self.quit_confirm { return; }
        let mut confirm = true;
        Window::new("Quit").id("qc".into())
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false).resizable(false)
            .open(&mut confirm)
            .show(ctx, |ui| {
                ui.label("Quit and lose progress?");
                ui.horizontal(|ui| {
                    if ui.button("Quit").clicked() { self.quit(); confirm = false; self.quit_confirm = false; }
                    if ui.button("Continue").clicked() { confirm = false; self.quit_confirm = false; }
                });
            });
    }

    fn show_rename_dialog(&mut self, ctx: &Context) {
        if let Some(ref old_name) = self.rename_bank.clone() {
            let mut confirm = true;
            let old = old_name.clone();
            Window::new("Rename Bank").id("rn".into())
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false).resizable(false)
                .open(&mut confirm)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:"); ui.text_edit_singleline(&mut self.rename_to);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Rename").clicked() {
                            let new_name = self.rename_to.trim().to_string();
                            if !new_name.is_empty() && new_name != old {
                                // Rename by re-inserting
                                if let Some(bank) = self.db.get_bank(&old) {
                                    // Create new bank with new name
                                    self.db.upsert_bank(&Bank {
                                        name: new_name.clone(), count: bank.count,
                                        created_at: bank.created_at,
                                        updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
                                    });
                                    // Move words
                                    let words = self.db.get_words_by_bank(&old);
                                    for w in words {
                                        let mut w = w; w.bank_name = new_name.clone();
                                        self.db.update_word(&w);
                                    }
                                    // Delete old bank
                                    self.db.delete_bank(&old);
                                    self.refresh_banks();
                                    if self.sel_bank == old { self.sel_bank = new_name; }
                                }
                            }
                            self.rename_bank = None;
                            confirm = false;
                        }
                        if ui.button("Cancel").clicked() { confirm = false; self.rename_bank = None; }
                    });
                });
        }
    }
}

// ============================================================================
//  PERSISTENCE
// ============================================================================

fn session_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap_or_default();
    p.pop(); p.push("saved_session.json"); p
}

fn load_saved() -> Option<SavedSession> {
    let p = session_path();
    if p.exists() { std::fs::read_to_string(&p).ok().and_then(|j| serde_json::from_str(&j).ok()) } else { None }
}

fn save_saved(s: &SavedSession) {
    if let Ok(j) = serde_json::to_string(s) { let _ = std::fs::write(session_path(), j); }
}

fn clear_saved() {
    let p = session_path();
    if p.exists() { let _ = std::fs::remove_file(p); }
}

fn config_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap_or_default();
    p.pop(); p.push("config.json"); p
}

fn load_config() -> AppConfig {
    let p = config_path();
    if p.exists() {
        std::fs::read_to_string(&p).ok()
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default()
    } else { AppConfig::default() }
}

fn _save_config(cfg: &AppConfig) {
    if let Ok(j) = serde_json::to_string(cfg) { let _ = std::fs::write(config_path(), j); }
}

