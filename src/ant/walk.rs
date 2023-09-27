use crate::{
    element::Element,
    grid::{position::Position, WorldMap},
    settings::Settings,
};

use super::{birthing::Birthing, AntInventory, AntOrientation, AntRole, Dead, Initiative, nesting::Nesting};
use bevy::prelude::*;
use bevy_turborand::{DelegatedRng, GlobalRng};

// Update the position and orientation of all ants. Does not affect the external environment.
pub fn ants_walk(
    mut ants_query: Query<
        (
            &mut Initiative,
            &mut Position,
            &mut AntOrientation,
            &AntRole,
            &AntInventory,
            // TODO: Optional component is usually a bad sign of encapsulation - feels like walking can't be as separate as I want it to be?
            Option<&Nesting>
        ),
        (Without<Dead>, Without<Birthing>),
    >,
    elements_query: Query<&Element>,
    world_map: Res<WorldMap>,
    settings: Res<Settings>,
    mut rng: ResMut<GlobalRng>,
) {
    for (mut initiative, mut position, mut orientation, role, inventory, nesting) in ants_query.iter_mut() {
        // If ant lost the ability to move (potentially due to falling through the air) then skip walking around.
        if !initiative.can_move() {
            continue;
        }

        // An ant might not have stable footing even though it's not falling through the air. This can occur when an ant is
        // standing perpendicular to the ground, with ground to the south of them, and the block they're walking on is dug up.
        // An ant without stable footing will turn in an attempt to find stable footing.
        let under_feet_position = *position + orientation.rotate_forward().get_forward_delta();
        let has_air_under_feet =
            world_map.is_element(&elements_query, under_feet_position, Element::Air);

        // An ant might be attempting to walk forward into a solid block. If so, they'll turn and walk up the block.
        let forward_position = *position + orientation.get_forward_delta();
        let has_air_ahead = world_map
            .get_element(forward_position)
            .map_or(false, |entity| {
                elements_query
                    .get(*entity)
                    .map_or(false, |element| *element == Element::Air)
            });

        // An ant might turn randomly. This is to prevent ants from getting stuck in loops and add visual variety.
        let is_turning_randomly = rng.chance(settings.probabilities.random_turn.into());

        // Queen should head back to the nest when dropping sand off above surface. This is a hacky
        // stub for now. Pheromones would be better?
        let mut is_queen_turning_towards_nest = false;
        if *role == AntRole::Queen
            && world_map.is_aboveground(&position)
            && inventory.0 == None
            && orientation.is_horizontal()
            && nesting.is_some() && nesting.unwrap().position().is_some()
        {
            let nest_position = nesting.unwrap().position().unwrap();
            // distance from position to nest position:
            let distance_to_nest =
                (position.x - nest_position.x).abs() + (position.y - nest_position.y).abs();

            // distance from forward position to nest position:
            let distance_to_nest_forward = (forward_position.x - nest_position.x).abs()
                + (forward_position.y - nest_position.y).abs();

            if distance_to_nest_forward > distance_to_nest {
                is_queen_turning_towards_nest = true;
            }
        }

        if has_air_under_feet
            || !has_air_ahead
            || is_turning_randomly
            || is_queen_turning_towards_nest
        {
            *orientation = get_turned_orientation(
                &orientation,
                &position,
                &elements_query,
                &world_map,
                &mut rng,
            );

            initiative.consume_movement();
            continue;
        }

        // Definitely walking forward, but if that results in standing over air then turn on current block.
        let foot_orientation = orientation.rotate_forward();
        let foot_position = forward_position + foot_orientation.get_forward_delta();

        if let Some(foot_entity) = world_map.get_element(foot_position) {
            let foot_element = elements_query.get(*foot_entity).unwrap();

            if *foot_element == Element::Air {
                // If ant moves straight forward, it will be standing over air. Instead, turn into the air and remain standing on current block
                *position = foot_position;
                *orientation = foot_orientation;
            } else {
                // Just move forward
                *position = forward_position;
            }

            initiative.consume_movement();
        }
    }
}

fn get_turned_orientation(
    orientation: &AntOrientation,
    position: &Position,
    elements_query: &Query<&Element>,
    world_map: &Res<WorldMap>,
    rng: &mut ResMut<GlobalRng>,
) -> AntOrientation {
    // First try turning perpendicularly towards the ant's back. If that fails, try turning around.
    let back_orientation = orientation.rotate_backward();
    if is_valid_location(back_orientation, *position, elements_query, world_map) {
        return back_orientation;
    }

    let opposite_orientation = orientation.turn_around();
    if is_valid_location(opposite_orientation, *position, elements_query, world_map) {
        return opposite_orientation;
    }

    // Randomly turn in a valid different when unable to simply turn around.
    let all_orientations = AntOrientation::all_orientations();
    let valid_orientations = all_orientations
        .iter()
        .filter(|&&inner_orientation| inner_orientation != *orientation)
        .filter(|&&inner_orientation| {
            is_valid_location(inner_orientation, *position, elements_query, world_map)
        })
        .collect::<Vec<_>>();

    if !valid_orientations.is_empty() {
        return *valid_orientations[rng.usize(0..valid_orientations.len())];
    }

    // If no valid orientations, just pick a random orientation.
    all_orientations[rng.usize(0..all_orientations.len())]
}

fn is_valid_location(
    orientation: AntOrientation,
    position: Position,
    elements_query: &Query<&Element>,
    world_map: &Res<WorldMap>,
) -> bool {
    // Need air at the ants' body for it to be a legal ant location.
    let Some(entity) = world_map.get_element(position) else {
        return false;
    };
    let Ok(element) = elements_query.get(*entity) else {
        panic!("is_valid_location - expected entity to exist")
    };

    if *element != Element::Air {
        return false;
    }

    // Get the location beneath the ants' feet and check for air
    let foot_position = position + orientation.rotate_forward().get_forward_delta();
    let Some(entity) = world_map.get_element(foot_position) else {
        return false;
    };
    let Ok(element) = elements_query.get(*entity) else {
        panic!("is_valid_location - expected entity to exist")
    };

    if *element == Element::Air {
        return false;
    }

    true
}
