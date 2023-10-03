use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ant::{commands::AntCommandsExt, AntInventory, AntOrientation, Initiative},
    element::Element,
    pheromone::{Pheromone, PheromoneMap},
    settings::Settings,
    world_map::{position::Position, WorldMap},
};

#[derive(Component, Debug, PartialEq, Copy, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Component)]
pub struct Chambering(pub isize);

/// If covered in Chamber pheromone then the following things need to occur:
///  1) Look forward in the direction the ant is facing - if something is diggable - dig it.
///  2) Look up in the direction in the ant is facing - if something is diggable - dig it.
///  3) Either step forward or turn around
///  4) Repeat while covered in pheromone
pub fn ants_chamber_pheromone_act(
    mut ants_query: Query<
        (
            &AntOrientation,
            &AntInventory,
            &mut Initiative,
            &Position,
            Entity,
        ),
        With<Chambering>,
    >,
    elements_query: Query<&Element>,
    world_map: Res<WorldMap>,
    mut commands: Commands,
) {
    for (ant_orientation, inventory, mut initiative, ant_position, ant_entity) in
        ants_query.iter_mut()
    {
        if !initiative.can_act() {
            continue;
        }

        // Safeguard, but not expected to run because shouldn't have Chambering pheromone with full inventory.
        if inventory.0 != None {
            continue;
        }

        // Don't dig chambers northward because it can break through the surface.
        if !ant_orientation.is_facing_north()
            && try_dig(
                &ant_entity,
                &ant_orientation.get_ahead_position(ant_position),
                &elements_query,
                &world_map,
                &mut commands,
            )
        {
            initiative.consume_action();
            continue;
        }

        // Don't dig chambers northward because it can break through the surface.
        if !ant_orientation.is_rightside_up()
            && try_dig(
                &ant_entity,
                &ant_orientation.get_above_position(ant_position),
                &elements_query,
                &world_map,
                &mut commands,
            )
        {
            initiative.consume_action();
            continue;
        }
    }
}

/// Apply chambering to ants which walk over tiles covered in chamber pheromone.
/// Chambering is set to Chambering(3). This encourages ants to dig for the next 3 steps.
pub fn ants_add_chamber_pheromone(
    ants_query: Query<(Entity, &Position, &AntInventory), Changed<Position>>,
    pheromone_query: Query<&Pheromone>,
    pheromone_map: Res<PheromoneMap>,
    mut commands: Commands,
    settings: Res<Settings>,
) {
    for (ant_entity, ant_position, inventory) in ants_query.iter() {
        if inventory.0 != None {
            continue;
        }

        if let Some(pheromone_entity) = pheromone_map.0.get(ant_position) {
            let pheromone = pheromone_query.get(*pheromone_entity).unwrap();

            if *pheromone == Pheromone::Chamber {
                commands
                    .entity(ant_entity)
                    .insert(Chambering(settings.chamber_size));
            }
        }
    }
}

/// Whenever an ant takes a step it loses 1 Chambering pheromone.
pub fn ants_fade_chamber_pheromone(mut ants_query: Query<&mut Chambering, Changed<Position>>) {
    for mut chambering in ants_query.iter_mut() {
        chambering.0 -= 1;
    }
}

/// Ants lose Chambering when they begin carrying anything because they've fulfilled the pheromones action.
/// Ants lose Chambering when they emerge on the surface because chambers aren't dug aboveground.
/// Ants lose Chambering when they've exhausted their pheromone by taking sufficient steps.
pub fn ants_remove_chamber_pheromone(
    mut ants_query: Query<
        (Entity, &Position, &AntInventory, &Chambering),
        Or<(Changed<Position>, Changed<AntInventory>)>,
    >,
    mut commands: Commands,
    world_map: Res<WorldMap>,
) {
    for (entity, position, inventory, chambering) in ants_query.iter_mut() {
        if inventory.0 != None {
            commands.entity(entity).remove::<Chambering>();
        } else if world_map.is_aboveground(position) {
            commands.entity(entity).remove::<Chambering>();
        } else if chambering.0 <= 0 {
            commands.entity(entity).remove::<Chambering>();
        }
    }
}

// TODO: better home for this? maybe in commands?
fn try_dig(
    ant_entity: &Entity,
    dig_position: &Position,
    elements_query: &Query<&Element>,
    world_map: &WorldMap,
    commands: &mut Commands,
) -> bool {
    if !world_map.is_within_bounds(&dig_position) {
        return false;
    }

    // Check if hitting a solid element and, if so, consider digging through it.
    let element_entity = world_map.get_element(*dig_position).unwrap();
    let Ok(element) = elements_query.get(*element_entity) else {
        panic!("act - expected entity to exist")
    };

    if *element == Element::Air {
        return false;
    }

    commands.dig(*ant_entity, *dig_position, *element_entity);

    true
}
