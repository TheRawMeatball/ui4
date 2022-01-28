use std::marker::PhantomData;

use bevy::ecs::{
    prelude::*,
    system::{SystemParam, SystemState},
};

use crate::{
    childable::Childable,
    dom::ControlBundle,
    insertable::Insertable,
    lens::ComponentLens,
    observer::{ComponentExistsObserver, Observer, OptComponentObserver, UninitObserver},
    runtime::{UfMarker, UiScratchSpace, UpdateFunc},
};

/// Entry point for creating entities. Having a `Ctx` means you control all components on this entity, and
/// can add children
pub struct Ctx<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) current_entity: Entity,
}

/// Stands for "Multi Child Context". Represents being able to add children to a pre-existing entity, but modifying said
/// entity isn't possible.
pub struct McCtx<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) get_new_child: &'a mut dyn FnMut(&mut World) -> Entity,
}

impl McCtx<'_> {
    /// Add a child to the entity.
    // TODO: bikeshed name
    pub fn c(&mut self, f: impl FnOnce(Ctx) -> Ctx) -> &mut Self {
        let new_child = (self.get_new_child)(self.world);
        f(Ctx {
            current_entity: new_child,
            world: self.world,
        });
        self
    }

    pub fn dyn_group<M>(&mut self, children: impl Childable<M>) -> &mut Self {
        self.c(|ctx: Ctx| ctx.with_bundle(ControlBundle::default()).children(children))
    }
}

impl Ctx<'_> {
    /// Statically inserts a component, or sets up reactive-ness if given a reactive template.
    ///
    /// See the documentation on [`Insertable`] for details.
    pub fn with<T: Component, M>(mut self, item: impl Insertable<T, M>) -> Self {
        item.insert_ui_val(&mut self);
        self
    }

    /// Inherit the configuration of a separate widget
    pub fn inherit(self, widget: impl FnOnce(Ctx) -> Ctx) -> Self {
        widget(self)
    }

    pub fn state<P: SystemParam>(&mut self) -> SystemState<P> {
        SystemState::new(self.world)
    }

    pub fn with_modified<T, O, F>(self, initial: T, observer: O, mutator: F) -> Self
    where
        T: Component,
        O: UninitObserver,
        for<'a> O::Observer: Observer<'a>,
        F: for<'a> Fn(<O::Observer as Observer<'a>>::Return, T) -> T,
        F: Send + Sync + 'static,
    {
        self.world.entity_mut(self.current_entity).insert(initial);
        let entity = self.current_entity;
        let uf = observer.register_self(self.world, |mut observer, world| {
            let mut first = true;
            let (uf, marker) = UpdateFunc::new::<T, _>(move |world| {
                world.resource_scope(|world, mut ctx: Mut<UiScratchSpace>| {
                    let t = world.entity_mut(entity).remove::<T>().unwrap();
                    let (val, changed) = observer.get(world);
                    if !changed && !first {
                        drop(val);
                        let mut e = world.entity_mut(entity);
                        e.insert(t);
                        return;
                    }
                    first = false;
                    let t = mutator(val, t);
                    let mut e = world.entity_mut(entity);
                    e.get_mut::<UfMarker<T>>().unwrap().trigger(&mut ctx);
                    e.insert(t);
                })
            });
            world.entity_mut(entity).insert(marker);
            uf
        });
        uf.run(self.world);
        self
    }

    /// Inserts a bundle *statically*. If you need certain parts of this bundle to be reactive,
    /// use [`Ctx::with`] for those particular components instead.
    pub fn with_bundle(self, bundle: impl Bundle) -> Self {
        self.world
            .entity_mut(self.current_entity)
            .insert_bundle(bundle);
        self
    }

    pub fn child(self, f: impl FnOnce(Ctx) -> Ctx) -> Self {
        self.children(|ctx: &mut McCtx| {
            ctx.c(f);
        })
    }

    pub fn children<M>(mut self, children: impl Childable<M>) -> Self {
        children.insert(&mut self);
        self
    }

    /// Gets an lens for a component on the entity being built. Panics if the component is removed
    /// and the lens is still in use.
    pub fn component<T: Component>(&self) -> ComponentLens<T> {
        ComponentLens(self.current_entity, PhantomData)
    }

    /// Gets an observer for a component on the entity being built.
    pub fn opt_component<T: Component>(&self) -> OptComponentObserver<T> {
        OptComponentObserver(self.current_entity, PhantomData)
    }

    /// Gets an observer for whether the current entity has a particular component. Most useful with marker
    /// components.
    pub fn has_component<T: Send + Sync + 'static>(&self) -> ComponentExistsObserver<T> {
        ComponentExistsObserver(self.current_entity, PhantomData)
    }

    #[inline]
    pub fn current_entity(&self) -> Entity {
        self.current_entity
    }
}

pub trait WidgetBuilderExtWith<T, M, I>
where
    T: Component,
    I: Insertable<T, M>,
{
    type WithOut: FnOnce(Ctx) -> Ctx;

    /// Statically inserts a component, or sets up reactive-ness if given a reactive template.
    ///
    /// See the documentation on [`Insertable`] for details.
    fn with(self, item: I) -> Self::WithOut;
}

pub trait WidgetBuilderExtWithModified<T, O, F>
where
    T: Component,
    O: UninitObserver,
    for<'a> O::Observer: Observer<'a>,
    F: for<'a> Fn(<O::Observer as Observer<'a>>::Return, T) -> T,
    F: Send + Sync + 'static,
{
    type WithModifiedOut: FnOnce(Ctx) -> Ctx;
    fn with_modified(self, initial: T, observer: O, mutator: F) -> Self::WithModifiedOut;
}

#[cfg(feature = "nightly")]
mod nightly_impls {
    use super::*;

    impl<W: 'static, T, M, I> WidgetBuilderExtWith<T, M, I> for W
    where
        W: FnOnce(Ctx) -> Ctx,
        T: Component,
        I: Insertable<T, M>,
    {
        type WithOut = impl FnOnce(Ctx) -> Ctx;

        fn with(self, item: I) -> Self::WithOut {
            |ctx| (self)(ctx).with(item)
        }
    }

    impl<W: 'static, T, O, F> WidgetBuilderExtWithModified<T, O, F> for W
    where
        W: FnOnce(Ctx) -> Ctx,
        T: Component,
        O: UninitObserver,
        for<'a> O::Observer: Observer<'a>,
        F: for<'a> Fn(<O::Observer as Observer<'a>>::Return, T) -> T,
        F: Send + Sync + 'static,
    {
        type WithModifiedOut = impl FnOnce(Ctx) -> Ctx;

        fn with_modified(self, initial: T, observer: O, mutator: F) -> Self::WithModifiedOut {
            |ctx| (self)(ctx).with_modified(initial, observer, mutator)
        }
    }
}

#[cfg(not(feature = "nightly"))]
mod stable_impls {
    use super::*;

    impl<W: 'static, T, M, I> WidgetBuilderExtWith<T, M, I> for W
    where
        W: FnOnce(Ctx) -> Ctx,
        T: Component,
        I: Insertable<T, M>,
    {
        type WithOut = Box<dyn FnOnce(Ctx) -> Ctx>;

        fn with(self, item: I) -> Self::WithOut {
            Box::new(|ctx| (self)(ctx).with(item))
        }
    }

    impl<W: 'static, T, O, F> WidgetBuilderExtWithModified<T, O, F> for W
    where
        W: FnOnce(Ctx) -> Ctx,
        T: Component,
        O: UninitObserver,
        for<'a> O::Observer: Observer<'a>,
        F: for<'a> Fn(<O::Observer as Observer<'a>>::Return, T) -> T,
        F: Send + Sync + 'static,
    {
        type WithModifiedOut = Box<dyn FnOnce(Ctx) -> Ctx>;

        fn with_modified(self, initial: T, observer: O, mutator: F) -> Self::WithModifiedOut {
            Box::new(|ctx| (self)(ctx).with_modified(initial, observer, mutator))
        }
    }
}
