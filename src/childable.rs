use bevy::{ecs::prelude::*, utils::HashMap};
use std::{collections::hash_map::Entry, hash::Hash};

use crate::animation::cancel_transition_out;
use crate::dom::{
    add_to_control_node, despawn_control_node, Control, FirstChild, NextSibling, Parent,
};
use crate::{
    animation::{trigger_transition_out_cn, TriggerCallState},
    ctx::{Ctx, McCtx},
    observer::{Observer, UninitObserver},
    runtime::UpdateFunc,
    Dynamic, Static,
};

pub mod tracked;

struct CnufMarker;

pub trait Childable<M> {
    fn insert(self, ctx: &mut Ctx);
}

impl<Func> Childable<Static> for Func
where
    Func: FnOnce(&mut McCtx),
{
    fn insert(self, ctx: &mut Ctx) {
        let (parent, mut last_child) = if ctx.world.get::<Control>(ctx.current_entity).is_some() {
            let parent = ctx.world.get::<Parent>(ctx.current_entity).unwrap().0;
            (parent, Some(ctx.current_entity))
        } else {
            (ctx.current_entity, ctx.last_child)
        };
        let mut new_child = move |world: &mut World| {
            let nc = world.spawn().insert(Parent(parent)).id();
            if let Some(lc) = last_child {
                world.entity_mut(lc).insert(NextSibling(nc));
            } else {
                world.entity_mut(parent).insert(FirstChild(nc));
            }
            last_child = Some(nc);
            nc
        };
        (self)(&mut McCtx {
            world: &mut ctx.world,
            get_new_child: &mut new_child,
        });
        ctx.last_child = last_child;
    }
}

pub trait ChildMapExt: Sized {
    fn map_child<F>(self, f: F) -> ChildMap<Self, F> {
        ChildMap(self, f)
    }
}

impl<UO> ChildMapExt for UO where UO: UninitObserver {}

pub struct ChildMap<UO, F>(UO, F);

#[rustfmt::skip]
impl<UO, F, MF, T> Childable<Dynamic> for ChildMap<UO, MF>
where
    UO: UninitObserver,
    MF: Fn(T) -> F,
    MF: Send + Sync + 'static,
    F: FnOnce(&mut McCtx),
    for<'w, 's> <UO as UninitObserver>::Observer: Observer<Return<'w, 's> = T>,
    T: Clone + Eq + Hash + Send + Sync + 'static,
{
    fn insert(self, ctx: &mut Ctx) {
        let (parent_entity, main_c_node) = if ctx.world.get::<Control>(ctx.current_entity).is_some()
        {
            let parent = ctx.world.get::<Parent>(ctx.current_entity).unwrap().0;
            (parent, ctx.current_entity)
        } else {
            (
                ctx.current_entity,
                ctx.world
                    .spawn()
                    .insert(Control::default())
                    .insert(Parent(ctx.current_entity))
                    .id(),
            )
        };

        if let Some(lc) = ctx.last_child {
            ctx.world.entity_mut(lc).insert(NextSibling(main_c_node));
        } else {
            ctx.world
                .entity_mut(parent_entity)
                .insert(FirstChild(main_c_node));
        }
        let mut parents = HashMap::<T, Entity>::default();
        let mut state = TriggerCallState::new(ctx.world);
        let mut last = None;
        let uf = self.0.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<CnufMarker, _>(move |world| {
                let (ret, changed) = observer.get(world);

                parents.retain(|_, entity| world.entities().contains(*entity));

                if !changed || Some(&ret) == last.as_ref() {
                    return;
                }

                if let Some(&old) = last.as_ref().and_then(|e| parents.get(e)) {
                    let mut params = state.get_mut(world);
                    if !trigger_transition_out_cn(
                        old,
                        world.get::<Control>(old).unwrap().last_managed,
                        None,
                        &mut params.0,
                        &params.1,
                        &params.2,
                        &mut params.3,
                        &mut params.4,
                        &mut params.5,
                    ) {
                        despawn_control_node(old, world);
                    }
                }

                let mut parent = parents.entry(ret.clone());
                if let Entry::Occupied(existing) = &mut parent {
                    let existing = existing.get();
                    let mut params = state.get_mut(world);
                    cancel_transition_out(
                        *existing,
                        &mut params.0,
                        &params.1,
                        &params.2,
                        &mut params.4,
                    );
                } else {
                    let c_node = world
                        .spawn()
                        .insert(Control::default())
                        .insert(Parent(parent_entity))
                        .id();

                    add_to_control_node(main_c_node, c_node, world);

                    parent.insert(c_node);

                    let mut new_child_func = |world: &mut World| {
                        let nc = world.spawn().insert(Parent(parent_entity)).id();
                        add_to_control_node(c_node, nc, world);
                        nc
                    };
                    (self.1)(ret.clone())(&mut McCtx {
                        world,
                        get_new_child: &mut new_child_func,
                    });
                }

                last = Some(ret);
                state.apply(world);
            });

            world.entity_mut(main_c_node).insert(marker);
            uf
        });
        uf.run(&mut ctx.world);
        ctx.last_child = Some(main_c_node);
    }
}
