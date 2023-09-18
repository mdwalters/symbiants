use bevy::prelude::*;
use bevy_save::{Rollbacks, SaveableRegistry};
use bevy_turborand::GlobalRng;

use crate::{
    ant::{
        act::ants_act,
        ants_initiative,
        birthing::ants_birthing,
        hunger::ants_hunger,
        initialize_ant,
        ui::{
            on_spawn_ant, on_update_ant_dead, on_update_ant_inventory, on_update_ant_orientation,
        },
        walk::ants_walk, deinitialize_ant,
    },
    background::setup_background,
    common::{initialize_common, ui::on_update_position, deinitialize_common},
    element::{initialize_element, ui::on_spawn_element, deinitialize_element},
    gravity::{gravity_ants, gravity_crush, gravity_elements, gravity_stability},
    grid::{
        cleanup_world_map, create_new_world_map, regenerate_cache,
        save::{
            cleanup_window_onunload_save_world_state, load_existing_world,
            periodic_save_world_state, setup_window_onunload_save_world_state,
        },
    },
    mouse::{handle_mouse_clicks, is_pointer_captured, IsPointerCaptured},
    nest::{initialize_nest, deinitialize_nest},
    settings::{initialize_settings, deinitialize_settings},
    story_state::{on_story_cleanup, setup_story_state, StoryState},
    time::{
        deinitialize_game_time, initialize_game_time, set_rate_of_time, setup_game_time,
        update_game_time, DEFAULT_SECONDS_PER_TICK,
    },
    ui::action_menu::on_interact_action_menu_button,
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveableRegistry>();
        // TODO: Not sure I need this?
        app.init_resource::<Rollbacks>();

        // TODO: Delegate this to setup/cleanup fns on various submodules

        // UI:
        app.init_resource::<IsPointerCaptured>();

        // TODO: I put very little thought into initializing this resource always vs saving/loading the seed.
        app.init_resource::<GlobalRng>();

        // Control the speed of the simulation by defining how many simulation ticks occur per second.
        //app.insert_resource(FixedTime::new_from_secs(1.0 / 60.0));
        app.insert_resource(FixedTime::new_from_secs(DEFAULT_SECONDS_PER_TICK));

        app.add_state::<StoryState>();

        app.add_systems(
            OnEnter(StoryState::Initializing),
            (
                initialize_settings,
                initialize_common,
                initialize_game_time,
                initialize_nest,
                initialize_element,
                initialize_ant,
                load_from_save,
            )
                .chain(),
        );

        app.add_systems(OnEnter(StoryState::Creating), create_new_world_map);

        app.add_systems(
            OnEnter(StoryState::FinalizingStartup),
            (
                regenerate_cache,
                setup_game_time,
                setup_background,
                setup_story_state,
                #[cfg(target_arch = "wasm32")]
                setup_window_onunload_save_world_state,
            )
                .chain(),
        );

        // NOTE: don't process user input events in FixedUpdate because events in FixedUpdate are broken (should be fixed in bevy 0.12)
        app.add_systems(
            Update,
            // TODO: coupling... need to handle clicking the simulation after menus so pointer capture works properly
            (
                is_pointer_captured,
                on_interact_action_menu_button,
                handle_mouse_clicks,
            )
                .run_if(in_state(StoryState::Telling))
                .chain(),
        );

        // TODO: revisit this idea - I want all simulation systems to be able to run in parallel.
        app.add_systems(
            FixedUpdate,
            (
                // It's helpful to apply gravity first because position updates are applied instantly and are seen by subsequent systems.
                // Thus, ant actions can take into consideration where an element is this frame rather than where it was last frame.
                gravity_elements,
                gravity_ants,
                // Gravity side-effects can run whenever with little difference.
                gravity_crush,
                gravity_stability,
                // Ants move before acting because positions update instantly, but actions use commands to mutate the world and are deferred + batched.
                // By applying movement first, commands do not need to anticipate ants having moved, but the opposite would not be true.
                ants_walk,
                // Apply specific ant actions in priority order because ants take a maximum of one action per tick.
                // An ant should not starve to hunger due to continually choosing to dig a tunnel, etc.
                ants_hunger,
                ants_birthing,
                ants_act,
                // Reset initiative only after all actions have occurred to ensure initiative properly throttles actions-per-tick.
                ants_initiative,
                // Bevy doesn't have support for PreUpdate/PostUpdate lifecycle from within FixedUpdate.
                // In an attempt to simulate this behavior, manually call `apply_deferred` because this would occur
                // when moving out of the Update stage and into the PostUpdate stage.
                // This is an important action which prevents panics while maintaining simpler code.
                // Without this, an Element might be spawned, and then despawned, with its initial render command still enqueued.
                // This would result in a panic due to missing Element entity unless the render command was rewritten manually
                // to safeguard against missing entity at time of command application.
                apply_deferred,
                // Provide an opportunity to write world state to disk.
                // This system does not run every time because saving is costly, but it does run periodically, rather than simply JIT,
                // to avoid losing too much state in the event of a crash.
                #[cfg(target_arch = "wasm32")]
                periodic_save_world_state,
                // Ensure render state reflects simulation state after having applied movements and command updates.
                on_update_position,
                on_update_ant_orientation,
                on_update_ant_dead,
                on_update_ant_inventory,
                on_spawn_ant,
                on_spawn_element,
                update_game_time,
                set_rate_of_time,
            )
                .run_if(in_state(StoryState::Telling))
                .chain(),
        );

        app.add_systems(
            OnEnter(StoryState::Cleanup),
            (
                deinitialize_ant,
                deinitialize_common,
                deinitialize_element,
                deinitialize_nest,
                deinitialize_game_time,
                deinitialize_settings,
                #[cfg(target_arch = "wasm32")]
                cleanup_window_onunload_save_world_state,
                cleanup_world_map,
                on_story_cleanup,
            )
                .chain(),
        );
    }
}

pub fn load_from_save(world: &mut World) {
    let is_loaded = load_existing_world(world);

    info!("is_loaded: {}", is_loaded);

    let mut story_state = world.resource_mut::<NextState<StoryState>>();
    if is_loaded {
        story_state.set(StoryState::FinalizingStartup);
    } else {
        story_state.set(StoryState::GatheringSettings);
    }
}
