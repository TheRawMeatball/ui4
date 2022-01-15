use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use bevy::prelude::World;

use crate::runtime::UpdateFunc;
use crate::{Dynamic, Static};

mod component;
mod has_component;
mod opt_component;
mod res;
mod single;

pub use {
    component::component, component::ComponentObserver, has_component::ComponentExistsObserver,
    opt_component::OptComponentObserver, res::res, single::single,
};

/// Types implementing this trait represent a mapping from world and internal state to a certain output.
pub trait UninitObserver: Send + Sync + 'static {
    #[doc(hidden)]
    type Observer: for<'a> Observer<'a>;

    /// ### INTERNAL METHOD!
    #[doc(hidden)]
    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc;
}

pub trait Observer<'a>: Send + Sync + 'static {
    type Return;

    /// ### INTERNAL METHOD!
    #[doc(hidden)]
    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool);
}

#[derive(Clone, Copy)]
pub struct Map<O, F>(O, F);
impl<'a, O, F> Observer<'a> for Map<O, F>
where
    O: for<'x> Observer<'x>,
    F: for<'x> FnHack<'x, O>,
{
    type Return = <F as FnHack<'a, O>>::Return;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let (val, change) = self.0.get(world);
        (self.1.call(val), change)
    }
}
impl<O, MF> UninitObserver for Map<O, MF>
where
    O: UninitObserver,
    MF: for<'a> FnHack<'a, O::Observer>,
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

pub trait FnHack<'a, O: for<'x> Observer<'x>>: Send + Sync + 'static {
    type Return;
    fn call(&self, x: <O as Observer<'a>>::Return) -> Self::Return;
}
impl<'a, O, F, T> FnHack<'a, O> for F
where
    O: for<'x> Observer<'x>,
    F: Fn(<O as Observer<'a>>::Return) -> T,
    F: Send + Sync + 'static,
{
    type Return = T;
    fn call(&self, x: <O as Observer<'a>>::Return) -> Self::Return {
        (self)(x)
    }
}

#[derive(Clone, Copy)]
pub struct ClonedTemplate<O>(O);
pub struct Cloned<O>(O);
impl<'a, O, T: Clone + 'static> Observer<'a> for Cloned<O>
where
    O: for<'x> Observer<'x, Return = &'x T>,
{
    type Return = T;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let (val, change) = self.0.get(world);
        (val.clone(), change)
    }
}

impl<UO, O, T: Clone + 'static> UninitObserver for ClonedTemplate<UO>
where
    O: for<'a> Observer<'a, Return = &'a T>,
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
impl<'a, O, T: Copy + 'static> Observer<'a> for Copied<O>
where
    O: for<'x> Observer<'x, Return = &'x T>,
{
    type Return = T;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let (val, change) = self.0.get(world);
        (*val, change)
    }
}

impl<UO, O, T: Copy + 'static> UninitObserver for CopiedTemplate<UO>
where
    O: for<'x> Observer<'x, Return = &'x T>,
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

impl<'a, O, T: Deref + 'static> Observer<'a> for Dereffed<O>
where
    O: for<'x> Observer<'x, Return = &'x T>,
{
    type Return = &'a T::Target;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let (val, change) = self.0.get(world);
        (val.deref(), change)
    }
}

impl<UO, O, T: Deref + 'static> UninitObserver for DereffedTemplate<UO>
where
    O: for<'a> Observer<'a, Return = &'a T>,
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

#[derive(Copy, Clone)]
pub struct And<O1, O2>(O1, O2);
impl<'a, O1: Observer<'a>, O2: Observer<'a>> Observer<'a> for And<O1, O2> {
    type Return = (O1::Return, O2::Return);

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
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

pub struct Dedup<O: for<'a> Observer<'a>>(Option<<O as Observer<'static>>::Return>, O);

impl<UO, O, T> UninitObserver for DedupTemplate<UO>
where
    UO: UninitObserver<Observer = O>,
    O: for<'a> Observer<'a, Return = T>,
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

impl<'a, O, T> Observer<'a> for Dedup<O>
where
    O: for<'x> Observer<'x, Return = T>,
    T: PartialEq + Send + Sync + 'static,
{
    type Return = &'a T;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
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

#[derive(Clone)]
pub struct FlattenTemplate<O>(O);

pub struct Flatten<O>(Arc<Mutex<Option<O>>>, Option<O>);

impl<UO, O, UO2, O2> UninitObserver for FlattenTemplate<UO>
where
    UO: UninitObserver<Observer = O>,
    UO2: UninitObserver<Observer = O2>,
    O: for<'a> Observer<'a, Return = UO2>,
    O2: for<'a> Observer<'a>,
{
    type Observer = Flatten<O2>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0.register_self(world, |mut obs, world| {
            let arc = Arc::new(Mutex::new(None));
            let (uo2, _) = obs.get(world);
            let inner_uf = uo2.register_self(world, |o2, world| {
                *arc.lock().unwrap() = Some(o2);
                uf(Flatten(arc.clone(), None), world)
            });

            let (uf, marker) = UpdateFunc::new::<(), _>(move |world| {
                let (uo2, changed) = obs.get(world);
                if changed {
                    uo2.register_self(world, |o2, _world| {
                        *arc.lock().unwrap() = Some(o2);
                        inner_uf.clone()
                    });
                    inner_uf.run(world);
                }
            });
            marker.forget();
            uf
        })
    }
}

pub struct FlattenReturn<'a, O: Observer<'a>> {
    this: *mut Flatten<O>,
    inner: Option<O::Return>,
}

impl<'a, O: Observer<'a>> FlattenReturn<'a, O>
where
    O::Return: 'static,
{
    pub fn into_inner(mut self) -> O::Return {
        self.inner.take().unwrap()
    }
}

impl<'a, O: Observer<'a>> std::ops::Deref for FlattenReturn<'a, O> {
    type Target = O::Return;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<'a, O: Observer<'a>> Drop for FlattenReturn<'a, O> {
    fn drop(&mut self) {
        drop(self.inner.take());
        let this = unsafe { &mut *self.this };
        *this.0.lock().unwrap() = this.1.take();
    }
}

impl<'a, O: Observer<'a>> Observer<'a> for Flatten<O> {
    type Return = FlattenReturn<'a, O>;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let this = self as *mut _;
        let mut guard = self.0.lock().unwrap();
        self.1 = Some(guard.take().unwrap());
        let (inner, changed) = self.1.as_mut().unwrap().get(world);
        (
            FlattenReturn {
                inner: Some(inner),
                this,
            },
            changed,
        )
    }
}

pub trait ObserverExt: UninitObserver + Sized {
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        F: for<'a> FnHack<'a, <Self as UninitObserver>::Observer>,
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

    fn copied<T: Copy>(self) -> CopiedTemplate<Self>
    where
        <Self as UninitObserver>::Observer: for<'a> Observer<'a, Return = &'a T>,
    {
        CopiedTemplate(self)
    }

    fn cloned<T: Clone>(self) -> ClonedTemplate<Self>
    where
        <Self as UninitObserver>::Observer: for<'a> Observer<'a, Return = &'a T>,
    {
        ClonedTemplate(self)
    }

    fn dereffed<T: Deref>(self) -> DereffedTemplate<Self>
    where
        <Self as UninitObserver>::Observer: for<'a> Observer<'a, Return = &'a T>,
    {
        DereffedTemplate(self)
    }

    fn flatten(self) -> FlattenTemplate<Self>
    where
        for<'a> <<Self as UninitObserver>::Observer as Observer<'a>>::Return: UninitObserver,
    {
        FlattenTemplate(self)
    }
}

impl<T: UninitObserver> ObserverExt for T {}

#[derive(Clone)]
pub struct StaticObserver<T>(pub(crate) T);

impl<'a, T: Send + Sync + 'static> Observer<'a> for StaticObserver<T> {
    type Return = &'a T;

    fn get(&'a mut self, _: &'a bevy::prelude::World) -> (Self::Return, bool) {
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

pub trait ReturnSpec<'a, T> {
    type R: Borrow<T>;
}

pub trait IntoObserver<T, M>: Send + Sync + 'static {
    type UninitObserver: UninitObserver<Observer = Self::Observer>;
    type Observer: for<'a> Observer<'a, Return = <Self::ReturnSpec as ReturnSpec<'a, T>>::R>;
    type ReturnSpec: for<'a> ReturnSpec<'a, T>;
    fn into_observer(self) -> Self::UninitObserver;
}
pub struct RsStatic<T>(PhantomData<T>);
impl<'a, T: 'static> ReturnSpec<'a, T> for RsStatic<T> {
    type R = &'a T;
}
impl<T: Send + Sync + 'static, I: Into<T> + Send + Sync + 'static> IntoObserver<T, Static> for I {
    type UninitObserver = StaticObserver<T>;
    type Observer = StaticObserver<T>;
    type ReturnSpec = RsStatic<T>;

    fn into_observer(self) -> Self::Observer {
        StaticObserver(self.into())
    }
}

pub struct RsDynamic<O, T>(PhantomData<(O, T)>);
impl<'a, O: for<'x> Observer<'x>, T: 'static> ReturnSpec<'a, T> for RsDynamic<O, T>
where
    <O as Observer<'a>>::Return: Borrow<T>,
{
    type R = <O as Observer<'a>>::Return;
}

impl<T: 'static, O: for<'a> Observer<'a, Return = T>, UO: UninitObserver<Observer = O>>
    IntoObserver<T, Dynamic> for UO
{
    type UninitObserver = Self;
    type Observer = O;
    type ReturnSpec = RsDynamic<O, T>;

    fn into_observer(self) -> Self::UninitObserver {
        self
    }
}
