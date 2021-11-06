use bevy::prelude::{Component, Mut};

use crate::{
    ctx::Ctx,
    observer::{Observer, UninitObserver},
    runtime::{UfMarker, UiScratchSpace, UpdateFunc},
    Dynamic, Static,
};

pub trait Insertable<T, M>: Send + Sync + 'static {
    /// ### Internal method!
    #[doc(hidden)]
    fn insert_ui_val(self, ctx: &mut Ctx);
}

impl<T: Component> Insertable<T, Static> for T {
    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        ctx.world.entity_mut(ctx.current_entity).insert(self);
    }
}

impl<T: Component, O, UO> Insertable<T, Dynamic> for UO
where
    for<'a> O: Observer<'a, Return = T>,
    UO: UninitObserver<Observer = O>,
{
    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        let entity = ctx.current_entity;
        let uf = self.register_self(ctx.world, |mut observer, world| {
            let mut first = true;
            let (uf, marker) = UpdateFunc::new::<T, _>(move |world| {
                let (val, changed) = observer.get(world);
                if !changed && !first {
                    return;
                }
                first = false;
                world.resource_scope(|world, mut ctx: Mut<UiScratchSpace>| {
                    let mut e = world.entity_mut(entity);
                    e.get_mut::<UfMarker<T>>().unwrap().trigger(&mut ctx);
                    e.insert(val);
                })
            });
            world.entity_mut(entity).insert(marker);
            uf
        });
        uf.run(ctx.world);
    }
}
