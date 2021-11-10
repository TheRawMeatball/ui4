use std::marker::PhantomData;

use bevy::{
    ecs::{prelude::*, query::QueryEntityError},
    utils::HashMap,
};

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

impl<'a, T: Component> Observer<'a> for ComponentExistsObserver<T> {
    type Return = bool;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
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
    detector: Query<ChangeTrackers<T>>,
) {
    update_funcs.0.retain(|entity, list| {
        match detector.get(*entity) {
            Ok(ticks) if ticks.is_added() => ui.process_list(list),
            Err(QueryEntityError::QueryDoesNotMatch) => ui.process_list(list),
            Ok(_) => {}
            // remove this tracker when entity is despawned
            Err(QueryEntityError::NoSuchEntity) => return false,
        }
        !list.is_empty()
    });
}
