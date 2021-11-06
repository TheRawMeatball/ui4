use bevy::{ecs::prelude::*, transform::prelude::*, utils::HashMap};
use std::{collections::hash_map::Entry, hash::Hash};

use crate::animation::cancel_transition_out;
use crate::dom::{ControlBundle, NodeBundle};
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
        let parent = ctx.current_entity;
        let mut new_child = |world: &mut World| {
            let nc = world.spawn().insert_bundle(NodeBundle::default()).id();
            let mut parent = world.entity_mut(parent);
            parent.push_children(&[nc]);
            nc
        };
        (self)(&mut McCtx {
            world: ctx.world,
            get_new_child: &mut new_child,
        });
    }
}

pub trait ChildMapExt: Sized {
    fn map_child<F>(self, f: F) -> ChildMap<Self, F> {
        ChildMap(self, f)
    }
}

impl<UO> ChildMapExt for UO where UO: UninitObserver {}

pub struct ChildMap<UO, F>(UO, F);

impl<UO, F, MF, T> Childable<Dynamic> for ChildMap<UO, MF>
where
    UO: UninitObserver,
    MF: Fn(T) -> F,
    MF: Send + Sync + 'static,
    F: FnOnce(&mut McCtx),
    for<'a> <UO as UninitObserver>::Observer: Observer<'a, Return = T>,
    T: Clone + Eq + Hash + Send + Sync + 'static,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let main_c_parent = ctx
            .world
            .spawn()
            .insert_bundle(ControlBundle::default())
            .id();
        ctx.world.entity_mut(parent).push_children(&[main_c_parent]);
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
                        None,
                        &mut params.0,
                        &params.1,
                        &params.2,
                        &mut params.3,
                        &mut params.4,
                    ) {
                        world.entity_mut(old).despawn_recursive();
                    }
                }

                let mut parent = parents.entry(ret.clone());
                if let Entry::Occupied(existing) = &mut parent {
                    let existing = existing.get();
                    let mut params = state.get_mut(world);
                    cancel_transition_out(*existing, &mut params.0, &params.1, &mut params.3);
                } else {
                    let c_parent = world.spawn().insert_bundle(ControlBundle::default()).id();
                    world.entity_mut(main_c_parent).push_children(&[c_parent]);
                    parents.insert(ret.clone(), c_parent);

                    let mut new_child_func = |world: &mut World| {
                        let nc = world.spawn().insert_bundle(NodeBundle::default()).id();
                        world.entity_mut(c_parent).push_children(&[nc]);
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

            world.entity_mut(main_c_parent).insert(marker);
            uf
        });
        uf.run(ctx.world);
    }
}
