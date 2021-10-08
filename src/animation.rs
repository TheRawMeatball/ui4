use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use bevy::core::Time;
use bevy::ecs::prelude::*;
use bevy::ecs::system::SystemState;
use bevy::prelude::{Children, DespawnRecursiveExt};
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

pub(crate) type TriggerCallState = SystemState<(
    Commands<'static, 'static>,
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static bevy::ui::ControlNode>,
    Query<
        'static,
        'static,
        (
            &'static Transition,
            &'static mut TransitionProgress,
            Option<&'static ActiveTransition>,
        ),
    >,
)>;

#[derive(Component)]
pub enum Transition {
    In { duration: f32 },
    Out { duration: f32 },
    Bidirectional { duration: f32 },
    InAndOut { duration_in: f32, duration_out: f32 },
}

#[derive(Component)]
pub struct TransitionProgress {
    // 0 is out, 1 is in
    progress: f32,
    direction: Option<TransitionDirection>,
}

#[derive(Bundle)]
pub struct TransitionBundle {
    pub progress: TransitionProgress,
    pub transition: Transition,
    active: ActiveTransition,
}

impl TransitionBundle {
    pub fn bidirectional(duration: f32) -> Self {
        Self {
            progress: TransitionProgress {
                progress: 0.,
                direction: Some(TransitionDirection::In),
            },
            transition: Transition::Bidirectional { duration },
            active: ActiveTransition(None),
        }
    }
}

impl TransitionProgress {
    pub fn progress(&self) -> f32 {
        self.progress
    }
}

#[derive(Component)]
pub(crate) struct ActiveTransition(Option<Entity>);

#[derive(Component)]
pub(crate) struct BlockingTransitionCount(usize, Option<Entity>);

#[derive(Copy, Clone, PartialEq, Eq)]
enum TransitionDirection {
    In,
    Out,
}

pub(crate) fn transition_system(
    mut q: Query<(
        Entity,
        &mut TransitionProgress,
        &Transition,
        &ActiveTransition,
    )>,
    mut btc_q: Query<&mut BlockingTransitionCount>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut progress, duration, active) in q.iter_mut() {
        let duration = match (duration, progress.direction.unwrap()) {
            (
                Transition::In { duration } | Transition::Bidirectional { duration },
                TransitionDirection::In,
            ) => *duration,
            (
                Transition::Out { duration } | Transition::Bidirectional { duration },
                TransitionDirection::Out,
            ) => -duration,
            (
                Transition::InAndOut {
                    duration_in,
                    duration_out,
                },
                dir,
            ) => match dir {
                TransitionDirection::In => *duration_in,
                TransitionDirection::Out => -duration_out,
            },
            _ => continue,
        };

        progress.progress += time.delta_seconds() / duration;
        if !(0.0..1.0).contains(&progress.progress) {
            progress.direction = None;
            commands.entity(entity).remove::<ActiveTransition>();
            if let Some(cn) = active.0 {
                recursive_cn_climb(cn, &mut commands, &mut btc_q);
            }
        }
    }
}

fn recursive_cn_climb(
    cn: Entity,
    commands: &mut Commands,
    btc_q: &mut Query<&mut BlockingTransitionCount>,
) {
    let mut count = btc_q.get_mut(cn).unwrap();
    count.0 -= 1;
    if count.0 == 0 {
        if let Some(e) = count.1 {
            recursive_cn_climb(e, commands, btc_q)
        } else {
            commands.entity(cn).despawn_children_recursive();
        }
    }
}

pub(crate) fn trigger_transition_out_cn(
    e: Entity,
    parent_cn: Option<Entity>,
    commands: &mut Commands,
    children_q: &Query<&Children>,
    control_node: &Query<&bevy::ui::ControlNode>,
    transition_q: &mut Query<(
        &Transition,
        &mut TransitionProgress,
        Option<&ActiveTransition>,
    )>,
) -> bool {
    let children = children_q.get(e).map(|c| &**c).unwrap_or(&[]);

    let mut acc = 0;

    for &child in children {
        if control_node.get(child).is_ok() {
            if trigger_transition_out_cn(
                child,
                Some(e),
                commands,
                children_q,
                control_node,
                transition_q,
            ) {
                acc += 1;
            }
        } else {
            trigger_transition_out_n(
                child,
                e,
                &mut acc,
                commands,
                children_q,
                control_node,
                transition_q,
            );
        }
    }

    if acc == 0 {
        false
    } else {
        commands
            .entity(e)
            .insert(BlockingTransitionCount(acc, parent_cn));
        true
    }
}

fn trigger_transition_out_n(
    e: Entity,
    cn: Entity,
    acc: &mut usize,
    commands: &mut Commands,
    children_q: &Query<&Children>,
    control_node: &Query<&bevy::ui::ControlNode>,
    transition_q: &mut Query<(
        &Transition,
        &mut TransitionProgress,
        Option<&ActiveTransition>,
    )>,
) {
    if let Some((transition, mut progress, running)) = transition_q.get_mut(e).ok() {
        if running.is_some() {
            if progress.direction.unwrap() == TransitionDirection::In {
                match transition {
                    Transition::Out { .. }
                    | Transition::Bidirectional { .. }
                    | Transition::InAndOut { .. } => {
                        progress.direction = Some(TransitionDirection::Out);
                    }
                    _ => {
                        progress.direction = None;
                        commands.entity(e).remove::<ActiveTransition>();
                    }
                }
            }
        } else {
            commands.entity(e).insert(ActiveTransition(Some(cn)));
            progress.progress = 1.;
            progress.direction = Some(TransitionDirection::Out);
            *acc += 1;
        }
    }

    let children = children_q.get(e).map(|c| &**c).unwrap_or(&[]);

    for &child in children {
        if control_node.get(child).is_ok() {
            trigger_transition_out_cn(
                child,
                Some(cn),
                commands,
                children_q,
                control_node,
                transition_q,
            );
        } else {
            trigger_transition_out_n(
                child,
                cn,
                acc,
                commands,
                children_q,
                control_node,
                transition_q,
            );
        }
    }
}
