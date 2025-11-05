use chrono::{DateTime, Utc};

pub fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 60 {
        return "just now".to_string();
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        return format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" });
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
    } else if duration.num_days() < 7 {
        let days = duration.num_days();
        if days == 0 {
            // Less than 24 hours but in the same day
            return dt.format("%A %I%p").to_string().to_lowercase();
        }
        return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
    } else if duration.num_days() < 60 {
        // Show weeks for anything less than 2 months
        let weeks = duration.num_weeks();
        return format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" });
    } else if duration.num_days() < 365 {
        // Show months only when >= 2 months
        let months = duration.num_days() / 30;
        return format!("{} month{} ago", months, if months == 1 { "" } else { "s" });
    } else {
        let years = duration.num_days() / 365;
        return format!("{} year{} ago", years, if years == 1 { "" } else { "s" });
    }
}
