# CalcKeep

Financial calculators built with **Rust + Slint**, using one codebase for desktop, web (WASM), Android and iOS.

Version 0.1.2: [release notes](docs/release-notes/0.1.2.md) · [known limits](docs/known-limits.md).

**13 category tiles, 57 calculators**

| Category | Calculators |
|---|---|
| Mortgage | Mortgage Payment, Home Affordability, Refinance, Pay Off Early, Rent vs. Buy, Mortgage Points |
| Auto Loan | Auto Loan Payment, Car Affordability, Cash Rebate vs. Low APR, Auto Refinance |
| Auto Lease | Lease Payment, Lease vs. Buy, Lease Buyout vs. Return, Lease Mileage |
| Pay Off | Credit Card Payoff, Debt Snowball / Avalanche, Debt Consolidation (up to 6 debts), Minimum Payment Trap, Pay Off Debt vs. Invest |
| Loan | Loan Payment, Amortization Schedule, Loan Comparison (true APR), HELOC Payment, Student Loan Payoff |
| Retirement | Retirement Savings, 401(k) Growth, Social Security Claiming Age, How Long Will Savings Last, Roth vs. Traditional, Required Minimum Distribution |
| Savings & CD | CD Calculator, Savings Goal, Emergency Fund, CD Ladder |
| Investments | Compound Growth, Return on Investment, Inflation, Dividend Reinvestment, Bond Yield to Maturity |
| Tax | Income Tax Estimate, Tax-Equivalent Yield, Paycheck / Take-Home Pay, Self-Employment & Quarterly Tax, Capital Gains Tax |
| Insurance | Life Insurance Needs, Deductible Comparison, Health Plan Comparison (HSA), Disability Insurance Needs |
| Business | Break-Even, Profit Margin, True Cost of an Employee, Payback / NPV / IRR, Markup & Pricing |
| Real Estate | Rental Property Cash Flow (cap rate, cash-on-cash), Cap Rate & Property Value |
| Education | 529 College Savings, College Cost |

2026 figures used: federal brackets and standard deduction, long-term capital gains brackets (IRS Rev. Proc. 2025-32), 401(k) $24,500 / IRA $7,500, HSA $4,400 / $8,750, Social Security wage base $184,500, and the IRS Uniform Lifetime Table for RMDs.

## Input widgets (picked per field)

- **Keypad pop-up** for open-ended amounts (prices, balances, income). It's a bottom sheet modeled on the MoneyKeep pad, with quick-add chips sized to the field (+$1k/+$10k for a home price, +$10/+$50 for a payment). The first key you press replaces the old value, like a calculator. On desktop you can also type: digits, `.`, Backspace, Enter (done), Esc (cancel).
- **Slider** for bounded ranges (rates, terms, ages, percentages). It has a large thumb and snaps to a sensible step (0.125% for rates). Tap the value pill to type an exact number on the keypad.
- **Segmented chips** for small choices (loan term, filing status, compounding). They're 44 px tall and wrap on narrow screens.
- **Conditional fields** appear only when they apply. For example, "Number of debts" shows that many debt rows, and Itemized deductions only appears when Itemized is chosen.

The layout is responsive. Phones get one column, a sticky headline result at the bottom (tap it to jump to the full results) and 2-column tiles. Desktop gets inputs and results side by side. Light/dark theme follows the system; the sun/moon button toggles it.

## History & saved data

- **History** (clock icon on the home screen) keeps your **last 30 calculations**. When there are more, the oldest drop off automatically.
- An entry is recorded automatically when you change inputs on a calculator, about a second after you stop. Changes during one visit update the same entry, so dragging a slider doesn't flood the list. Each new visit creates a new entry.
- Tap an entry to reopen that calculator with those exact inputs. Tap ✕ to delete one entry, or *Clear all* (tap twice to confirm) to delete them all.
- Each calculator also remembers its **last-used inputs**, and the app remembers your **light/dark choice**.
- Everything stays on the device, as one small JSON document:

| Platform | Location |
|---|---|
| Windows | `%APPDATA%\CalcKeep\calckeep.json` |
| macOS | `~/Library/Application Support/CalcKeep/calckeep.json` |
| Linux | `~/.config/calckeep/calckeep.json` |
| Web | browser `localStorage`, key `calckeep` |
| Android | the app's internal data folder |
| iOS | the app's Documents folder |

History entries are matched to calculators by name, so renaming a calculator orphans its old entries; they're hidden. The **About** screen (ⓘ icon) shows the version, the Slint attribution required by Slint's royalty-free license, and the disclaimer.

## Run on desktop (Windows / macOS / Linux)

```bash
cargo run --release
```

Linux needs the usual windowing libraries (`libfontconfig-dev`, `libxkbcommon-dev`, plus a GL driver).

Dev helpers (environment variables):

```bash
CALCKEEP_SIZE=390x844 cargo run          # phone-sized window
CALCKEEP_OPEN=0,1 cargo run              # open category 0, calculator 1
CALCKEEP_KEYPAD=0 CALCKEEP_OPEN=0,0 cargo run   # open keypad on field 0
CALCKEEP_THEME=light cargo run
CALCKEEP_PAGE=3 cargo run                # open History (4 = About)
CALCKEEP_DATA=/tmp/test.json cargo run   # use a throwaway data file
```

## Windows installer (NSIS)

Same setup as MoneyKeep: the script is `installer/calckeep.nsi`, built by an xtask.

```powershell
winget install NSIS.NSIS        # once (or https://nsis.sourceforge.io/Download); then open a new terminal
cargo xtask installer           # -> dist\calckeep-<version>-windows-x64-setup.exe and dist\SHA256SUMS.txt
```

- `cargo xtask release` builds the release program and gathers `dist\CalcKeep-<version>\` (calckeep.exe, calckeep.ico, LICENSE-MIT, LICENSE-APACHE, NOTICE, README.md). `--dry-run` lists the steps only.
- `cargo xtask installer` runs `release`, then `makensis`, then writes `SHA256SUMS.txt` (`<sha256>  <name>`) beside the setup. `--skip-release` reuses an existing release folder.
- **Release files:** publish `calckeep-<version>-windows-x64-setup.exe` and `SHA256SUMS.txt` on the GitHub release. These are the names MoneyKeep Launcher looks for: it checks the setup against `SHA256SUMS.txt` and installs, updates and removes CalcKeep with it.
- The version comes from `Cargo.toml` (`[package] version`). Bump it there for each release.
- **The setup:** installs for all users in `C:\Program Files\CalcKeep`, so it asks for administrator permission. It has a Start menu shortcut (on) and a Desktop shortcut (off), registers in Installed apps, updates in place, and asks you to close CalcKeep if it's open. The uninstaller asks before deleting your history (`%APPDATA%\CalcKeep`).
- **Silent mode** (what MoneyKeep Launcher uses): `calckeep-<version>-windows-x64-setup.exe /S` installs or updates with the default options and no questions (`/D=<folder>` picks the folder of a new install). If CalcKeep is open it is asked to close, and after 30 s the setup stops with exit code 2; 0 means done, 3 any other failure. Windows still asks for administrator permission. The silent uninstall (`QuietUninstallString`: `"C:\Program Files\CalcKeep\uninstall.exe" /S`) never deletes your history.
- **Signing:** set `CK_SIGN_CERT` (path to a `.pfx`) and `CK_SIGN_PASSWORD` to sign calckeep.exe and the setup with `signtool`. `CK_SIGN_TIMESTAMP` overrides the timestamp server. Without them the setup is unsigned and SmartScreen will warn.
- `cargo test -p xtask` compiles the installer script with a placeholder program when NSIS is installed.

## Run in the browser (WASM)

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
./build-web.sh
python3 -m http.server 8080 --directory web
```

Then open http://localhost:8080. Deploy `web/` (index.html + pkg/) to any static host, such as Cloudflare Pages.
Slint draws into a `<canvas>`, so browser text selection and screen-reader support are limited compared with a regular web page.

## Android

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk
# needs ANDROID_HOME (SDK) and ANDROID_NDK_ROOT set
cargo apk run --target aarch64-linux-android --lib --release
```

The entry point (`android_main`) is already in `src/lib.rs`, and the package settings are in `Cargo.toml` under `[package.metadata.android]`.
See https://docs.slint.dev → Platforms → Android for signing and Play Store packaging.

## iOS

Slint supports iOS for Rust apps. It needs a Mac with Xcode:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
```

Then follow Slint's iOS guide (https://docs.slint.dev → Platforms → iOS) to wrap the library in an Xcode project. The app code itself (`calckeep::run()`) is unchanged.

## Adding a calculator

All calculators live in `src/engine/catalog/`, one file per category (`mortgage.rs`, `tax.rs`, …), and are registered in `catalog/mod.rs`. Each one is a list of fields plus a compute function:

```rust
fn my_calc() -> Calc {
    Calc {
        name: "Savings Goal",
        blurb: "How much to save each month to reach a goal.",
        fields: vec![
            Field::money("goal", "Savings goal", 20_000.0),                      // keypad
            Field::percent("rate", "Interest rate", 4.0, 0.0, 10.0, 0.05),       // slider
            Field::slider("years", "Years", Unit::Years, 3.0, 1.0, 30.0, 1.0),   // slider
            Field::choice("comp", "Compounding", &["Monthly", "Yearly"], 0),     // chips
        ],
        compute: |v| {
            let n = v.get("years") * 12.0;
            let r = v.pct("rate") / 12.0;
            let monthly = v.get("goal") * r / ((1.0 + r).powf(n) - 1.0);
            let mut o = Output::default();
            o.primary("Save each month", money(monthly));
            o
        },
    }
}
```

Then add `my_calc()` to a category's `calcs` in `categories()` in `catalog/mod.rs`. Useful extras: `.show_if("key", value)` / `.show_if_at_least("key", n)` to show a field conditionally, and `solve()`, `rate_for()`, `npv()`, `irr()` in `finance.rs` for rate solving. The UI picks it up automatically: no Slint changes are needed. To add a category, also add an SVG to `ui/icons/` and to the `Icons` list in `ui/app.slint`, plus a color in `Theme.cat-colors`.

## Project layout

```
src/engine/catalog/     calculators, one file per category (+ unit tests in mod.rs)
src/engine/finance.rs   payment / amortization / bracket math
src/engine/format.rs    $ / % / duration formatting
src/engine/mod.rs       Field, Calc, Output types
src/lib.rs              UI wiring, keypad, history, WASM + Android entry points
src/storage.rs          on-device storage (history cap of 30, last inputs, theme)
src/main.rs             desktop entry point
ui/*.slint              UI (theme, widgets, keypad sheet, main window)
web/index.html          WASM host page
```

`cargo test` runs the math checks: mortgage $320k at 6.5%/30y = $2,022.62, 2026 tax brackets, lease money factor, CD APY, and others.

## Licence

CalcKeep is licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

The user interface is built with [Slint](https://slint.dev), used under the Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0 (requires attribution). The About screen shows Slint's official `AboutSlint` widget to meet that requirement (licence §2(a)). See https://slint.dev/pricing.

Every third-party crate compiled into CalcKeep is listed with its licence in `THIRD-PARTY-LICENSES.txt`, which `cargo xtask release` writes and the installer installs (About → Third-party licences opens it).

*Calculators give estimates for planning only, not financial, tax or legal advice.*
