//! Calculator engine: data-driven calculator specs, values and formatting.
//!
//! A calculator is a list of [`Field`]s plus a `compute` function that turns the
//! current values into an [`Output`]. The UI renders any calculator generically,
//! so adding a new calculator only means adding a [`Calc`] to `catalog.rs`.

// Calculators live in src/engine/catalog/ (one file per category).
#[path = "catalog/mod.rs"]
pub mod catalog;
pub mod finance;
pub mod format;

/// How a field's value should be shown and entered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    Money,
    Percent,
    Years,
    Months,
    Count,
    Plain,
}

/// Which input widget the UI should use.
#[derive(Clone, Debug)]
pub enum Input {
    /// Pop-up calculator keypad (unbounded or large ranges: prices, balances…).
    Keypad,
    /// Slider for limited ranges (rates, terms, percentages). The value pill is
    /// also tappable to type an exact value on the keypad.
    Slider { step: f64 },
    /// Segmented choice; the value is the selected index.
    Choice(Vec<&'static str>),
}

/// Condition for showing a field based on another field's value.
#[derive(Clone, Copy, Debug)]
pub enum Show {
    /// Show when the other field's value is at least this (e.g. "number of debts" >= 3)
    AtLeast(f64),
    /// Show when the other field equals this (e.g. a choice index)
    Equals(f64),
}

#[derive(Clone, Debug)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub unit: Unit,
    pub input: Input,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    /// Quick-add amounts shown on the keypad (e.g. +$1k, +$10k).
    pub quick: Vec<f64>,
    /// Only show this field when another field meets a condition.
    pub show: Option<(&'static str, Show)>,
}

impl Field {
    fn base(
        key: &'static str,
        label: &'static str,
        unit: Unit,
        input: Input,
        default: f64,
    ) -> Self {
        Field {
            key,
            label,
            help: "",
            unit,
            input,
            default,
            min: 0.0,
            max: 1e12,
            quick: vec![],
            show: None,
        }
    }

    /// Dollar amount entered on the keypad.
    pub fn money(key: &'static str, label: &'static str, default: f64) -> Self {
        let quick = if default >= 100_000.0 {
            vec![1_000.0, 10_000.0, 50_000.0, 100_000.0]
        } else if default >= 10_000.0 {
            vec![500.0, 1_000.0, 5_000.0, 10_000.0]
        } else if default >= 1_000.0 {
            vec![100.0, 500.0, 1_000.0, 5_000.0]
        } else {
            vec![10.0, 50.0, 100.0, 500.0]
        };
        Self::base(key, label, Unit::Money, Input::Keypad, default).quick(&quick)
    }

    /// Plain number entered on the keypad.
    pub fn number(key: &'static str, label: &'static str, unit: Unit, default: f64) -> Self {
        Self::base(key, label, unit, Input::Keypad, default).quick(&[1.0, 5.0, 10.0])
    }

    /// Bounded value entered with a slider.
    pub fn slider(
        key: &'static str,
        label: &'static str,
        unit: Unit,
        default: f64,
        min: f64,
        max: f64,
        step: f64,
    ) -> Self {
        let mut f = Self::base(key, label, unit, Input::Slider { step }, default);
        f.min = min;
        f.max = max;
        f.quick = vec![step, step * 2.0, step * 4.0, step * 8.0];
        f
    }

    pub fn percent(
        key: &'static str,
        label: &'static str,
        default: f64,
        min: f64,
        max: f64,
        step: f64,
    ) -> Self {
        Self::slider(key, label, Unit::Percent, default, min, max, step)
    }

    pub fn choice(
        key: &'static str,
        label: &'static str,
        options: &[&'static str],
        default_index: usize,
    ) -> Self {
        let mut f = Self::base(
            key,
            label,
            Unit::Plain,
            Input::Choice(options.to_vec()),
            default_index as f64,
        );
        f.max = (options.len() - 1) as f64;
        f
    }

    pub fn help(mut self, help: &'static str) -> Self {
        self.help = help;
        self
    }

    pub fn quick(mut self, q: &[f64]) -> Self {
        self.quick = q.to_vec();
        self
    }

    /// Show only when field `key` >= `n` (for "how many rows" selectors).
    pub fn show_if_at_least(mut self, key: &'static str, n: f64) -> Self {
        self.show = Some((key, Show::AtLeast(n)));
        self
    }

    /// Show only when field `key` == `v` (e.g. a choice index).
    pub fn show_if(mut self, key: &'static str, v: f64) -> Self {
        self.show = Some((key, Show::Equals(v)));
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Clamp and (for sliders) snap a value to this field's rules.
    pub fn sanitize(&self, v: f64) -> f64 {
        let v = if v.is_finite() { v } else { self.default };
        let v = v.clamp(self.min, self.max);
        match &self.input {
            Input::Choice(_) => v.round(),
            _ => v,
        }
    }

    /// Number of decimals allowed when typing on the keypad.
    pub fn decimals(&self) -> usize {
        match self.unit {
            Unit::Money => 2,
            Unit::Percent => 3,
            Unit::Years => 1,
            Unit::Months | Unit::Count => 0,
            Unit::Plain => 4,
        }
    }
}

/// Kind of a result row; drives how the UI styles it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowKind {
    Primary,
    Normal,
    Note,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub label: String,
    pub value: String,
    pub kind: RowKind,
}

#[derive(Clone, Debug, Default)]
pub struct Table {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct Output {
    pub rows: Vec<Row>,
    pub table: Option<Table>,
}

impl Output {
    pub fn primary(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.push(RowKind::Primary, label, value)
    }
    pub fn row(&mut self, label: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.push(RowKind::Normal, label, value)
    }
    pub fn note(&mut self, text: impl Into<String>) -> &mut Self {
        self.push(RowKind::Note, text, "")
    }
    pub fn warn(&mut self, text: impl Into<String>) -> &mut Self {
        self.push(RowKind::Warning, text, "")
    }
    fn push(
        &mut self,
        kind: RowKind,
        label: impl Into<String>,
        value: impl Into<String>,
    ) -> &mut Self {
        self.rows.push(Row {
            label: label.into(),
            value: value.into(),
            kind,
        });
        self
    }
    pub fn warning_only(text: impl Into<String>) -> Output {
        let mut o = Output::default();
        o.warn(text);
        o
    }
}

/// Read-only access to the current values of a calculator, by field key.
pub struct Values<'a> {
    pub fields: &'a [Field],
    pub vals: &'a [f64],
}

impl Values<'_> {
    pub fn get(&self, key: &str) -> f64 {
        match self.fields.iter().position(|f| f.key == key) {
            Some(i) => self.vals.get(i).copied().unwrap_or(self.fields[i].default),
            None => panic!("unknown field key {key}"),
        }
    }
    /// Percent field as a fraction (6.5 -> 0.065).
    pub fn pct(&self, key: &str) -> f64 {
        self.get(key) / 100.0
    }
    /// Selected index of a choice field.
    pub fn idx(&self, key: &str) -> usize {
        self.get(key).max(0.0).round() as usize
    }
}

pub struct Calc {
    pub name: &'static str,
    pub blurb: &'static str,
    pub fields: Vec<Field>,
    pub compute: fn(&Values) -> Output,
}

impl Calc {
    pub fn defaults(&self) -> Vec<f64> {
        self.fields.iter().map(|f| f.default).collect()
    }
    /// Indexes of fields currently shown, given the values.
    pub fn visible(&self, vals: &[f64]) -> Vec<usize> {
        (0..self.fields.len())
            .filter(|&i| match self.fields[i].show {
                None => true,
                Some((key, cond)) => {
                    let v = self
                        .fields
                        .iter()
                        .position(|f| f.key == key)
                        .map(|j| vals[j])
                        .unwrap_or(0.0);
                    match cond {
                        Show::AtLeast(n) => v >= n - 1e-9,
                        Show::Equals(x) => (v - x).abs() < 1e-9,
                    }
                }
            })
            .collect()
    }

    pub fn run(&self, vals: &[f64]) -> Output {
        (self.compute)(&Values {
            fields: &self.fields,
            vals,
        })
    }
}

pub struct Category {
    pub name: &'static str,
    pub subtitle: &'static str,
    /// Index into the icon list in `ui/icons.slint`.
    pub icon: i32,
    pub calcs: Vec<Calc>,
}
