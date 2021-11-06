use std::marker::PhantomData;

use bevy::ecs::{prelude::*, system::SystemState};

use crate::{
    lens::Identity,
    prelude::WorldLens,
    runtime::{UiManagedSystems, UiScratchSpace, UpdateFunc},
};

use super::{Observer, UninitObserver};

struct ResUpdateFuncs<T>(Vec<UpdateFunc>, PhantomData<T>);

pub struct ResObserverTemplate<R>(PhantomData<R>);
pub struct ResLens<R>(PhantomData<R>);
impl<R: Send + Sync + 'static> WorldLens for ResLens<R> {
    type UninitObserver = ResObserverTemplate<R>;
    type Observer = ResObserver<R>;
    type LensIn = R;
    type Lens = Identity<R>;
    type Out = R;

    fn get<'a>(&mut self, world: &'a World) -> &'a Self::Out {
        world.get_resource::<R>().unwrap()
    }

    fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Self::Out {
        world.get_resource_mut::<R>().unwrap().into_inner()
    }

    fn to_observer(self) -> (Self::UninitObserver, Self::Lens) {
        (ResObserverTemplate(PhantomData), Identity(PhantomData))
    }
}

impl<R> Clone for ResObserverTemplate<R> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<R> Clone for ResLens<R> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
impl<R> Copy for ResLens<R> {}
pub struct ResObserver<R: Send + Sync + 'static>(SystemState<Res<'static, R>>);

impl<'a, R: Send + Sync + 'static> Observer<'a> for ResObserver<R> {
    type Return = &'a R;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
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

/// Gets an lens for a particular resource
pub fn res<R: Send + Sync + 'static>() -> ResLens<R> {
    ResLens(PhantomData)
}

fn resource_change_track_system<T: Send + Sync + 'static>(
    mut ui: ResMut<UiScratchSpace>,
    mut update_funcs: ResMut<ResUpdateFuncs<T>>,
    detector: Res<T>,
) {
    if detector.is_changed() {
        ui.process_list(&mut update_funcs.0);
    }
}
