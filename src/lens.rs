use std::marker::PhantomData;

use bevy::prelude::{Component, Entity, World};

use crate::{
    observer::{Observer, UninitObserver},
    runtime::UpdateFunc,
};

pub trait WorldLens: Copy + Send + Sync + 'static {
    type UninitObserver: UninitObserver<Observer = Self::Observer>;

    type Observer: for<'a> Observer<'a, Return = &'a <Self::Lens as Lens>::In>;
    type Lens: Lens<Out = Self::Out>;
    type Out: 'static;

    fn get<'a>(&mut self, world: &'a World) -> &'a Self::Out;
    fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Self::Out;

    fn to_observer(self) -> (Self::UninitObserver, Self::Lens);

    fn lens<L: Lens<In = Self::Out>>(self, other: L) -> LensMerge<Self, L> {
        LensMerge(self, other)
    }
}

pub trait Lens: Copy + Send + Sync + 'static {
    type In: 'static;
    type Out: 'static;

    fn get<'a>(&self, val: &'a Self::In) -> &'a Self::Out;
    fn get_mut<'a>(&self, val: &'a mut Self::In) -> &'a mut Self::Out;

    fn lens<L: Lens<In = Self::Out>>(self, other: L) -> LensMerge<Self, L> {
        LensMerge(self, other)
    }
}

impl<W, L> WorldLens for LensMerge<W, L>
where
    W: WorldLens,
    L: Lens<In = W::Out>,
{
    type UninitObserver = W::UninitObserver;
    type Observer = W::Observer;
    type Lens = LensMerge<W::Lens, L>;
    type Out = L::Out;

    fn get<'a>(&mut self, world: &'a World) -> &'a Self::Out {
        self.1.get(self.0.get(world))
    }

    fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Self::Out {
        self.1.get_mut(self.0.get_mut(world))
    }

    fn to_observer(self) -> (Self::UninitObserver, Self::Lens) {
        let (observer, lens) = self.0.to_observer();
        (observer, LensMerge(lens, self.1))
    }
}

#[derive(Copy, Clone)]
pub struct LensMerge<A, B>(A, B);

impl<A, B> Lens for LensMerge<A, B>
where
    A: Lens,
    B: Lens<In = A::Out>,
{
    type In = A::In;
    type Out = B::Out;

    fn get<'a>(&self, val: &'a Self::In) -> &'a Self::Out {
        self.1.get(self.0.get(val))
    }

    fn get_mut<'a>(&self, val: &'a mut Self::In) -> &'a mut Self::Out {
        self.1.get_mut(self.0.get_mut(val))
    }
}

pub struct LensObserver<L: WorldLens>(L::Lens, <L::UninitObserver as UninitObserver>::Observer);

impl<'a, L: WorldLens> Observer<'a> for LensObserver<L> {
    type Return = &'a L::Out;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let (val, changed) = self.1.get(world);
        (self.0.get(val), changed)
    }
}

impl<L: WorldLens> UninitObserver for L {
    type Observer = LensObserver<L>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let (observer, lens) = self.to_observer();
        observer.register_self(world, |obs, world| uf(LensObserver(lens, obs), world))
    }
}

pub struct Identity<T>(pub(crate) PhantomData<fn(T) -> T>);
impl<T> Copy for Identity<T> {}
impl<T> Clone for Identity<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> Lens for Identity<T> {
    type In = T;
    type Out = T;

    fn get<'a>(&self, val: &'a Self::In) -> &'a Self::Out {
        val
    }

    fn get_mut<'a>(&self, val: &'a mut Self::In) -> &'a mut Self::Out {
        val
    }
}

pub struct ComponentLens<T: Component>(pub(crate) Entity, pub(crate) PhantomData<T>);
impl<T: Component> Copy for ComponentLens<T> {}
impl<T: Component> Clone for ComponentLens<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T: Component> WorldLens for ComponentLens<T> {
    type UninitObserver = crate::observer::ComponentObserver<T>;
    type Observer = crate::observer::ComponentObserver<T>;
    type Lens = Identity<T>;
    type Out = T;

    fn get<'a>(&mut self, world: &'a World) -> &'a Self::Out {
        world.get(self.0).unwrap()
    }

    fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Self::Out {
        world.get_mut::<T>(self.0).unwrap().into_inner()
    }

    fn to_observer(self) -> (Self::UninitObserver, Self::Lens) {
        (
            crate::observer::ComponentObserver {
                entity: self.0,
                _marker: PhantomData,
            },
            Identity(PhantomData),
        )
    }
}
