use bevy::prelude::*;

use crate::{
    ant::{commands::AntCommandsExt, AntInventory},
    common::position::Position,
    element::Element,
    nest::Nest,
};

use super::Dead;

/// Force ants to drop, or despawn, their inventory upon death.
/// TODO:
///     * It might be preferable to find an adjacent, available location to move inventory to rather than despawning.
pub fn on_ants_add_dead(
    ants_query: Query<(Entity, &Position, &AntInventory), Added<Dead>>,
    mut commands: Commands,
    nest: Res<Nest>,
    elements_query: Query<&Element>,
) {
    for (ant_entity, ant_position, ant_inventory) in ants_query.iter() {
        if ant_inventory.0 != None {
            let element_entity = nest.get_element_entity(*ant_position).unwrap();

            if nest.is_element(&elements_query, *ant_position, Element::Air) {
                commands.drop(ant_entity, *ant_position, *element_entity);
            } else {
                commands.entity(*element_entity).remove_parent().despawn();
            }
        }
    }
}