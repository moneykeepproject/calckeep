// Desktop entry point (Windows, macOS, Linux).
//
// Optional environment variables for development/screenshots:
//   CALCKEEP_SIZE=390x844     start with a phone-sized window
//   CALCKEEP_OPEN=0           open category 0;  CALCKEEP_OPEN=0,1  open calculator 1 of category 0
//   CALCKEEP_KEYPAD=0         open the keypad for field 0
//   CALCKEEP_THEME=light|dark force a theme
//   CALCKEEP_PAGE=3           open History (3) or About (4)
//   CALCKEEP_DATA=/tmp/x.json use a different data file
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() -> Result<(), slint::PlatformError> {
    let app = calckeep::App::new()?;

    if let Ok(size) = std::env::var("CALCKEEP_SIZE") {
        if let Some((w, h)) = size.split_once('x') {
            if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                use slint::ComponentHandle;
                app.ui.window().set_size(slint::LogicalSize::new(w, h));
            }
        }
    }
    if let Ok(t) = std::env::var("CALCKEEP_THEME") {
        app.set_dark(t != "light");
    }
    if let Ok(open) = std::env::var("CALCKEEP_OPEN") {
        let mut parts = open
            .split(',')
            .filter_map(|p| p.trim().parse::<usize>().ok());
        if let Some(cat) = parts.next() {
            app.open(cat, parts.next());
        }
    }
    if let Ok(p) = std::env::var("CALCKEEP_PAGE") {
        if let Ok(p @ (3 | 4)) = p.parse::<i32>() {
            app.show_page(p);
        }
    }
    if let Ok(k) = std::env::var("CALCKEEP_KEYPAD") {
        if let Ok(i) = k.parse::<usize>() {
            if i < app.field_count() {
                app.open_keypad(i);
            }
        }
    }
    app.run()
}
