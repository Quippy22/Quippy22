use chrono::{Datelike, Local, NaiveDate};

#[derive(Clone)]
pub struct Language {
    pub name: String,
    pub bytes: u64,
}

pub struct Stats {
    pub user: String,
    pub joined: String,
    pub repos: u64,
    pub stars: u64,
    pub commits: u64,
    pub loc: u64,
    pub languages: Vec<Language>,
}

impl Stats {
    /// Read `assets/stats.txt` (written by `scripts/refresh.py`).
    pub fn load(path: &str) -> Stats {
        let raw = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("{path} is missing — run scripts/refresh.py to fetch it"));

        let mut kv: Vec<(String, String)> = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                kv.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        let get = |k: &str| {
            kv.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let num = |k: &str| get(k).parse::<u64>().unwrap_or(0);

        let languages = get("languages")
            .split(',')
            .filter_map(|part| {
                let (name, bytes) = part.split_once(':')?;
                Some(Language {
                    name: name.trim().to_string(),
                    bytes: bytes.trim().parse().ok()?,
                })
            })
            .filter(|l| l.bytes > 0)
            .collect();

        Stats {
            user: get("user"),
            joined: get("joined"),
            repos: num("repos"),
            stars: num("stars"),
            commits: num("commits"),
            loc: num("loc"),
            languages,
        }
    }

    /// Time since the account was created, as neofetch-style uptime ("4y 4m").
    pub fn uptime(&self) -> String {
        let Some(joined) = NaiveDate::parse_from_str(&self.joined, "%Y-%m-%d").ok() else {
            return "?".into();
        };
        let (years, months, _) = elapsed(joined, Local::now().date_naive());
        match (years, months) {
            (0, m) => format!("{m}m"),
            (y, 0) => format!("{y}y"),
            (y, m) => format!("{y}y {m}m"),
        }
    }

    /// Languages by share of bytes, biggest first.
    pub fn language_shares(&self) -> Vec<(&Language, f64)> {
        let total: u64 = self.languages.iter().map(|l| l.bytes).sum();
        if total == 0 {
            return Vec::new();
        }
        let mut v: Vec<(&Language, f64)> = self
            .languages
            .iter()
            .map(|l| (l, l.bytes as f64 / total as f64))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v
    }
}

/// Years, months and days since a fixed date, e.g. "17y 8m 27d".
pub fn uptime_since(year: i32, month: u32, day: u32) -> String {
    let Some(from) = NaiveDate::from_ymd_opt(year, month, day) else {
        return "?".into();
    };
    let (years, months, days) = elapsed(from, Local::now().date_naive());
    format!("{years}y {months}m {days}d")
}

/// Calendar split of the span between two dates: full years, months and days.
fn elapsed(from: NaiveDate, to: NaiveDate) -> (i32, i32, i32) {
    let mut years = to.year() - from.year();
    let mut months = to.month() as i32 - from.month() as i32;
    let mut days = to.day() as i32 - from.day() as i32;

    if days < 0 {
        months -= 1;
        let (year, month) = if to.month() == 1 {
            (to.year() - 1, 12)
        } else {
            (to.year(), to.month() - 1)
        };
        days += days_in_month(year, month);
    }
    if months < 0 {
        months += 12;
        years -= 1;
    }
    (years.max(0), months.max(0), days.max(0))
}

fn days_in_month(year: i32, month: u32) -> i32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid month");
    (next - first).num_days() as i32
}

/// 32796 -> "32,796"
pub fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
