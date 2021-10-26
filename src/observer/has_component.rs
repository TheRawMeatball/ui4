use std::marker::PhantomData;

use bevy::{ecs::prelude::*, utils::HashMap};

use crate::runtime::{UiManagedSystems, UiScratchSpace, UpdateFunc};

use super::{Observer, UninitObserver};

struct ComponentExistsUpdateFuncs<T>(HashMap<Entity, Vec<UpdateFunc>>, PhantomData<T>);

pub struct ComponentExistsObserver<T: Send + Sync + 'static>(
    pub(crate) Entity,
    pub(crate) PhantomData<T>,
);

impl<T: Send + Sync + 'static> Clone for ComponentExistsObserver<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T: Send + Sync + 'static> Copy for ComponentExistsObserver<T> {}

impl<T: Component> Observer for ComponentExistsObserver<T> {
    type Return<'w, 's> = bool;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        // TODO: use change detection
        (world.get::<T>(self.0).is_some(), true)
    }
}

impl<T: Component> UninitObserver for ComponentExistsObserver<T> {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = (uf)(self, world);
        let ufc = uf.clone();
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut lists) = world.get_resource_mut::<ComponentExistsUpdateFuncs<T>>() {
                lists.0.entry(self.0).or_default().push(uf);
            } else {
                systems.0.add_system(component_exist_track_system::<T>);
                world.insert_resource(ComponentExistsUpdateFuncs::<T>(
                    [(self.0, vec![uf])].into_iter().collect(),
                    PhantomData,
                ));
            };
        });
        ufc
    }
}

fn component_exist_track_system<T: Component>(
    mut ui: ResMut<UiScratchSpace>,
    mut update_funcs: ResMut<ComponentExistsUpdateFuncs<T>>,
    added_detector: Query<(), Added<T>>,
    removed_detector: RemovedComponents<T>,
) {
    update_funcs.0.retain(|entity, list| {
        if added_detector.get(*entity).is_ok() {
            ui.process_list(list);
        }
        !list.is_empty()
    });
    for e in removed_detector.iter() {
        if let Some(list) = update_funcs.0.get_mut(&e) {
            ui.process_list(list);
            if list.is_empty() {
                update_funcs.0.remove(&e);
            }
        }
    }
}
