# Known limits

What CalcKeep 0.1.2 doesn't do yet, or doesn't do on purpose. This list is updated
with each release. For what it does do, see the [release notes](release-notes/0.1.2.md).

## The answers

- **Results are estimates for planning only.** They are not financial, tax, legal or
  insurance advice. Check important decisions with a qualified professional.
- **Tax figures are for 2026** and use simplified assumptions. For other years the
  results may be off.
- **The Income Tax Estimate** ignores the alternative minimum tax, credit phase-outs
  and special deductions.
- **State tax** is a single rate you enter, not your state's own rules.
- **The Deductible Comparison** assumes every claim is larger than the deductible.

## Your data

- **History keeps the last 30 calculations;** older ones drop off by themselves.
- **Everything stays on this device,** in one small file. Nothing is synced between
  devices.
- **If a calculator is renamed** in a later version, its old history entries are
  hidden.

## Installing

- **The Windows setup is not signed yet.** Windows SmartScreen may warn: choose
  **More info**, then **Run anyway**.
- **The Mac build is not signed with an Apple Developer ID yet** (it is signed "ad
  hoc"). macOS warns the first time you open it: right-click CalcKeep, then choose
  **Open** (or System Settings > Privacy & Security > **Open Anyway**).
- **MoneyKeep Launcher can't install CalcKeep on a Mac yet.** It installs only Mac
  builds signed with the project's Developer ID. Until then, install from the
  release's `.dmg`; the launcher opens the copy you installed and links to new
  releases.
- **Release files are for Windows (64-bit) and Mac only.** CalcKeep can also be built
  for Linux, the web browser, Android and iPhone, but there are no downloads for those
  yet: build them from the source.
- **In a web browser,** selecting text and screen-reader support are limited compared
  with a regular web page.
