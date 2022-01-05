use std::marker::PhantomData;

use bevy::{ecs::prelude::*, utils::HashMap};

use crate::{
    lens::ComponentLens,
    runtime::{UfMarker, UiManagedSystems, UiScratchSpace, UpdateFunc},
};

use super::{Observer, UninitObserver};

struct ComponentUpdateFuncs<T>(HashMap<Entity, Vec<UpdateFunc>>, PhantomData<T>);

pub struct ComponentObserver<T: Send + Sync + 'static> {
    pub(crate) entity: Entity,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> Clone for ComponentObserver<T> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            _marker: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> Copy for ComponentObserver<T> {}

impl<'a, T: Component> Observer<'a> for ComponentObserver<T> {
    type Return = &'a T;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        // TODO: use change detection
        (world.get::<T>(self.entity).unwrap(), true)
    }
}

impl<T: Component> UninitObserver for ComponentObserver<T> {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = (uf)(self, world);
        let ufc = uf.clone();
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut marker) = world.get_mut::<UfMarker<T>>(self.entity) {
                marker.add_dependent(uf);
            } else if let Some(mut lists) = world.get_resource_mut::<ComponentUpdateFuncs<T>>() {
                lists.0.entry(self.entity).or_default().push(uf);
            } else {
                systems.0.add_system(component_change_track_system::<T>);
                world.insert_resource(ComponentUpdateFuncs::<T>(
                    [(self.entity, vec![uf])].into_iter().collect(),
                    PhantomData,
                ));
            };
        });
        ufc
    }
}

fn component_change_track_system<T: Component>(
    ui: Res<UiScratchSpace>,
    mut update_funcs: ResMut<ComponentUpdateFuncs<T>>,
    detector: Query<ChangeTrackers<T>>,
) {
    update_funcs.0.retain(|entity, list| {
        if let Ok(ticks) = detector.get(*entity) {
            if ticks.is_changed() {
                ui.process_list(list);
            }
            !list.is_empty()
        } else {
            false
        }
    });
}

pub fn component<T: Component>(entity: Entity) -> ComponentLens<T> {
    ComponentLens(entity, PhantomData)
}
