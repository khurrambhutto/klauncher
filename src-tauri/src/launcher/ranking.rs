use crate::platform::linux::desktop_entries::DesktopApp;
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

#[derive(Clone, Debug)]
pub struct RankedApp {
    pub app: DesktopApp,
    pub score: i64,
}

pub fn rank_apps(apps: &[DesktopApp], query: &str) -> Vec<RankedApp> {
    let query = query.trim();
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::new(
        query,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );

    let mut ranked: Vec<RankedApp> = apps
        .iter()
        .filter_map(|app| {
            let score = if query.is_empty() {
                base_score(app)
            } else {
                app_score(app, &pattern, &mut matcher)?
            };

            Some(RankedApp {
                app: app.clone(),
                score,
            })
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.app.name.cmp(&right.app.name))
    });

    ranked
}

fn app_score(app: &DesktopApp, pattern: &Pattern, matcher: &mut Matcher) -> Option<i64> {
    let mut best = fuzzy_score(pattern, matcher, &app.name)?;

    if let Some(generic_name) = &app.generic_name {
        if let Some(score) = fuzzy_score(pattern, matcher, generic_name) {
            best = best.max(score - 10);
        }
    }

    if let Some(comment) = &app.comment {
        if let Some(score) = fuzzy_score(pattern, matcher, comment) {
            best = best.max(score - 30);
        }
    }

    for keyword in &app.keywords {
        if let Some(score) = fuzzy_score(pattern, matcher, keyword) {
            best = best.max(score + 40);
        }
    }

    if let Some(score) = fuzzy_score(pattern, matcher, &app.id) {
        best = best.max(score - 20);
    }

    Some(best + base_score(app))
}

fn base_score(app: &DesktopApp) -> i64 {
    let mut score = 0;

    if app.no_display {
        score -= 200;
    }

    if !app.terminal {
        score += 15;
    }

    score
}

fn fuzzy_score(pattern: &Pattern, matcher: &mut Matcher, candidate: &str) -> Option<i64> {
    let mut utf32_buffer = Vec::new();
    let candidate = Utf32Str::new(candidate, &mut utf32_buffer);

    pattern
        .score(candidate, matcher)
        .map(|score| i64::from(score))
}
