use std::any::Any;

use bevy::{
    ecs::{prelude::*, world::EntityMut},
    prelude::{BuildWorldChildren, DespawnRecursiveExt, Entity},
};

use crate::{
    ctx::{Ctx, McCtx},
    observer::{Observer, UninitObserver},
    runtime::{UfMarker, UpdateFunc},
    Dynamic, Static,
};

pub mod tracked;

fn get_marker_list(mut current: EntityMut) -> &mut Vec<ChildNodeGroupKind> {
    if current.get::<ManagedChildrenTracker>().is_none() {
        current.insert(ManagedChildrenTracker::default());
    }
    &mut current
        .get_mut::<ManagedChildrenTracker>()
        .unwrap()
        .into_inner()
        .children
}

fn get_index_from_cng_list(list: &[ChildNodeGroupKind], group_index: usize) -> usize {
    list[..group_index]
        .iter()
        .map(|node| match node {
            ChildNodeGroupKind::StaticChildren(len) => *len,
            ChildNodeGroupKind::Dynamic(entities, _) => entities.len(),
            ChildNodeGroupKind::List(entities, _, _) => entities.iter().map(|v| v.len()).sum(),
        })
        .sum()
}

struct ChildNodeUpdateFuncMarker;

enum ChildNodeGroupKind {
    StaticChildren(usize),
    Dynamic(Vec<Entity>, UfMarker<ChildNodeUpdateFuncMarker>),
    List(
        Vec<Vec<Entity>>,
        Box<dyn Any + Send + Sync>,
        UfMarker<ChildNodeUpdateFuncMarker>,
    ),
}

#[derive(Default, Component)]
struct ManagedChildrenTracker {
    children: Vec<ChildNodeGroupKind>,
}

pub trait Childable<M> {
    fn insert(self, ctx: &mut Ctx);
}

impl<Func> Childable<Static> for Func
where
    Func: FnOnce(&mut McCtx),
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let mut new_child = |world: &mut World| {
            let nc = world.spawn().id();
            let mut parent = world.entity_mut(parent);
            parent.push_children(&[nc]);
            let list = get_marker_list(parent);
            match list.last_mut() {
                Some(ChildNodeGroupKind::StaticChildren(count)) => *count += 1,
                _ => list.push(ChildNodeGroupKind::StaticChildren(1)),
            }
            nc
        };
        (self)(&mut McCtx {
            world: &mut ctx.world,
            get_new_child: &mut new_child,
        });
    }
}

#[rustfmt::skip]
impl<UO, O, F> Childable<Dynamic> for UO
where
    for<'w, 's> O: Observer<Return<'w, 's> = F>,
    UO: UninitObserver<Observer = O>,
    F: FnOnce(&mut McCtx),
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let list = get_marker_list(ctx.current_entity());
        let group_index = list.len();

        let uf = self.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<ChildNodeUpdateFuncMarker, _>(move |world| {
                let (func, changed) = observer.get(world);
                if !changed {
                    return;
                }
                let list = get_marker_list(world.entity_mut(parent));
                let index = get_index_from_cng_list(list, group_index);
                let entities = match &mut list[group_index] {
                    ChildNodeGroupKind::Dynamic(entities, _) => entities,
                    _ => unreachable!(),
                };
                // TODO: find a way to somehow do double buffering or sth with these vecs
                let mut old_entities = std::mem::replace(entities, Vec::new());
                for &entity in old_entities.iter() {
                    world.entity_mut(entity).despawn_recursive();
                }
                old_entities.clear();
                let list = get_marker_list(world.entity_mut(parent));
                match &mut list[group_index] {
                    ChildNodeGroupKind::Dynamic(l, _) => *l = old_entities,
                    _ => unreachable!(),
                };
                let mut new_child_func = |world: &mut World| {
                    let nc = world.spawn().id();
                    let list = get_marker_list(world.entity_mut(parent));
                    let entities = match &mut list[group_index] {
                        ChildNodeGroupKind::Dynamic(entities, _) => entities,
                        _ => unreachable!(),
                    };
                    let len = entities.len();
                    entities.push(nc);
                    world.entity_mut(parent).insert_children(index + len, &[nc]);
                    nc
                };
                func(&mut McCtx {
                    world,
                    get_new_child: &mut new_child_func,
                });
            });

            get_marker_list(world.entity_mut(parent))
                .push(ChildNodeGroupKind::Dynamic(vec![], marker));
            uf
        });
        uf.run(&mut ctx.world);
    }
}
