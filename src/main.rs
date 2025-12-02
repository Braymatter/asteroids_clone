use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use bevy::{prelude::*, time::Stopwatch};
use rand::Rng;

use crate::physics::{CircleCollider, CollisionEvent, Velocity, physics_plugin};

mod physics;

fn main() {
    info!("Starting Bevy App");

    let mut app = App::new();
    app.add_plugins(physics_plugin);

    app.add_plugins(DefaultPlugins);

    app.init_resource::<GameStats>();

    app.add_systems(Startup, (load_assets, setup_scene).chain());

    app.add_systems(Update, (game_tick, control_ship, handle_collisions));

    app.run();
}

#[derive(Resource)]
pub struct GameStats {
    pub score: u32,
    pub stopwatch: Stopwatch,
    pub roid_timer: Timer,
    pub roid_chance: i8,
}

impl Default for GameStats {
    fn default() -> Self {
        Self {
            score: Default::default(),
            stopwatch: Default::default(),
            roid_timer: Timer::new(Duration::from_millis(500), TimerMode::Repeating),
            roid_chance: 10,
        }
    }
}

#[derive(Resource, Default)]
pub struct GameAssets {
    pub meteors: Vec<Handle<Image>>,
    pub ship: Handle<Image>,
    pub laser: Handle<Image>,
}

pub fn load_assets(asset_server: Res<AssetServer>, mut cmds: Commands) {
    let assets = GameAssets {
        ship: asset_server.load("kenney-space/PNG/playerShip1_orange.png"),
        laser: asset_server.load("kenney-space/PNG/Lasers/laserRed08.png"),
        meteors: vec![
            asset_server.load("kenney-space/PNG/Meteors/meteorGrey_big1.png"),
            asset_server.load("kenney-space/PNG/Meteors/meteorGrey_big2.png"),
            asset_server.load("kenney-space/PNG/Meteors/meteorGrey_big3.png"),
            asset_server.load("kenney-space/PNG/Meteors/meteorGrey_big4.png"),
        ],
    };

    cmds.insert_resource(assets);
}

/// Sets up the game scene
/// - Spawns the player
/// - Spawns 10 asteroids
/// - Spawns a camera
pub fn setup_scene(mut cmds: Commands, assets: Res<GameAssets>) {
    //Spawns a NEW entity with the specified components / bundle
    cmds.spawn((Camera2d, GameCleanup));

    cmds.spawn((
        Velocity::default(),
        GameCleanup,
        PlayerShip::default(),
        Sprite::from_image(assets.ship.clone()),
        CircleCollider { radius: 50.0 },
    ));
}

pub fn game_tick(time: Res<Time>, mut cmds: Commands, mut game_stats: ResMut<GameStats>) {
    game_stats.roid_timer.tick(time.delta());
    game_stats.stopwatch.tick(time.delta());

    let mut rand = rand::rng();

    if game_stats.roid_timer.just_finished() {
        let val = rand.random_range(0..100);

        if val < game_stats.roid_chance {
            //Generate random position and velocity
            let pos = Vec2::new(
                rand.random_range(-550.0..55.0),
                rand.random_range(-550.0..55.0),
            );
            let rotation = rand.random_range(-PI..PI);
            let speed = rand.random_range(-200.0..200.0);
            let angvel = rand.random_range(-PI..PI);
            cmds.run_system_cached_with(spawn_asteroid, (pos, rotation, speed, angvel));
        }
    }
}

pub fn control_ship(
    ship: Single<(&mut PlayerShip, &mut Velocity, &Transform)>,
    btn_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cmds: Commands,
) {
    let (ship, mut ship_vel, ship_tsf) = ship.into_inner();

    let forward_key = KeyCode::KeyW;
    let rotate_right = KeyCode::KeyD;
    let rotate_left = KeyCode::KeyA;
    #[cfg(feature = "mac-dev")]
    {
        let rotate_right = KeyCode::KeyS;
    }
    let euler_rot = ship_tsf.rotation.to_euler(EulerRot::XYZ).2;
    if btn_input.pressed(forward_key) {
        let new_vel =
            Vec2::new(-euler_rot.sin(), euler_rot.cos()) * ship.linear_accel * time.delta_secs();
        ship_vel.linear += new_vel;
    }

    if btn_input.pressed(rotate_right) {
        ship_vel.angular -= time.delta_secs() * ship.angular_accel;
    }

    if btn_input.pressed(rotate_left) {
        ship_vel.angular += time.delta_secs() * ship.angular_accel;
    }

    if btn_input.just_pressed(KeyCode::Space) {
        cmds.run_system_cached_with(
            spawn_laser_shot,
            (ship_tsf.translation.xy(), euler_rot, ship_vel.linear),
        );
    }
}

#[derive(Component)]
pub struct PlayerShip {
    /// How many shots per second
    pub fire_rate: f32,
    pub last_fired: Instant,

    // Movement limitations
    pub linear_accel: f32,
    pub angular_accel: f32,
}

impl Default for PlayerShip {
    fn default() -> Self {
        Self {
            fire_rate: 0.5,
            last_fired: Instant::now(),
            linear_accel: 50.0,
            angular_accel: 2.0 * PI,
        }
    }
}

#[derive(Component)]
pub struct Asteroid;

pub fn handle_collisions(
    mut collisions: MessageReader<CollisionEvent>,
    lasers: Query<Entity, With<LaserShot>>,
    asteroids: Query<Entity, With<Asteroid>>,
    ship: Single<Entity, With<PlayerShip>>,
    ents: Query<Entity, With<GameCleanup>>,
    mut cmds: Commands,
    mut game_stats: ResMut<GameStats>,
) {
    for collision in collisions.read() {
        let mut destroyed_roid = false;
        if let Ok(laser) = lasers.get(collision.0)
            && let Ok(asteroid) = asteroids.get(collision.1)
        {
            cmds.entity(laser).try_despawn();
            cmds.entity(asteroid).try_despawn();
            destroyed_roid = true;
        }

        //Check the other way now
        if let Ok(laser) = lasers.get(collision.1)
            && let Ok(asteroid) = asteroids.get(collision.0)
        {
            cmds.entity(laser).try_despawn();
            cmds.entity(asteroid).try_despawn();
            destroyed_roid = true;
        }

        if destroyed_roid {
            game_stats.score += 10;
            info!("Score: {}", game_stats.score);
            continue;
        }

        //Check if player ship collided with asteroid
        if (collision.0 == *ship || collision.1 == *ship)
            && (asteroids.contains(collision.1) || asteroids.contains(collision.0))
        {
            for ent in ents {
                cmds.entity(ent).try_despawn();
            }

            cmds.run_system_cached(setup_scene);
        }
    }
}

#[derive(Component)]
pub struct GameCleanup;

#[derive(Component)]
pub struct LaserShot;

pub fn spawn_laser_shot(
    In((loc, forward, init_vel)): In<(Vec2, f32, Vec2)>,
    mut cmds: Commands,
    game_assets: Res<GameAssets>,
) {
    info!("Shooting");

    //Set pos and rot
    let mut tsf = Transform::from_xyz(loc.x, loc.y, 0.0);
    tsf.rotate_z(forward);

    let euler_rot = tsf.rotation.to_euler(EulerRot::XYZ).2;

    let velocity = Vec2::new(-euler_rot.sin(), euler_rot.cos()) * 400.0;

    let velocity = Velocity {
        linear: velocity + init_vel,
        linear_drag: Vec2::ZERO,
        angular: 0.0,
        angular_drag: 0.0,
    };

    let mut laser_sprite = Sprite::from_image(game_assets.laser.clone());
    let size = 15.0;
    laser_sprite.custom_size = Some(Vec2::splat(size));

    cmds.spawn((
        LaserShot,
        GameCleanup,
        velocity,
        tsf,
        CircleCollider { radius: size },
        laser_sprite,
    ));
}

pub fn spawn_asteroid(
    In((location, heading, speed, angvel)): In<(Vec2, f32, f32, f32)>,
    assets: Res<GameAssets>,
    mut cmds: Commands,
) {
    let mut rng = rand::rng();
    let asteroid_variant = rng.random_range(0..3);

    let mut tsf = Transform::from_xyz(location.x, location.y, 0.0);

    tsf.rotate_z(heading);

    let euler_rot = tsf.rotation.to_euler(EulerRot::XYZ).2;
    let velocity = Vec2::new(-euler_rot.sin(), euler_rot.cos()) * speed;

    cmds.spawn((
        Sprite::from_image(assets.meteors[asteroid_variant].clone()),
        Asteroid,
        Velocity {
            linear: velocity,
            linear_drag: Vec2::ZERO,
            angular: angvel,
            angular_drag: 0.0,
        },
        GameCleanup,
        CircleCollider { radius: 50.0 },
        tsf,
    ));
}
