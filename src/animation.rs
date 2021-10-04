use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use bevy::core::Time;
use bevy::ecs::prelude::*;
use slotmap::{DefaultKey, SlotMap};

use crate::observer::{Observer, UninitObserver};
use crate::runtime::{UiScratchSpace, UpdateFunc};

struct ActiveTween {
    duration: f32,
    time_left: f32,
    start: f32,
    end: f32,
    arc: Arc<AtomicU32>,
    uf: UpdateFunc,
}

#[derive(Default)]
pub(crate) struct RunningTweens {
    arena: SlotMap<DefaultKey, ActiveTween>,
}

pub struct UninitTweenObserver<UO> {
    observer: UO,
    settings: TweenSettings,
}

struct TweenSettings {
    duration: f32,
    // interpolation_type: ?
}

pub struct TweenObserver {
    current_val: Arc<AtomicU32>,
}

pub trait TweenExt: Sized {
    fn tween(self, duration: f32) -> UninitTweenObserver<Self> {
        UninitTweenObserver {
            observer: self,
            settings: TweenSettings { duration },
        }
    }
}

#[rustfmt::skip]
impl<UO, O> TweenExt for UO
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = f32>,
{
}

impl TweenObserver {
    fn new() -> (Self, Arc<AtomicU32>) {
        let arc = Arc::<AtomicU32>::default();
        (
            Self {
                current_val: arc.clone(),
            },
            arc,
        )
    }
}

#[rustfmt::skip]
impl<UO, O> UninitObserver for UninitTweenObserver<UO>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = f32>,
{
    type Observer = TweenObserver;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = self.observer.register_self(world, |mut observer, world| {
            let (obs, arc) = TweenObserver::new();
            let uf = uf(obs, world);
            let ufm = Arc::new(Mutex::new(None));
            let ufmc = ufm.clone();
            let mut first = true;
            let mut current = None;
            let (uf, marker) = UpdateFunc::new::<(), _>(move |world| {
                if uf.flagged() {
                    ufmc.lock().unwrap().take();
                    return;
                }
                let (val, changed) = observer.get(world);
                if !changed && !first {
                    return;
                }
                first = false;
                let old = f32::from_bits(arc.load( std::sync::atomic::Ordering::SeqCst));
                arc.store(f32::to_bits(val), std::sync::atomic::Ordering::SeqCst);
                let running_tweens = world.get_resource_mut::<RunningTweens>().unwrap().into_inner();
                if let Some(ct) = current {
                    if let Some(current) = running_tweens.arena.get_mut(ct) {
                        let intp = current.time_left / current.duration;
                        current.start = current.end + (current.start - current.end) * intp.clamp(0., 1.);
                        current.end = val;
                        current.time_left = current.duration;
                        return;
                    } 
                }
                current = Some(running_tweens.arena.insert(ActiveTween {
                    duration: self.settings.duration,
                    time_left: self.settings.duration,
                    start: old,
                    end: val,
                    arc: arc.clone(),
                    uf: uf.clone(),
                }));
            });
            *ufm.lock().unwrap() = Some(marker);
            uf
        });

        uf
    }
}

impl Observer for TweenObserver {
    type Return<'w, 's> = f32;

    fn get<'w, 's>(&'s mut self, _: &'w World) -> (Self::Return<'w, 's>, bool) {
        let val = self.current_val.load(std::sync::atomic::Ordering::SeqCst);
        (f32::from_bits(val), true)
    }
}

pub(crate) fn tween_system(
    time: Res<Time>,
    mut tweens: ResMut<RunningTweens>,
    mut ufs: ResMut<UiScratchSpace>,
) {
    tweens.arena.retain(|_, tween| {
        tween.time_left -= time.delta_seconds();
        let intp = tween.time_left / tween.duration;
        let val = tween.end + (tween.start - tween.end) * intp.clamp(0., 1.);
        tween
            .arc
            .store(f32::to_bits(val), std::sync::atomic::Ordering::SeqCst);
        ufs.register_update_func(tween.uf.clone());
        tween.time_left >= 0.
    })
}
