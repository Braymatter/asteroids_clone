use bevy::{platform::collections::HashMap, prelude::*};

pub fn physics_plugin(app: &mut App) {
    app.add_message::<CollisionEvent>();

    app.add_systems(Update, (apply_velocity, detect_collisions));
}

#[derive(Component)]
pub struct Velocity {
    pub linear: Vec2,
    pub linear_drag: Vec2,

    pub angular: f32,
    pub angular_drag: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear_drag: Vec2::splat(0.5),
            angular_drag: 0.5,
            linear: Vec2::ZERO,
            angular: 0.0,
        }
    }
}

#[derive(Component)]
pub struct CircleCollider {
    pub radius: f32,
}

impl Default for CircleCollider {
    fn default() -> Self {
        Self { radius: 1.0 }
    }
}

#[derive(Message)]
pub struct CollisionEvent(pub Entity, pub Entity);

pub fn detect_collisions(
    physical: Query<(&Transform, &CircleCollider, Entity)>,
    mut events: MessageWriter<CollisionEvent>,
) {
    let mut collisions: HashMap<Entity, Vec<Entity>> = HashMap::new();

    for (tsf, collider, entity) in physical.iter() {
        if !collisions.contains_key(&entity) {
            collisions.insert(entity, vec![]);
        }

        for (tsf_b, _collider_b, ent_b) in physical.iter() {
            //Don't collide with self
            if entity == ent_b {
                continue;
            }

            if tsf.translation.distance(tsf_b.translation) < collider.radius {
                if let Some(collisions_entb) = collisions.get(&ent_b)
                    && collisions_entb.contains(&entity)
                {
                    continue;
                }

                collisions.get_mut(&entity).unwrap().push(ent_b)
            }
        }
    }

    let mut events_to_send = vec![];
    for (ent, collided_with) in collisions.iter() {
        collided_with.iter().for_each(|entb| {
            events_to_send.push(CollisionEvent(*ent, *entb));
        });
    }

    events.write_batch(events_to_send);
}

pub fn apply_velocity(mut movers: Query<(&mut Transform, &mut Velocity)>, time: Res<Time>) {
    for (mut tsf, mut vel) in movers.iter_mut() {
        let vel_drag = vel.linear_drag;
        vel.linear *= 1.0 - (vel_drag * time.delta_secs());
        let ang_drag = vel.angular_drag;
        vel.angular *= 1.0 - (ang_drag * time.delta_secs());

        tsf.translation += Vec3::new(vel.linear.x, vel.linear.y, 0.0) * time.delta_secs();
        tsf.rotate_z(vel.angular * time.delta_secs());
    }
}
