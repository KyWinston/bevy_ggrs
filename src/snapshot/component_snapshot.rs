use crate::{
    GgrsComponentSnapshot, GgrsComponentSnapshots, LoadWorld, LoadWorldSet, Rollback,
    RollbackFrameCount, SaveWorld, SaveWorldSet, Strategy,
};
use bevy::prelude::*;
use std::marker::PhantomData;

/// A [`Plugin`] which manages snapshots for a [`Component`] using a provided [`Strategy`].
///
/// # Examples
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_ggrs::{prelude::*, ComponentSnapshotPlugin, CloneStrategy};
/// #
/// # const FPS: usize = 60;
/// #
/// # type MyInputType = u8;
/// #
/// # fn read_local_inputs() {}
/// #
/// # fn start(session: Session<GgrsConfig<MyInputType>>) {
/// # let mut app = App::new();
/// // The Transform component is a good candidate for Clone-based rollback
/// app.add_plugins(ComponentSnapshotPlugin::<CloneStrategy<Transform>>::default());
/// # }
/// ```
pub struct ComponentSnapshotPlugin<S>
where
    S: Strategy,
    S::Target: Component,
    S::Stored: Send + Sync + 'static,
{
    _phantom: PhantomData<S>,
}

impl<S> Default for ComponentSnapshotPlugin<S>
where
    S: Strategy,
    S::Target: Component,
    S::Stored: Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            _phantom: default(),
        }
    }
}

impl<S> ComponentSnapshotPlugin<S>
where
    S: Strategy,
    S::Target: Component,
    S::Stored: Send + Sync + 'static,
{
    pub fn save(
        mut snapshots: ResMut<GgrsComponentSnapshots<S::Target, S::Stored>>,
        frame: Res<RollbackFrameCount>,
        query: Query<(&Rollback, &S::Target)>,
    ) {
        let components = query
            .iter()
            .map(|(&rollback, component)| (rollback, S::store(component)));

        let snapshot = GgrsComponentSnapshot::new(components);

        trace!(
            "Snapshot {} {} component(s)",
            snapshot.iter().count(),
            bevy::utils::get_short_name(std::any::type_name::<S::Target>())
        );

        snapshots.push(frame.0, snapshot);
    }

    pub fn load(
        mut commands: Commands,
        mut snapshots: ResMut<GgrsComponentSnapshots<S::Target, S::Stored>>,
        frame: Res<RollbackFrameCount>,
        mut query: Query<(Entity, &Rollback, Option<&mut S::Target>)>,
    ) {
        let snapshot = snapshots.rollback(frame.0).get();

        for (entity, rollback, component) in query.iter_mut() {
            let snapshot = snapshot.get(rollback);

            match (component, snapshot) {
                (Some(mut component), Some(snapshot)) => S::update(component.as_mut(), snapshot),
                (Some(_), None) => {
                    commands.entity(entity).remove::<S::Target>();
                }
                (None, Some(snapshot)) => {
                    commands.entity(entity).insert(S::load(snapshot));
                }
                (None, None) => {}
            }
        }

        trace!(
            "Rolled back {} {} component(s)",
            snapshot.iter().count(),
            bevy::utils::get_short_name(std::any::type_name::<S::Target>())
        );
    }
}

impl<S> Plugin for ComponentSnapshotPlugin<S>
where
    S: Send + Sync + 'static + Strategy,
    S::Target: Component,
    S::Stored: Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<GgrsComponentSnapshots<S::Target, S::Stored>>()
            .add_systems(
                SaveWorld,
                (
                    GgrsComponentSnapshots::<S::Target, S::Stored>::discard_old_snapshots,
                    Self::save,
                )
                    .chain()
                    .in_set(SaveWorldSet::Snapshot),
            )
            .add_systems(LoadWorld, Self::load.in_set(LoadWorldSet::Data));
    }
}
