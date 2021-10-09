use bevy::{ecs::prelude::*, prelude::ControlBundle, transform::prelude::*};

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
            let nc = world.spawn().id();
            let mut parent = world.entity_mut(parent);
            parent.push_children(&[nc]);
            nc
        };
        (self)(&mut McCtx {
            world: &mut ctx.world,
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

#[rustfmt::skip]
impl<UO, F, MF, T> Childable<Dynamic> for ChildMap<UO, MF>
where
    UO: UninitObserver,
    MF: Fn(T) -> F,
    MF: Send + Sync + 'static,
    F: FnOnce(&mut McCtx),
    for<'w, 's> <UO as UninitObserver>::Observer: Observer<Return<'w, 's> = T>,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let c_parent = ctx
            .world
            .spawn()
            .insert_bundle(ControlBundle::default())
            .id();
        ctx.world.entity_mut(parent).push_children(&[c_parent]);

        let mut state = TriggerCallState::new(ctx.world);
        let uf = self.0.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<CnufMarker, _>(move |world| {
                let (ret, changed) = observer.get(world);
                if !changed {
                    return;
                }
                let func = (self.1)(ret);

                let mut params = state.get_mut(world);
                if !trigger_transition_out_cn(
                    c_parent,
                    None,
                    &mut params.0,
                    &params.1,
                    &params.2,
                    &mut params.3,
                ) {
                    world.entity_mut(c_parent).despawn_children_recursive();
                }
                state.apply(world);

                let mut new_child_func = |world: &mut World| {
                    let nc = world.spawn().id();
                    world.entity_mut(c_parent).push_children(&[nc]);
                    nc
                };
                func(&mut McCtx {
                    world,
                    get_new_child: &mut new_child_func,
                });
            });

            world.entity_mut(c_parent).insert(marker);
            uf
        });
        uf.run(&mut ctx.world);
    }
}
