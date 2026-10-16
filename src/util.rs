// ------------------------------------------------------------
// Module: util
// Purpose: Small display helpers shared by the queue and the UI.
// ------------------------------------------------------------

use std::time::Duration;

use chrono::Datelike;

// ------------------------------------------------------------
// Function: format_time
// Purpose: Renders a duration as m:ss, or h:mm:ss past one hour.
// ------------------------------------------------------------
pub fn format_time(duration: Duration) -> String {
    let total = duration.as_secs();
    let seconds = total % 60;
    let minutes = (total / 60) % 60;
    let hours = total / 3600;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

// ------------------------------------------------------------
// Function: format_date
// Purpose: Renders a unix timestamp as "14 Oct 2026".
// ------------------------------------------------------------
pub fn format_date(secs: i64) -> String {
    calendar(secs)
        .map(|date| render_date(date, true))
        .unwrap_or_default()
}

// ------------------------------------------------------------
// Function: format_date_span
// Purpose: Renders the oldest and newest dates. The year is
//          written once when both dates share it: "25 Aug – 14 Oct 2026".
// ------------------------------------------------------------
pub fn format_date_span(oldest: i64, newest: i64) -> String {
    let Some(oldest_date) = calendar(oldest) else {
        return format_date(newest);
    };
    let Some(newest_date) = calendar(newest) else {
        return format_date(oldest);
    };
    if oldest_date == newest_date {
        return render_date(newest_date, true);
    }
    if oldest_date.year() == newest_date.year() {
        format!(
            "{} – {}",
            render_date(oldest_date, false),
            render_date(newest_date, true)
        )
    } else {
        format!(
            "{} – {}",
            render_date(oldest_date, true),
            render_date(newest_date, true)
        )
    }
}

// ------------------------------------------------------------
// Function: skip_target
// Purpose: Where a rewind or forward lands. Never before 0.
//          The end is a limit only when the length is known.
// ------------------------------------------------------------
pub fn skip_target(position: f32, delta: f32, total: f32) -> f32 {
    let next = (position + delta).max(0.0);
    if total > 0.0 { next.min(total) } else { next }
}

fn calendar(secs: i64) -> Option<chrono::NaiveDate> {
    chrono::DateTime::from_timestamp(secs, 0).map(|time| time.date_naive())
}

fn render_date(date: chrono::NaiveDate, with_year: bool) -> String {
    let day = date.format("%d").to_string();
    let day = day.trim_start_matches('0');
    if with_year {
        format!("{day} {} {}", date.format("%b"), date.format("%Y"))
    } else {
        format!("{day} {}", date.format("%b"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_minutes_and_hours() {
        assert_eq!(format_time(Duration::from_secs(0)), "0:00");
        assert_eq!(format_time(Duration::from_secs(65)), "1:05");
        assert_eq!(format_time(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn skip_target_stops_at_zero_and_only_at_a_known_end() {
        assert_eq!(skip_target(10.0, -30.0, 100.0), 0.0);
        assert_eq!(skip_target(90.0, 30.0, 100.0), 100.0);
        assert_eq!(skip_target(40.0, 30.0, 0.0), 70.0);
        assert_eq!(skip_target(5.0, -15.0, 0.0), 0.0);
    }

    #[test]
    fn formats_a_publish_date_and_span() {
        assert_eq!(format_date(1_791_936_000), "14 Oct 2026");
        assert_eq!(
            format_date_span(1_787_616_000, 1_791_936_000),
            "25 Aug – 14 Oct 2026"
        );
    }
}
