use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContexts};

use crate::{
    ant::{
        birthing::Birthing, hunger::Hunger, sleep::Asleep, Ant, AntInventory, AntName, AntRole,
        Dead,
    },
    common::IdMap,
    element::Element,
    pheromone::{Pheromone, PheromoneStrength},
    world_map::position::Position,
};

#[derive(Component, Default, PartialEq, Copy, Clone, Debug)]
pub struct Selected;

pub fn update_selection_menu(
    mut contexts: EguiContexts,
    primary_window_query: Query<&Window, With<PrimaryWindow>>,

    selected_ant_query: Query<
        (
            &Hunger,
            &AntName,
            &AntRole,
            &AntInventory,
            Option<&Birthing>,
            Option<&Dead>,
            Option<&Asleep>,
        ),
        (With<Ant>, With<Selected>),
    >,
    selected_element_query: Query<(&Element, &Position), With<Selected>>,
    pheromone_query: Query<(&Position, &Pheromone, &PheromoneStrength)>,
    elements_query: Query<&Element>,
    id_map: Res<IdMap>,
) {
    let window = primary_window_query.single();
    let ctx = contexts.ctx_mut();

    egui::Window::new("Selection")
        .default_pos(egui::Pos2::new(0.0, window.height()))
        .resizable(false)
        .show(ctx, |ui| {
            if let Ok((element, element_position)) = selected_element_query.get_single() {
                ui.label("Element");
                ui.label(&format!("Type: {:?}", element));

                // TODO: It's weird to show Pheromone here when they're tied to Tile not Element
                if let Some((_, pheromone, pheromone_strength)) = pheromone_query
                    .iter()
                    .find(|(&pheromone_position, _, _)| pheromone_position == *element_position)
                {
                    ui.label(&format!("Pheromone Type: {:?}", pheromone));
                    ui.label(&format!(
                        "Pheromone Strength: {:.0}",
                        pheromone_strength.value()
                    ));
                }
            } else if let Ok((hunger, name, ant_role, inventory, birthing, dead, asleep)) =
                selected_ant_query.get_single()
            {
                ui.label("Ant");
                ui.label(&format!("Name: {}", name.0));
                ui.label(&format!("Role: {:?}", ant_role));
                ui.label(&format!("Hunger: {:.0}%", hunger.value()));

                if let Some(inventory_element_id) = &inventory.0 {
                    let entity = id_map.0.get(inventory_element_id).unwrap();
                    let element = elements_query.get(*entity).unwrap();

                    ui.label(&format!("Carrying: {:?}", element));
                }

                if let Some(birthing) = birthing {
                    ui.label(&format!("Birthing: {:.0}%", birthing.value()));
                }

                if let Some(_) = asleep {
                    ui.label(&format!("Sleeping"));
                }

                if let Some(_) = dead {
                    // TODO: Maybe have it say "Died at XXX"
                    ui.label("Dead");
                }
            }
        });
}