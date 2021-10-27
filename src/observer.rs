use std::borrow::Borrow;
use std::ops::Deref;

use bevy::prelude::World;

use crate::runtime::UpdateFunc;
use crate::{Dynamic, Static};

mod component;
mod has_component;
mod res;
mod single;

pub use {
    component::ComponentObserver, has_component::ComponentExistsObserver, res::res, single::single,
};

/// Types implementing this trait represent a mapping from world and internal state to a certain output.
pub trait UninitObserver: Send + Sync + 'static {
    #[doc(hidden)]
    type Observer: Observer;

    /// ### Internal method!
    #[doc(hidden)]
    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc;
}

pub trait Observer: Send + Sync + 'static {
    type Return<'a>;

    /// ### INTERNAL METHOD!
    #[doc(hidden)]
    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool);
}

#[derive(Clone, Copy)]
pub struct Map<O, F>(O, F);
impl<O, F> Observer for Map<O, F>
where
    O: Observer,
    F: for<'a> Fn<(O::Return<'a>,)> + Send + Sync + 'static,
{
    type Return<'a> = <F as FnOnce<(O::Return<'a>,)>>::Output;

    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool) {
        let (val, change) = self.0.get(world);
        ((self.1)(val), change)
    }
}
impl<O, MF> UninitObserver for Map<O, MF>
where
    O: UninitObserver,
    MF: for<'a> Fn<(<<O as UninitObserver>::Observer as Observer>::Return<'a>,)>,
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

#[derive(Clone, Copy)]
pub struct ClonedTemplate<O>(O);
pub struct Cloned<O>(O);
#[rustfmt::skip]
impl<O, T: Clone + 'static> Observer for Cloned<O>
where
    O: for<'a> Observer<Return<'a> = &'a T>,
{
    type Return<'a> = T;

    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool) {
        let (val, change) = self.0.get(world);
        (val.clone(), change)
    }
}
#[rustfmt::skip]
impl<UO, O, T: Clone + 'static> UninitObserver for ClonedTemplate<UO>
where
    O: for<'a> Observer<Return<'a> = &'a T>,
    UO: UninitObserver<Observer = O>,
{
    type Observer = Cloned<O>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, move |obs, world| uf(Cloned(obs), world))
    }
}

#[derive(Clone, Copy)]
pub struct CopiedTemplate<O>(O);
pub struct Copied<O>(O);
#[rustfmt::skip]
impl<O, T: Copy + 'static> Observer for Copied<O>
where
    O: for<'a> Observer<Return<'a> = &'a T>,
{
    type Return<'a> = T;

    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool) {
        let (val, change) = self.0.get(world);
        (val.clone(), change)
    }
}
#[rustfmt::skip]
impl<UO, O, T: Copy + 'static> UninitObserver for CopiedTemplate<UO>
where
    O: for<'a> Observer<Return<'a> = &'a T>,
    UO: UninitObserver<Observer = O>,
{
    type Observer = Copied<O>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, move |obs, world| uf(Copied(obs), world))
    }
}

#[derive(Clone, Copy)]
pub struct DereffedTemplate<O>(O);
pub struct Dereffed<O>(O);
#[rustfmt::skip]
impl<O, T: Deref + 'static> Observer for Dereffed<O>
where
    O: for<'a> Observer<Return<'a> = &'a T>,
{
    type Return<'a> = &'a T::Target;

    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool) {
        let (val, change) = self.0.get(world);
        (val.deref(), change)
    }
}
#[rustfmt::skip]
impl<UO, O, T: Deref + 'static> UninitObserver for DereffedTemplate<UO>
where
    O: for<'a> Observer<Return<'a> = &'a T>,
    UO: UninitObserver<Observer = O>,
{
    type Observer = Dereffed<O>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, move |obs, world| uf(Dereffed(obs), world))
    }
}

#[derive(Clone)]
pub struct And<O1, O2>(O1, O2);
impl<O1: Observer, O2: Observer> Observer for And<O1, O2> {
    type Return<'a> = (O1::Return<'a>, O2::Return<'a>);

    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool) {
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

pub struct Dedup<O: Observer>(Option<O::Return<'static>>, O);

#[rustfmt::skip]
impl<UO, O, T> UninitObserver for DedupTemplate<UO>
where
    UO: UninitObserver<Observer = O>,
    O: for<'a> Observer<Return<'a> = T>,
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
    O: for<'a> Observer<Return<'a> = T>,
    T: PartialEq + Send + Sync + 'static,
{
    type Return<'a> = &'a T;

    fn get<'a>(&'a mut self, world: &'a World) -> (Self::Return<'a>, bool) {
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
        F: for<'a> Fn<(<<Self as UninitObserver>::Observer as Observer>::Return<'a>,)>,
        F: Send + Sync + 'static,
    {
        Map(self, f)
    }
    fn and<O>(self, o: O) -> And<Self, O>
    where
        O: UninitObserver,
    {
        And(self, o)
    }
    fn dedup(self) -> DedupTemplate<Self> {
        DedupTemplate(self)
    }

    #[rustfmt::skip]
    fn copied<T: Copy>(self) -> CopiedTemplate<Self>
    where
        <Self as UninitObserver>::Observer: for<'a> Observer<Return<'a> = &'a T>,
    {
        CopiedTemplate(self)
    }

    #[rustfmt::skip]
    fn cloned<T: Clone>(self) -> ClonedTemplate<Self>
    where
        <Self as UninitObserver>::Observer: for<'a> Observer<Return<'a> = &'a T>,
    {
        ClonedTemplate(self)
    }

    #[rustfmt::skip]
    fn dereffed<T: Deref>(self) -> DereffedTemplate<Self>
    where
        <Self as UninitObserver>::Observer: for<'a> Observer<Return<'a> = &'a T>,
    {
        DereffedTemplate(self)
    }
}

impl<T: UninitObserver> ObserverExt for T {}

#[derive(Clone)]
pub struct StaticObserver<T>(T);

impl<T: Send + Sync + 'static> Observer for StaticObserver<T> {
    type Return<'a> = &'a T;

    fn get<'a>(&'a mut self, _: &'a bevy::prelude::World) -> (Self::Return<'a>, bool) {
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
pub trait IntoObserver<T, M>: Send + Sync + 'static {
    type UninitObserver: UninitObserver<Observer = Self::Observer>;
    type Observer: for<'a> Observer<Return<'a> = Self::ObserverReturn<'a>>;
    type ObserverReturn<'a>: Borrow<T>;
    fn into_observer(self) -> Self::UninitObserver;
}

impl<T: Send + Sync + 'static, I: Into<T> + Send + Sync + 'static> IntoObserver<T, Static> for I {
    type UninitObserver = StaticObserver<T>;
    type Observer = StaticObserver<T>;
    type ObserverReturn<'a> = &'a T;

    fn into_observer(self) -> Self::Observer {
        StaticObserver(self.into())
    }
}

#[rustfmt::skip]
impl<T, O: for<'a> Observer<Return<'a> = T>, UO: UninitObserver<Observer = O>> IntoObserver<T, Dynamic> for UO {
    type UninitObserver = Self;
    type Observer = O;
    type ObserverReturn<'a> = O::Return<'a>;

    fn into_observer(self) -> Self::UninitObserver {
        self
    }
}
