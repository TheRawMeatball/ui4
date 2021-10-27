use crate::runtime::{UiManagedSystems, UiScratchSpace};
use crate::{observer::UninitObserver, runtime::UpdateFunc};
use bevy::ecs::query::ChangeTrackers;
use bevy::ecs::world::Mut;
use bevy::prelude::ResMut;
use bevy::{ecs::component::Component, prelude::Query};
use std::marker::PhantomData;

use bevy::{
    ecs::query::{Fetch, ReadOnlyFetch, WorldQuery},
    prelude::{QueryState, World},
};

use super::Observer;

struct SingleUpdateFuncs<T: SingleObserverTuple>(Vec<UpdateFunc>, PhantomData<T>);

pub struct UninitSingleObserver<T>(PhantomData<T>);
pub struct SingleObserver<T: SingleObserverTuple>(QueryState<T::DataQuery>);

pub fn single<T: WorldQuery + 'static>() -> UninitSingleObserver<T>
where
    T::Fetch: ReadOnlyFetch,
{
    UninitSingleObserver(PhantomData)
}

impl<T: SingleObserverTuple> UninitObserver for UninitSingleObserver<T>
where
    <T::DataQuery as WorldQuery>::Fetch: ReadOnlyFetch,
    <T::ChangeDetectionQuery as WorldQuery>::Fetch: ReadOnlyFetch,
{
    type Observer = SingleObserver<T>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = uf(SingleObserver(world.query::<T::DataQuery>()), world);
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut list) = world.get_resource_mut::<SingleUpdateFuncs<T>>() {
                list.0.push(uf.clone());
            } else {
                systems.0.add_system(single_change_track_system::<T>);
                world.insert_resource(SingleUpdateFuncs::<T>(vec![uf.clone()], PhantomData));
            };
        });
        uf
    }
}

impl<'a, T: SingleObserverTuple> Observer<'a> for SingleObserver<T>
where
    <T::DataQuery as WorldQuery>::Fetch: ReadOnlyFetch,
{
    type Return = <<T::DataQuery as WorldQuery>::Fetch as Fetch<'a, 'a>>::Item;

    fn get(&'a mut self, world: &'a bevy::prelude::World) -> (Self::Return, bool) {
        let mut iter = self.0.iter(world);
        let item = iter.next().unwrap();
        assert!(iter.next().is_none());
        (item, true)
    }
}

pub trait SingleObserverTuple: Send + Sync + 'static {
    type DataQuery: WorldQuery;
    type ChangeDetectionQuery: WorldQuery;

    fn get_changed(
        cdq: &<<Self::ChangeDetectionQuery as WorldQuery>::Fetch as Fetch<'_, '_>>::Item,
    ) -> bool;
}

fn single_change_track_system<T: SingleObserverTuple>(
    q: Query<T::ChangeDetectionQuery>,
    mut list: ResMut<SingleUpdateFuncs<T>>,
    mut ui: ResMut<UiScratchSpace>,
) where
    <T::ChangeDetectionQuery as WorldQuery>::Fetch: ReadOnlyFetch,
{
    let flags = q.single();
    if T::get_changed(&flags) {
        ui.process_list(&mut list.0);
    }
}

macro_rules! impl_singleobserver_tuple {
    ($($item:ident),*) => {
        #[allow(unused_parens)]
        #[allow(non_snake_case)]
        impl<$($item: Component),*> SingleObserverTuple for ($($item,)*) {
            type DataQuery = ($(&'static $item,)*);
            type ChangeDetectionQuery = ($(ChangeTrackers<$item>,)*);

            fn get_changed(
                cdq: &<<Self::ChangeDetectionQuery as WorldQuery>::Fetch as Fetch<'_, '_>>::Item,
            ) -> bool {
                let ($($item,)*) = cdq;
                false $( || $item.is_changed())*
            }
        }
    };
}

impl_singleobserver_tuple!(A);
impl_singleobserver_tuple!(A, B);
impl_singleobserver_tuple!(A, B, C);
