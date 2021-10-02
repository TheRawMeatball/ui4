use std::marker::PhantomData;

use bevy::ecs::{prelude::*, system::SystemState};

use crate::runtime::{UiManagedSystems, UiScratchSpace, UpdateFunc};

use super::{process_update_func_list, Observer, UninitObserver};

struct ResUpdateFuncs<T>(Vec<UpdateFunc>, PhantomData<T>);

pub struct ResObserverTemplate<R>(PhantomData<R>);

impl<R> Clone for ResObserverTemplate<R> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
pub struct ResObserver<R: Send + Sync + 'static>(SystemState<Res<'static, R>>);

impl<R: Send + Sync + 'static> Observer for ResObserver<R> {
    type Return<'w, 's> = &'w R;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let res = self.0.get(world);
        let changed = res.is_changed();
        (res.into_inner(), changed)
    }
}
impl<R: Send + Sync + 'static> UninitObserver for ResObserverTemplate<R> {
    type Observer = ResObserver<R>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = uf(ResObserver(SystemState::new(world)), world);
        let ufc = uf.clone();
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut list) = world.get_resource_mut::<ResUpdateFuncs<R>>() {
                list.0.push(uf);
            } else {
                systems.0.add_system(resource_change_track_system::<R>);
                world.insert_resource(ResUpdateFuncs::<R>(vec![uf], PhantomData));
            };
        });
        ufc
    }
}

/// Gets an observer for a particular resource
pub fn res<R: Send + Sync + 'static>() -> ResObserverTemplate<R> {
    ResObserverTemplate(PhantomData)
}

fn resource_change_track_system<T: Send + Sync + 'static>(
    mut ui: ResMut<UiScratchSpace>,
    mut update_funcs: ResMut<ResUpdateFuncs<T>>,
    detector: Res<T>,
) {
    if detector.is_changed() {
        process_update_func_list(&mut update_funcs.0, &mut ui)
    }
}
