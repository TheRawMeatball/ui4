use bevy::{ecs::prelude::*, transform::prelude::*};

use crate::{
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

#[rustfmt::skip]
impl<UO, O, F> Childable<Dynamic> for UO
where
    for<'w, 's> O: Observer<Return<'w, 's> = F>,
    UO: UninitObserver<Observer = O>,
    F: FnOnce(&mut McCtx),
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let c_parent = ctx.world.spawn().id();
        ctx.world.entity_mut(parent).push_children(&[c_parent]);

        let uf = self.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<CnufMarker, _>(move |world| {
                let (func, changed) = observer.get(world);
                if !changed {
                    return;
                }

                world.entity_mut(c_parent).despawn_children_recursive();
                
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
