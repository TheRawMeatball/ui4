use bevy::ecs::prelude::*;
use bevy::math::*;

mod layout;

#[derive(Component)]
struct FinalSize(Vec2);

#[derive(Component, Default)]
pub(crate) struct Control {
    pub last_managed: Option<Entity>,
}

#[derive(Component, Copy, Clone)]
pub struct Parent(pub Entity);
#[derive(Component, Copy, Clone)]
pub struct FirstChild(pub Entity);
#[derive(Component, Copy, Clone)]
pub struct NextSibling(pub Entity);

pub(crate) fn despawn_control_node(cn: Entity, world: &mut World) {
    let next_unmanaged = world
        .get::<Control>(cn)
        .unwrap()
        .last_managed
        .and_then(|e| world.get::<NextSibling>(e).map(|c| c.0));

    if let Some(&parent) = world.get::<Parent>(cn) {
        let mut prev_child = None;
        let mut child = world.get::<FirstChild>(parent.0).map(|x| x.0).unwrap();
        loop {
            if child == cn {
                match (prev_child, next_unmanaged) {
                    (None, None) => {
                        world.entity_mut(parent.0).remove::<FirstChild>();
                    }
                    (None, Some(unmanaged)) => {
                        world.get_mut::<FirstChild>(parent.0).unwrap().0 = unmanaged;
                    }
                    (Some(prev), None) => {
                        world.entity_mut(prev).remove::<NextSibling>();
                    }
                    (Some(prev), Some(unmanaged)) => {
                        world.get_mut::<NextSibling>(parent.0).unwrap().0 = unmanaged;
                    }
                }

                break;
            }
            prev_child = Some(child);
            child = world.get::<NextSibling>(child).unwrap().0;
        }
    }

    let mut next_sibling = world.get::<NextSibling>(cn);

    while let Some(&sibling) = next_sibling {
        if Some(sibling.0) == next_unmanaged {
            break;
        }

        next_sibling = world.get::<NextSibling>(sibling.0);
        despawn_recursive(sibling.0, world);
    }
}

pub(crate) fn despawn_recursive(e: Entity, world: &mut World) {
    if let Some(&p) = world.get::<Parent>(e) {
        let next = world.get::<NextSibling>(e).copied();
        if let Some(fc) = world.get_mut::<FirstChild>(p.0) {
            if fc.0 == e {
                if let Some(next) = next {
                    fc.0 = next.0;
                } else {
                    world.entity_mut(p.0).remove::<FirstChild>();
                }
            }
        }
    }

    fn despawn_recursive_inner(e: Entity, world: &mut World) {
        let mut next_sibling = world.get::<NextSibling>(e).map(|x| x.0);

        while let Some(sibling) = next_sibling {
            next_sibling = world.get::<NextSibling>(sibling).map(|x| x.0);
            if let Some(child) = world.get::<FirstChild>(sibling).map(|x| x.0) {
                despawn_recursive_inner(child, world);
            }
            world.despawn(sibling);
        }
    }

    despawn_recursive_inner(e, world);
}

pub(crate) fn add_to_control_node(cn: Entity, new: Entity, world: &mut World) {
    let mut control = world.get_mut::<Control>(cn).unwrap();
    let lm = control.last_managed;
    control.last_managed = Some(new);

    if let Some(previous_lm) = lm {
        if let Some(ns) = world.get_mut::<NextSibling>(previous_lm) {
            let unmanaged_entity = ns.0;
            ns.0 = new;
            world.entity_mut(new).insert(NextSibling(unmanaged_entity));
        } else {
            world.entity_mut(previous_lm).insert(NextSibling(new));
        }
    } else if let Some(unmanaged) = world.get::<NextSibling>(cn) {
        world.entity_mut(new).insert(NextSibling(unmanaged.0));
    }
}
