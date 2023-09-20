use bevy::{prelude::*, reflect::Reflect};
use bevy_turborand::{DelegatedRng, GlobalRng};

use crate::{common::register, grid::position::Position};

#[derive(Clone, Copy, Reflect, Debug)]
pub struct Probabilities {
    pub random_dig: f32,              // dig down while wandering
    pub random_drop: f32,             // drop while wandering
    pub random_turn: f32,             // turn while wandering
    pub random_fall: f32,             // fall while upside down
    pub random_slip: f32,             // fall while vertical
    pub below_surface_dirt_dig: f32,  // chance to dig dirt when below surface level
    pub above_surface_sand_drop: f32, // chance to randomly drop sand when at-or-above surface level
    pub below_surface_food_drop: f32, // chance to randomly drop food when below surface level

    pub above_surface_queen_nest_dig: f32,
    pub below_surface_queen_nest_dig: f32,
}

#[derive(Resource, Copy, Clone, Reflect, Debug)]
#[reflect(Resource)]
pub struct Settings {
    pub snapshot_interval: isize,
    pub save_interval: isize,
    pub world_width: isize,
    pub world_height: isize,
    // sand turns to dirt when stacked this high
    pub compact_sand_depth: isize,
    pub initial_dirt_percent: f32,
    pub initial_ant_worker_count: isize,
    pub ant_color: Color,
    pub probabilities: Probabilities,
}

// TODO: It feels weird to put these methods here rather than on WorldMap, but I need access to these
// calculations when creating a new WorldMap instance.
impl Settings {
    pub fn get_surface_level(&self) -> isize {
        (self.world_height as f32 - (self.world_height as f32 * self.initial_dirt_percent)) as isize
    }

    pub fn get_random_surface_position(&self, rng: &mut Mut<GlobalRng>) -> Position {
        Position::new(rng.isize(0..self.world_width), self.get_surface_level())
    }
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            // Save the world automatically because it's possible the browser could crash so saving on window unload isn't 100% reliable.
            save_interval: 60,
            // Saving data to local storage is slow, but generating the snapshot of the world is also slow.
            // Take snapshots aggressively because browser tab closes too quickly to JIT snapshot.
            snapshot_interval: 5, // TODO: prefer 1 here but it's too slow (in debug at least), kills FPS
            world_width: 144,
            world_height: 81,
            compact_sand_depth: 15,
            initial_dirt_percent: 3.0 / 4.0,
            initial_ant_worker_count: 0,
            ant_color: Color::rgb(0.584, 0.216, 0.859), // purple!
            probabilities: Probabilities {
                random_dig: 0.003,
                random_drop: 0.003,
                random_turn: 0.005,
                // Ants that are upside down have a high likelihood of falling to gravity
                // Ants that are vertical have a low likelihood of falling to gravity and will probably catch themselves when they fall
                // These settings help prevent scenarios where ants dig themselves onto islands and become trapped.
                random_fall: 0.005,
                random_slip: 0.001,
                below_surface_dirt_dig: 0.05,
                above_surface_sand_drop: 0.05,
                below_surface_food_drop: 0.20,

                above_surface_queen_nest_dig: 0.10,
                below_surface_queen_nest_dig: 0.50,
            },
        }
    }
}

pub fn initialize_settings(world: &mut World) {
    register::<Settings>(world);
    register::<Probabilities>(world);

    world.init_resource::<Settings>();
}

pub fn deinitialize_settings(world: &mut World) {
    world.remove_resource::<Settings>();
}
