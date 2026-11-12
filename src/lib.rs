//! CalcKeep: financial calculators in Rust + Slint.
//! The same code runs on desktop, in the browser (WASM) and on Android/iOS.

pub mod engine;
pub mod storage;

use engine::format::{self, affixes, field_display, field_raw, quick_label};
use engine::{catalog, Calc, Category, Field, Input, RowKind};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use storage::{HistoryEntry, Saved};

slint::include_modules!();

struct Keypad {
    field: usize,
    text: String,
    /// The first key press replaces the existing value (calculator behavior).
    fresh: bool,
}

struct State {
    cats: Vec<Category>,
    /// Values per (category, calculator), kept while the app runs.
    values: HashMap<(usize, usize), Vec<f64>>,
    cat: usize,
    calc: usize,
    fields: Rc<VecModel<FieldData>>,
    /// Real field index for each row shown in the UI (fields can be hidden)
    visible: Vec<usize>,
    keypad: Option<Keypad>,
    /// Persisted data (history, last inputs, theme)
    saved: Saved,
    /// History entry for the current visit to a calculator; changes during one
    /// visit update this entry instead of creating new ones.
    session_entry: Option<u64>,
    /// Inputs changed since the last save
    dirty: bool,
    /// Debounce generation counter for auto-save
    gen: u64,
    /// Page the back button returns to from a calculator
    return_page: i32,
}

impl State {
    fn current(&self) -> &Calc {
        &self.cats[self.cat].calcs[self.calc]
    }
    fn vals(&self) -> &[f64] {
        &self.values[&(self.cat, self.calc)]
    }
    fn set_val(&mut self, i: usize, v: f64) {
        let key = (self.cat, self.calc);
        self.values.get_mut(&key).unwrap()[i] = v;
    }
}

fn find_calc(cats: &[Category], name: &str) -> Option<(usize, usize)> {
    cats.iter()
        .enumerate()
        .find_map(|(ci, c)| c.calcs.iter().position(|k| k.name == name).map(|k| (ci, k)))
}

fn values_map(calc: &Calc, vals: &[f64]) -> HashMap<String, f64> {
    calc.fields
        .iter()
        .zip(vals)
        .map(|(f, v)| (f.key.to_string(), *v))
        .collect()
}

/// Map stored values onto a calculator's fields; unknown/missing keys use defaults.
fn apply_map(calc: &Calc, map: &HashMap<String, f64>) -> Vec<f64> {
    calc.fields
        .iter()
        .map(|f| map.get(f.key).map(|v| f.sanitize(*v)).unwrap_or(f.default))
        .collect()
}

fn s(x: impl AsRef<str>) -> SharedString {
    SharedString::from(x.as_ref())
}

fn strings(v: impl IntoIterator<Item = impl AsRef<str>>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(v.into_iter().map(s).collect::<Vec<_>>()))
}

fn field_data(f: &Field, v: f64) -> FieldData {
    let (kind, options) = match &f.input {
        Input::Keypad => (0, vec![]),
        Input::Slider { .. } => (1, vec![]),
        Input::Choice(opts) => (2, opts.clone()),
    };
    FieldData {
        label: s(f.label),
        help: s(f.help),
        kind,
        value: v as f32,
        display: s(field_display(f, v)),
        min: f.min as f32,
        max: f.max as f32,
        min_label: s(field_display(f, f.min)),
        max_label: s(field_display(f, f.max)),
        options: strings(options),
        selected: v as i32,
    }
}

fn refresh_field(st: &State, i: usize) {
    let f = &st.current().fields[i];
    if let Some(row) = st.visible.iter().position(|&v| v == i) {
        st.fields.set_row_data(row, field_data(f, st.vals()[i]));
    }
}

/// Rebuild the UI field list from the currently visible fields.
fn rebuild_fields(st: &mut State) {
    let vis = st.current().visible(st.vals());
    let c = st.current();
    let rows: Vec<FieldData> = vis
        .iter()
        .map(|&i| field_data(&c.fields[i], st.vals()[i]))
        .collect();
    st.fields.set_vec(rows);
    st.visible = vis;
}

/// After field `i` changed: update its row (or the whole list if fields
/// appeared/disappeared), recalc, and schedule the history save.
fn after_change(ui: &AppWindow, st: &mut State, rc: &Rc<RefCell<State>>, i: usize) {
    if st.current().visible(st.vals()) != st.visible {
        rebuild_fields(st);
    } else {
        refresh_field(st, i);
    }
    recompute(ui, st);
    mark_changed(ui, st, rc);
}

fn recompute(ui: &AppWindow, st: &State) {
    let out = st.current().run(st.vals());
    let rows: Vec<ResultRow> = out
        .rows
        .iter()
        .map(|r| ResultRow {
            label: s(&r.label),
            value: s(&r.value),
            kind: match r.kind {
                RowKind::Normal => 0,
                RowKind::Primary => 1,
                RowKind::Note => 2,
                RowKind::Warning => 3,
            },
        })
        .collect();
    // Headline for the sticky result bar on phones
    match out.rows.iter().find(|r| r.kind == RowKind::Primary) {
        Some(p) => {
            ui.set_headline_label(s(&p.label));
            ui.set_headline_value(s(&p.value));
        }
        None => {
            let w = out.rows.iter().find(|r| r.kind == RowKind::Warning);
            ui.set_headline_label(s(w.map(|w| w.label.as_str()).unwrap_or("")));
            ui.set_headline_value(s(if w.is_some() { "!" } else { "" }));
        }
    }
    ui.set_results(ModelRc::new(VecModel::from(rows)));
    let table = out.table.unwrap_or_default();
    ui.set_table_title(s(&table.title));
    ui.set_table_headers(strings(&table.headers));
    ui.set_table_rows(ModelRc::new(VecModel::from(
        table
            .rows
            .iter()
            .map(|r| TableRow { cells: strings(r) })
            .collect::<Vec<_>>(),
    )));
}

fn open_calc(ui: &AppWindow, st: &mut State, cat: usize, calc: usize) {
    st.cat = cat;
    st.calc = calc;
    if !st.values.contains_key(&(cat, calc)) {
        let k = &st.cats[cat].calcs[calc];
        let vals = match st.saved.last.get(k.name) {
            Some(map) => apply_map(k, map),
            None => k.defaults(),
        };
        st.values.insert((cat, calc), vals);
    }
    st.session_entry = None;
    st.dirty = false;
    rebuild_fields(st);
    let c = st.current();
    ui.set_calc_name(s(c.name));
    ui.set_calc_blurb(s(c.blurb));
    ui.set_cat_icon(st.cats[cat].icon);
    recompute(ui, st);
    ui.set_page(2);
}

fn open_category(ui: &AppWindow, st: &mut State, cat: usize) {
    st.cat = cat;
    let c = &st.cats[cat];
    if c.calcs.len() == 1 {
        // e.g. Savings & CD goes straight to its calculator
        st.return_page = 0;
        open_calc(ui, st, cat, 0);
        return;
    }
    ui.set_cat_name(s(c.name));
    ui.set_cat_icon(c.icon);
    ui.set_calc_items(ModelRc::new(VecModel::from(
        c.calcs
            .iter()
            .map(|k| CalcItem {
                name: s(k.name),
                blurb: s(k.blurb),
            })
            .collect::<Vec<_>>(),
    )));
    ui.set_page(1);
}

// ------------------------------------------------------------- history

fn refresh_history(ui: &AppWindow, st: &State) {
    let items: Vec<HistoryItem> = st
        .saved
        .history
        .iter()
        .map(|e| HistoryItem {
            calc: s(&e.calc),
            icon: find_calc(&st.cats, &e.calc)
                .map(|(c, _)| st.cats[c].icon)
                .unwrap_or(0),
            headline_label: s(&e.headline_label),
            headline_value: s(&e.headline_value),
            summary: s(&e.summary),
            when: s(storage::when(e.ts)),
        })
        .collect();
    ui.set_history(ModelRc::new(VecModel::from(items)));
}

/// Write the current calculator into history (one entry per visit) and save.
fn commit(ui: &AppWindow, st: &mut State) {
    if !st.dirty {
        return;
    }
    st.dirty = false;
    let calc = st.current();
    let vals = st.vals().to_vec();
    let out = calc.run(&vals);
    let head = out
        .rows
        .iter()
        .find(|r| r.kind == RowKind::Primary)
        .map(|r| (r.label.clone(), r.value.clone()))
        .or_else(|| {
            out.rows
                .iter()
                .find(|r| r.kind == RowKind::Warning)
                .map(|r| (r.label.clone(), String::new()))
        })
        .unwrap_or_default();
    let summary = calc
        .fields
        .iter()
        .zip(&vals)
        .take(3)
        .map(|(f, v)| field_display(f, *v))
        .collect::<Vec<_>>()
        .join(" · ");
    let map = values_map(calc, &vals);
    let name = calc.name.to_string();
    let id = st.session_entry.unwrap_or_else(|| st.saved.next_id());
    st.saved.upsert(HistoryEntry {
        id,
        calc: name.clone(),
        ts: storage::now_ms(),
        values: map.clone(),
        headline_label: head.0,
        headline_value: head.1,
        summary,
    });
    st.session_entry = Some(id);
    st.saved.last.insert(name, map);
    storage::save(&st.saved);
    refresh_history(ui, st);
}

/// Inputs changed: auto-save to history after a short pause.
fn mark_changed(ui: &AppWindow, st: &mut State, rc: &Rc<RefCell<State>>) {
    st.dirty = true;
    st.gen += 1;
    let gen = st.gen;
    let weak = ui.as_weak();
    let rc = rc.clone();
    slint::Timer::single_shot(std::time::Duration::from_millis(1200), move || {
        let (Some(ui), Ok(mut st)) = (weak.upgrade(), rc.try_borrow_mut()) else {
            return;
        };
        if st.gen == gen && ui.get_page() == 2 {
            commit(&ui, &mut st);
        }
    });
}

fn open_history_item(ui: &AppWindow, st: &mut State, i: usize) {
    let Some(e) = st.saved.history.get(i).cloned() else {
        return;
    };
    let Some((cat, k)) = find_calc(&st.cats, &e.calc) else {
        return;
    };
    let vals = apply_map(&st.cats[cat].calcs[k], &e.values);
    st.values.insert((cat, k), vals);
    st.return_page = 3;
    open_calc(ui, st, cat, k);
}

// ------------------------------------------------------------- keypad

fn keypad_text_display(f: &Field, text: &str) -> String {
    let (pre, suf) = affixes(f);
    let body = if text.is_empty() {
        "0".to_string()
    } else {
        format::group(text)
    };
    format!("{pre}{body}{suf}")
}

fn keypad_sync(ui: &AppWindow, st: &State) {
    if let Some(k) = &st.keypad {
        let f = &st.current().fields[k.field];
        ui.set_keypad_display(s(keypad_text_display(f, &k.text)));
    }
}

fn keypad_open(ui: &AppWindow, st: &mut State, i: usize) {
    let f = &st.current().fields[i];
    if let Input::Choice(_) = f.input {
        return;
    }
    let v = st.vals()[i];
    let text = field_raw(f, v);
    ui.set_keypad_label(s(f.label));
    let hint = match f.input {
        Input::Slider { .. } => format!(
            "Range {} – {}",
            field_display(f, f.min),
            field_display(f, f.max)
        ),
        _ if f.max < 1e11 => format!("Up to {}", field_display(f, f.max)),
        _ => String::new(),
    };
    ui.set_keypad_hint(s(hint));
    ui.set_keypad_quick(strings(f.quick.iter().map(|q| quick_label(f, *q))));
    st.keypad = Some(Keypad {
        field: i,
        text,
        fresh: true,
    });
    keypad_sync(ui, st);
    ui.set_keypad_open(true);
}

fn keypad_key(ui: &AppWindow, st: &mut State, key: &str) {
    let decimals = match &st.keypad {
        Some(k) => st.current().fields[k.field].decimals(),
        None => return,
    };
    let k = st.keypad.as_mut().unwrap();
    match key {
        "back" => {
            k.fresh = false;
            k.text.pop();
        }
        "clear" => {
            k.fresh = false;
            k.text.clear();
        }
        "." | "," => {
            if k.fresh {
                k.text.clear();
                k.fresh = false;
            }
            if decimals > 0 && !k.text.contains('.') {
                if k.text.is_empty() {
                    k.text.push('0');
                }
                k.text.push('.');
            }
        }
        d if d.len() == 1 && d.chars().all(|c| c.is_ascii_digit()) => {
            if k.fresh {
                k.text.clear();
                k.fresh = false;
            }
            if let Some(dot) = k.text.find('.') {
                if k.text.len() - dot > decimals {
                    return;
                }
            }
            if k.text.len() >= 12 {
                return;
            }
            if k.text == "0" {
                k.text.clear();
            }
            k.text.push_str(d);
        }
        _ => return,
    }
    keypad_sync(ui, st);
}

fn keypad_quick(ui: &AppWindow, st: &mut State, qi: usize) {
    let Some(k) = &st.keypad else { return };
    let f = &st.current().fields[k.field];
    let Some(q) = f.quick.get(qi).copied() else {
        return;
    };
    let cur: f64 = k.text.parse().unwrap_or(0.0);
    let v = f.sanitize(cur + q);
    let text = field_raw(f, v);
    let k = st.keypad.as_mut().unwrap();
    k.text = text;
    k.fresh = false;
    keypad_sync(ui, st);
}

fn keypad_done(ui: &AppWindow, st: &mut State, rc: &Rc<RefCell<State>>) {
    if let Some(k) = st.keypad.take() {
        let f = &st.current().fields[k.field];
        let v = f.sanitize(k.text.parse().unwrap_or(0.0));
        if (v - st.vals()[k.field]).abs() > 1e-9 {
            st.set_val(k.field, v);
            after_change(ui, st, rc, k.field);
        }
    }
    ui.set_keypad_open(false);
}

// ------------------------------------------------------------- app

/// `THIRD-PARTY-LICENSES.txt` beside this program (security-review-1 M8): installed by
/// the Windows setup and written by `cargo xtask release`. None on the web and in a
/// development build.
fn third_party_licences() -> Option<std::path::PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("THIRD-PARTY-LICENSES.txt")))
        .filter(|f| f.is_file())
}

/// Opens `THIRD-PARTY-LICENSES.txt` with the system's viewer for text files (About's
/// "Third-party licences").
fn open_third_party_licences() -> Result<(), String> {
    let file =
        third_party_licences().ok_or("THIRD-PARTY-LICENSES.txt isn't installed beside CalcKeep")?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let program = if cfg!(target_os = "windows") {
            "notepad"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(program)
            .arg(&file)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("couldn't open {}: {e}", file.display()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(format!("{} can't be opened here", file.display()))
    }
}

/// Owns the window and wires every callback to the calculator engine.
pub struct App {
    pub ui: AppWindow,
    state: Rc<RefCell<State>>,
}

impl App {
    pub fn new() -> Result<App, slint::PlatformError> {
        let ui = AppWindow::new()?;
        let cats = catalog::categories();
        ui.set_tiles(ModelRc::new(VecModel::from(
            cats.iter()
                .map(|c| TileData {
                    name: s(c.name),
                    subtitle: s(c.subtitle),
                    icon: c.icon,
                    count: c.calcs.len() as i32,
                })
                .collect::<Vec<_>>(),
        )));
        let fields = Rc::new(VecModel::<FieldData>::default());
        ui.set_fields(ModelRc::from(fields.clone()));
        ui.set_app_version(s(env!("CARGO_PKG_VERSION")));
        ui.set_licences_available(third_party_licences().is_some());
        ui.on_open_licences(|| {
            if let Err(e) = open_third_party_licences() {
                eprintln!("calckeep: {e}");
            }
        });
        let mut saved = storage::load();
        saved
            .history
            .retain(|e| find_calc(&cats, &e.calc).is_some());
        if let Some(dark) = saved.dark {
            ui.global::<Theme>().set_dark(dark);
        }
        let state = Rc::new(RefCell::new(State {
            cats,
            values: HashMap::new(),
            cat: 0,
            calc: 0,
            fields,
            visible: vec![],
            keypad: None,
            saved,
            session_entry: None,
            dirty: false,
            gen: 0,
            return_page: 0,
        }));
        refresh_history(&ui, &state.borrow());

        macro_rules! on {
            ($setter:ident, |$ui:ident, $st:ident, $rc:ident $(, $arg:ident : $t:ty)*| $body:block) => {{
                let weak = ui.as_weak();
                let state = state.clone();
                ui.$setter(move |$($arg: $t),*| {
                    let Some($ui) = weak.upgrade() else { return };
                    #[allow(unused_variables)]
                    let $rc = &state;
                    let mut guard = state.borrow_mut();
                    let $st: &mut State = &mut guard;
                    $body
                });
            }};
        }

        on!(on_open_category, |ui, st, rc, i: i32| {
            open_category(&ui, st, i as usize);
        });
        on!(on_open_calc, |ui, st, rc, i: i32| {
            let cat = st.cat;
            st.return_page = 1;
            open_calc(&ui, st, cat, i as usize);
        });
        on!(on_go_back, |ui, st, rc| {
            st.keypad = None;
            ui.set_keypad_open(false);
            if ui.get_page() == 2 {
                commit(&ui, st);
                st.session_entry = None;
                ui.set_page(st.return_page);
            } else {
                ui.set_page(0);
            }
        });
        on!(on_open_history, |ui, st, rc| {
            ui.set_page(3);
            refresh_history(&ui, st);
        });
        on!(on_open_about, |ui, _st, rc| {
            ui.set_page(4);
        });
        on!(on_open_history_item, |ui, st, rc, i: i32| {
            open_history_item(&ui, st, i as usize);
        });
        on!(on_delete_history_item, |ui, st, rc, i: i32| {
            let i = i as usize;
            if i < st.saved.history.len() {
                let removed = st.saved.history.remove(i);
                if st.session_entry == Some(removed.id) {
                    st.session_entry = None;
                }
                storage::save(&st.saved);
                refresh_history(&ui, st);
            }
        });
        on!(on_clear_history, |ui, st, rc| {
            st.saved.history.clear();
            st.session_entry = None;
            storage::save(&st.saved);
            refresh_history(&ui, st);
        });
        on!(on_theme_changed, |_ui, st, rc, dark: bool| {
            st.saved.dark = Some(dark);
            storage::save(&st.saved);
        });
        on!(on_field_slider, |ui, st, rc, i: i32, raw: f32| {
            let Some(&i) = st.visible.get(i as usize) else {
                return;
            };
            let f = &st.current().fields[i];
            let step = match f.input {
                Input::Slider { step } => step,
                _ => return,
            };
            let snapped = f.min + ((raw as f64 - f.min) / step).round() * step;
            let v = (f.sanitize(snapped) * 1e6).round() / 1e6;
            if (v - st.vals()[i]).abs() < 1e-9 {
                return;
            }
            st.set_val(i, v);
            after_change(&ui, st, rc, i);
        });
        on!(on_field_choice, |ui, st, rc, i: i32, j: i32| {
            let Some(&i) = st.visible.get(i as usize) else {
                return;
            };
            if st.vals()[i] as i32 == j {
                return;
            }
            st.set_val(i, j as f64);
            after_change(&ui, st, rc, i);
        });
        on!(on_field_keypad, |ui, st, rc, i: i32| {
            if let Some(&i) = st.visible.get(i as usize) {
                keypad_open(&ui, st, i);
            }
        });
        on!(on_reset_calc, |ui, st, rc| {
            let defaults = st.current().defaults();
            let key = (st.cat, st.calc);
            st.values.insert(key, defaults);
            rebuild_fields(st);
            recompute(&ui, st);
            mark_changed(&ui, st, rc);
        });
        on!(on_keypad_key, |ui, st, rc, key: SharedString| {
            keypad_key(&ui, st, key.as_str());
        });
        on!(on_keypad_quick_add, |ui, st, rc, i: i32| {
            keypad_quick(&ui, st, i as usize);
        });
        on!(on_keypad_done, |ui, st, rc| {
            keypad_done(&ui, st, rc);
        });
        on!(on_keypad_cancel, |ui, st, rc| {
            st.keypad = None;
            ui.set_keypad_open(false);
        });

        Ok(App { ui, state })
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.ui.run()?;
        // save anything still pending when the window closes
        if self.ui.get_page() == 2 {
            commit(&self.ui, &mut self.state.borrow_mut());
        }
        Ok(())
    }

    // ---- helpers used by the desktop binary for screenshots/testing

    pub fn open(&self, cat: usize, calc: Option<usize>) {
        let mut st = self.state.borrow_mut();
        match calc {
            Some(k) => open_calc(&self.ui, &mut st, cat, k),
            None => open_category(&self.ui, &mut st, cat),
        }
    }

    /// Open the keypad for the `field`-th visible field.
    pub fn open_keypad(&self, field: usize) {
        let mut st = self.state.borrow_mut();
        if let Some(&i) = st.visible.get(field) {
            keypad_open(&self.ui, &mut st, i);
        }
    }

    pub fn set_dark(&self, dark: bool) {
        self.ui.global::<Theme>().set_dark(dark);
    }

    pub fn show_page(&self, page: i32) {
        refresh_history(&self.ui, &self.state.borrow());
        self.ui.set_page(page);
    }

    /// Simulate a slider change (used by tests/dev tools).
    pub fn set_field(&self, field: usize, value: f64) {
        self.ui.invoke_field_slider(field as i32, value as f32);
    }

    pub fn flush(&self) {
        commit(&self.ui, &mut self.state.borrow_mut());
    }

    pub fn field_count(&self) -> usize {
        self.state.borrow().visible.len()
    }
}

pub fn run() -> Result<(), slint::PlatformError> {
    App::new()?.run()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_main() {
    console_error_panic_hook::set_once();
    run().unwrap();
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    if let Some(dir) = app.internal_data_path() {
        storage::set_data_dir(dir);
    }
    slint::android::init(app).unwrap();
    run().unwrap();
}
