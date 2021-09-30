use std::borrow::Borrow;

use bevy::prelude::World;

use crate::runtime::{UiScratchSpace, UpdateFunc};
use crate::{Dynamic, Static};

mod component;
mod has_component;
mod res;
mod single;

pub use {
    component::ComponentObserver, has_component::ComponentExistsObserver, res::res, single::single,
};

pub trait UninitObserver: Send + Sync + 'static {
    type Observer: Observer;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc;
}

pub trait Observer: Send + Sync + 'static {
    type Return<'w, 's>;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool);
}

#[derive(Clone)]
pub struct Map<O, F>(O, F);
impl<O, F> Observer for Map<O, F>
where
    O: Observer,
    F: for<'w, 's> Fn<(O::Return<'w, 's>,)> + Send + Sync + 'static,
{
    type Return<'w, 's> = <F as FnOnce<(O::Return<'w, 's>,)>>::Output;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let (val, change) = self.0.get(world);
        ((self.1)(val), change)
    }
}
impl<O, MF> UninitObserver for Map<O, MF>
where
    O: UninitObserver,
    MF: for<'w, 's> Fn<(<<O as UninitObserver>::Observer as Observer>::Return<'w, 's>,)>,
    MF: Send + Sync + 'static,
{
    type Observer = Map<O::Observer, MF>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, move |obs, world| uf(Map(obs, self.1), world))
    }
}

#[derive(Clone)]
pub struct And<O1, O2>(O1, O2);
impl<O1: Observer, O2: Observer> Observer for And<O1, O2> {
    type Return<'w, 's> = (O1::Return<'w, 's>, O2::Return<'w, 's>);

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let (v0, c0) = self.0.get(world);
        let (v1, c1) = self.1.get(world);
        ((v0, v1), c0 || c1)
    }
}

impl<O1: UninitObserver, O2: UninitObserver> UninitObserver for And<O1, O2> {
    type Observer = And<O1::Observer, O2::Observer>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0.register_self(world, |obs1, world| {
            self.1
                .register_self(world, move |obs2, world| (uf)(And(obs1, obs2), world))
        })
    }
}

#[derive(Clone)]
pub struct DedupTemplate<O>(O);

pub struct Dedup<O: Observer>(Option<O::Return<'static, 'static>>, O);

#[rustfmt::skip]
impl<UO, O, T> UninitObserver for DedupTemplate<UO>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = T>,
    T: PartialEq + Send + Sync + 'static,
{
    type Observer = Dedup<O>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, |obs, world| (uf)(Dedup(None, obs), world))
    }
}

#[rustfmt::skip]
impl<O, T> Observer for Dedup<O>
where
    O: for<'w, 's> Observer<Return<'w, 's> = T>,
    T: PartialEq + Send + Sync + 'static,
{
    type Return<'w, 's> = &'s T;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let (maybe_new, changed) = self.1.get(world);
        if self.0.is_none() {
            self.0 = Some(maybe_new);
            return (self.0.as_ref().unwrap(), true);
        }
        if !changed || self.0.as_ref() == Some(&maybe_new) {
            (self.0.as_ref().unwrap(), false)
        } else {
            self.0 = Some(maybe_new);
            (self.0.as_ref().unwrap(), true)
        }
    }
}

pub trait ObserverExt: UninitObserver + Sized {
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        F: for<'w, 's> Fn<(<<Self as UninitObserver>::Observer as Observer>::Return<'w, 's>,)>,
        F: Send + Sync + 'static;
    fn and<O>(self, o: O) -> And<Self, O>
    where
        O: UninitObserver;
    fn dedup(self) -> DedupTemplate<Self>;
}

impl<T: UninitObserver> ObserverExt for T {
    fn map<F>(self, f: F) -> Map<Self, F> {
        Map(self, f)
    }

    fn and<O>(self, o: O) -> And<Self, O> {
        And(self, o)
    }

    fn dedup(self) -> DedupTemplate<Self> {
        DedupTemplate(self)
    }
}

fn process_update_func_list(list: &mut Vec<UpdateFunc>, ui: &mut UiScratchSpace) {
    let mut i = 0usize;
    loop {
        if i == list.len() {
            break;
        }
        let relevant_uf = &list[i];
        if relevant_uf.flagged() {
            list.swap_remove(i);
        } else {
            ui.register_update_func(relevant_uf.clone());
            i += 1;
        }
    }
}

#[derive(Clone)]
pub struct StaticObserver<T>(T);

impl<T: Send + Sync + 'static> Observer for StaticObserver<T> {
    type Return<'w, 's> = &'s T;

    fn get<'w, 's>(&'s mut self, _: &'w bevy::prelude::World) -> (Self::Return<'w, 's>, bool) {
        (&self.0, false)
    }
}

impl<T: Send + Sync + 'static> UninitObserver for StaticObserver<T> {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        uf(self, world)
    }
}

#[rustfmt::skip]
pub trait IntoObserver<T, M>: Send + Sync + 'static
{
    type UninitObserver: UninitObserver<Observer = Self::Observer>;
    type Observer: for<'w, 's> Observer<Return<'w, 's> = Self::ObserverReturn<'w, 's>>;
    type ObserverReturn<'w, 's>: Borrow<T>;
    fn into_observer(self) -> Self::UninitObserver;
}

impl<T: Send + Sync + 'static> IntoObserver<T, Static> for T {
    type UninitObserver = StaticObserver<T>;
    type Observer = StaticObserver<T>;
    type ObserverReturn<'w, 's> = &'s T;

    fn into_observer(self) -> Self::Observer {
        StaticObserver(self)
    }
}

#[rustfmt::skip]
impl<T, O: for<'w, 's> Observer<Return<'w, 's> = T>, UO: UninitObserver<Observer = O>> IntoObserver<T, Dynamic> for UO {
    type UninitObserver = Self;
    type Observer = O;
    type ObserverReturn<'w, 's> = O::Return<'w, 's>;

    fn into_observer(self) -> Self::UninitObserver {
        self
    }

}
