pub(super) mod campus_card;
pub(super) mod chaoxing;
pub(super) mod edu;
pub(super) mod mooc;
pub(super) mod physics_experiment;

use chrono::{DateTime, FixedOffset};
use csustkit::Campus;

use super::console::Console;

fn choose_campus(console: &mut Console) -> Option<Campus> {
    console
        .select("选择校区", &Campus::ALL, |campus| campus.display_name())
        .copied()
}

fn display_date(date: DateTime<FixedOffset>) -> impl std::fmt::Display {
    date.format("%Y-%m-%d %H:%M")
}
