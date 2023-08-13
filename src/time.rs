use bevy::prelude::*;
use chrono::{Utc, TimeZone, LocalResult};
use gloo_storage::{LocalStorage, Storage};
use std::time::Duration;

use crate::map::{LOCAL_STORAGE_KEY, LastSaveTime};

pub const DEFAULT_TICK_RATE: f32 = 10.0 / 60.0;
pub const FAST_FORWARD_TICK_RATE: f32 = 0.001 / 60.0;
pub const SECONDS_PER_HOUR: i64 = 3600;
pub const SECONDS_PER_DAY: i64 = 86_400;

#[derive(Resource, Default)]
pub struct IsFastForwarding(pub bool);

#[derive(Resource, Default)]
pub struct PendingTicks(pub isize);

impl PendingTicks {
    pub fn as_minutes(&self) -> f32 {
        (self.0 as f32) / (60.0 * DEFAULT_TICK_RATE) / 60.0
    }
}

// When the app first loads - determine how long user has been away and fast-forward time accordingly.
pub fn setup_fast_forward_time(last_save_time: Res<LastSaveTime>, mut fixed_time: ResMut<FixedTime>) {
    if last_save_time.0 == 0 {
        return;
    }

    // Recreate UTC from last_save_time timestamp
    let last_save_time_utc =  match Utc.timestamp_opt(last_save_time.0, 0) {
        LocalResult::Single(datetime) => datetime,
        LocalResult::Ambiguous(a, b) => {
            // Handle the case where two possible DateTime<Utc> values exist.
            panic!("Ambiguous DateTime<Utc> values: {} and {}", a, b);
        }
        LocalResult::None => {
            // Handle the case where the timestamp is out of range and cannot be represented as a DateTime<Utc>.
            panic!("Invalid timestamp");
        }
    };

    let delta_seconds = Utc::now()
        .signed_duration_since(last_save_time_utc)
        .num_seconds();

    // Only one day of time is allowed to be fast-forwarded. If the user skips more time than this - they lose out on simulation.
    let elapsed_seconds = std::cmp::min(delta_seconds, SECONDS_PER_DAY);

    fixed_time.tick(Duration::from_secs(elapsed_seconds as u64));
}

// Determine how many simulation ticks should occur based off of default tick rate.
// Determine the conversation rate between simulation ticks and fast forward time.
// Update tick rate to fast forward time.
// When sufficient number of ticks have occurred, revert back to default tick rate.
pub fn play_time(
    mut fixed_time: ResMut<FixedTime>,
    mut is_fast_forwarding: ResMut<IsFastForwarding>,
    mut pending_ticks: ResMut<PendingTicks>,
) {
    if pending_ticks.0 > 0 {
        pending_ticks.0 -= 1;
    } else if is_fast_forwarding.0 {
        fixed_time.period = Duration::from_secs_f32(DEFAULT_TICK_RATE);
        is_fast_forwarding.0 = false;
    } else {
        // If fixed_time.accumulated() is large (more than a few ms) then it's assumed that either the app was closed
        // or the tab the app runs on was hidden. In either case, need to fast-forward time when app is active again.
        let accumulated_time = fixed_time.accumulated();

        if accumulated_time.as_secs() > 1 {
            // Reset fixed_time to zero and run the main Update schedule. This prevents the UI from becoming unresponsive for large time values.
            // The UI becomes unresponsive because the FixedUpdate schedule, when behind, will run in a loop without yielding until it catches up.
            fixed_time.period = accumulated_time;
            let _ = fixed_time.expend();

            fixed_time.period = Duration::from_secs_f32(FAST_FORWARD_TICK_RATE);
            is_fast_forwarding.0 = true;
            pending_ticks.0 =
                ((60.0 * DEFAULT_TICK_RATE) * accumulated_time.as_secs() as f32) as isize;
        }
    }
}
