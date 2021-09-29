use bevy::prelude::Component;

use crate::{
    ctx::Ctx,
    observer::{Observer, UninitObserver},
    runtime::UpdateFunc,
    Dynamic, Static,
};

pub trait Insertable<T, M>: Send + Sync + 'static {
    #[track_caller]
    fn insert_ui_val(self, ctx: &mut Ctx);
}

impl<T: Component> Insertable<T, Static> for T {
    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        ctx.current_entity().insert(self);
    }
}

#[rustfmt::skip]
impl<T: Component, O, UO> Insertable<T, Dynamic> for UO
where
    for<'w, 's> O: Observer<Return<'w, 's> = T>,
    UO: UninitObserver<Observer = O>,
{
    #[track_caller]
    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        let entity = ctx.current_entity;
        let uf = self.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<T, _>(move |world| {
                let (val, changed) = observer.get(world);
                if !changed {
                    return;
                }
                world.entity_mut(entity).insert(val);
            });
            world.entity_mut(entity).insert(marker);
            uf
        });
        uf.run(&mut ctx.world);
    }
}
