use bevy::input::mouse::MouseMotion;
use bevy::mesh::CuboidMeshBuilder;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future::YieldNow;
use bevy::window::PrimaryWindow;

// Simple item kinds for inventory/collecting/using
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ItemKind {
    Coin,
    Block,
}

impl Default for ItemKind {
    fn default() -> Self { ItemKind::Block }
}

// Inventory resource
#[derive(Resource, Default)]
struct Inventory {
    coins: u32,
    blocks: u32,
    selected: ItemKind,
}
#[derive(Component)]
struct Ground{
    x: f32,
    y: f32,
    z: f32,
    width: f32,
    depth: f32,
}

#[derive(Component)]
struct Character{
    velocity: Vec3,
    grounded: bool,
    // allow one extra jump in-air for simple parkour
    jumps_left: u8,
    x: f32,
    z: f32,
}

#[derive(Component)]
struct CameraController {
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub locked: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Inventory>()
        .add_systems(Startup, (setup, setup_character))
        .add_systems(
            Update,
            (
                move_cam,
                mouse_look,
                character_physics,
                pickup_nearby,
                use_selected_item,
                inventory_hotkeys,
            ),
        )
        .run();
}

fn character_physics(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Character)>,
    ground: Query<&Ground>
){
    let gravity = -9.81;
    let jump_speed = 5.0;

    for (mut transform, mut character) in query.iter_mut(){
        let want_jump = keyboard_input.just_pressed(KeyCode::Space);
        if want_jump {
            if character.grounded {
                character.velocity.y = jump_speed;
                character.grounded = false;
                character.jumps_left = 1; // reset extra jump on initial takeoff
            } else if character.jumps_left > 0 {
                character.velocity.y = jump_speed;
                character.jumps_left -= 1; // consume double jump
            } else {
                // no jumps available; apply gravity
                character.velocity.y += gravity * time.delta_secs();
            }
        } else {
            character.velocity.y += gravity * time.delta_secs();
        }
        let last_pos = transform.translation.y;
        transform.translation += character.velocity * time.delta_secs();
        if character.velocity.y <= 0.0 {
            for ground in ground.iter() {
                let ground_min_x = ground.x - ground.width;
                let ground_max_x = ground.x + ground.width;
                let ground_min_z = ground.z - ground.depth;
                let ground_max_z = ground.z + ground.depth;

                if transform.translation.y <= ground.y
                    && character.x >= ground_min_x
                    && character.x <= ground_max_x
                    && character.z >= ground_min_z
                    && character.z <= ground_max_z
                    && last_pos >= ground.y
                    && transform.translation.y <= ground.y
                {
                    transform.translation.y = ground.y;
                    character.velocity.y = 0.0;
                    character.grounded = true;
                    character.jumps_left = 1;
                }
            }
        }


    }
}

fn setup_character(mut commands: Commands,
mut meshes: ResMut<Assets<Mesh>>,
mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Mesh3d(meshes.add(CuboidMeshBuilder::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8,0.2,0.2))),
        Transform::from_translation(Vec3::new(0.0, 1.2, 0.0)),
        Character{
            velocity: Vec3::ZERO,
            grounded: true,
            jumps_left: 1,
            x: 0.0,
            z: 0.0,
        },
    )).with_children(|parent| {
        parent.spawn((
            Camera3d::default(),
        CameraController{
            sensitivity: 0.0002,
            pitch: 0.0,
            yaw: 0.0,
            locked: true,
        }));
    });
}

fn mouse_look(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
    if let Some(mut window) = windows.iter_mut().next() {
        let toggle = keyboard_input.just_pressed(KeyCode::Escape);

        let mut any_locked = false;
        for (_, mut c) in query.iter_mut() {
            if toggle {
                c.locked = !c.locked;
            }
            any_locked |= c.locked;
        }

        if any_locked {
            let center = vec2(window.width() * 0.5, window.height() * 0.5);
            window.set_cursor_position(Some(center));
        }
    }
    for (mut transform, mut controller) in query.iter_mut() {
        for motion in mouse_motion.read() {
            controller.yaw -= motion.delta.x * controller.sensitivity;
            controller.pitch -= motion.delta.y * controller.sensitivity;
            controller.pitch = controller.pitch.clamp(-1.54, 1.54);

            transform.rotation = 
                Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);

        }
    }
}

fn move_cam(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut char_query: Query<(&mut Transform, &mut Character), With<Character>>,
    cam_query: Query<&GlobalTransform, With<Camera3d>>,
    time: Res<Time>,
) {
    let (mut cam_forward, mut cam_right) = (Vec3::Z, Vec3::X);

    if let Some(cam_tf) = cam_query.iter().next() {
        cam_forward = cam_tf.forward().as_vec3();
        cam_forward.y = 0.0;
        if cam_forward.length_squared() > 0.0 {
            cam_forward = cam_forward.normalize();
        }
        cam_right = cam_tf.right().as_vec3();
        cam_right.y = 0.0;
        if cam_right.length_squared() > 0.0 { cam_right = cam_right.normalize(); }
        let mut speed = 5.0;
        if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
            speed *= 1.8; // sprint for parkour
        }
        for (mut transform, mut character) in char_query.iter_mut() {
            let mut dir = Vec3::ZERO;
            if keyboard_input.pressed(KeyCode::KeyW) {
                dir += cam_forward;
            }if keyboard_input.pressed(KeyCode::KeyS) {
                dir -= cam_forward;
            }if keyboard_input.pressed(KeyCode::KeyA) {
                dir -= cam_right;
            }if keyboard_input.pressed(KeyCode::KeyD) {
                dir += cam_right;
            }
                if dir != Vec3::ZERO { transform.translation += dir.normalize() * speed * time.delta_secs();
                    character.x = transform.translation.x;
                    character.z = transform.translation.z;
                }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ana zemin - y: 0.0 olmalı (zemin seviyesi)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::new(0.0, 1.0, 0.0), vec2(10.0, 10.0)))),
        MeshMaterial3d(materials.add(Color::srgb(0.3,0.5,0.3))),
        Transform::from_translation(Vec3::ZERO),
        Ground{ x: 0.0, z: 0.0, y: 1.0, width: 10.0, depth: 10.0 },
    ));

    commands.spawn((
        PointLight {
            intensity: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 3.0, 0.0),

    ));

    // Simple parkour blocks/platforms
    let platform_color = materials.add(Color::srgb(0.4, 0.4, 0.5));
    let platform_mesh = meshes.add(CuboidMeshBuilder::default());
    let heights = [0.5, 1.0, 1.5, 2.0];
    let xs = [-2.0, -0.5, 1.0, 2.5];
    // Parkour platformları - y değeri platformun üst yüzeyi olmalı
    for (i, h) in heights.iter().enumerate() {
         // scale.y = 0.2 -> yarısı 0.1, üst yüzey = h + 0.1
        commands.spawn((
            Mesh3d(platform_mesh.clone()),
            MeshMaterial3d(platform_color.clone()),
            Transform { translation: Vec3::new(xs[i], *h, -2.0), scale: Vec3::new(1.0, 0.2, 1.0), ..Default::default() },
            Ground{ x: xs[i], z: -2.0, y: *h+0.5, width: 1.0, depth: 1.0 }, // width/depth yarım birim (scale 1.0 için)
        ));
    }

    // Spawn some collectibles
    spawn_collectible(&mut commands, &mut meshes, &mut materials, ItemKind::Coin, Vec3::new(2.0, 0.5, 2.0));
    spawn_collectible(&mut commands, &mut meshes, &mut materials, ItemKind::Coin, Vec3::new(-2.0, 0.5, 1.0));
    spawn_collectible(&mut commands, &mut meshes, &mut materials, ItemKind::Block, Vec3::new(1.0, 0.5, -1.5));
}


// ---------- Collecting/Inventory/Using ----------

#[derive(Component)]
struct Collectible(ItemKind);

fn spawn_collectible(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    kind: ItemKind,
    pos: Vec3,
) {
    let color = match kind {
        ItemKind::Coin => Color::srgb(1.0, 0.85, 0.0),
        ItemKind::Block => Color::srgb(0.2, 0.6, 1.0),
    };
    let mesh = meshes.add(CuboidMeshBuilder::default());
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(color)),
        Transform { translation: pos, scale: Vec3::splat(0.3), ..Default::default() },
        Collectible(kind),
    ));
}

fn pickup_nearby(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
    char_query: Query<&Transform, With<Character>>,
    collectibles: Query<(Entity, &Transform, &Collectible)>,
) {
    if !keyboard_input.just_pressed(KeyCode::KeyE) { return; }
    let Some(char_tf) = char_query.iter().next() else { return; };
    let mut best: Option<(Entity, ItemKind, f32)> = None;
    for (e, tf, c) in collectibles.iter() {
        let d = char_tf.translation.distance(tf.translation);
        if d < 1.2 { // pickup radius
            if best.map(|(_,_,bd)| d < bd).unwrap_or(true) {
                best = Some((e, c.0, d));
            }
        }
    }
    if let Some((e, kind, _)) = best {
        match kind {
            ItemKind::Coin => inventory.coins += 1,
            ItemKind::Block => inventory.blocks += 1,
        }
        commands.entity(e).despawn();
        println!("Picked up {:?}. Inventory -> Coins: {}, Blocks: {}", kind, inventory.coins, inventory.blocks);
    }
}

fn inventory_hotkeys(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        inventory.selected = ItemKind::Block;
        println!("Selected: Block ({} available)", inventory.blocks);
    }
    if keyboard_input.just_pressed(KeyCode::Digit2) {
        inventory.selected = ItemKind::Coin;
        println!("Selected: Coin ({} available)", inventory.coins);
    }
    if keyboard_input.just_pressed(KeyCode::KeyI) {
        println!(
            "Inventory => Coins: {}, Blocks: {} | Selected: {:?}",
            inventory.coins, inventory.blocks, inventory.selected
        );
    }
}

fn use_selected_item(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    cam_query: Query<&GlobalTransform, With<Camera3d>>,
    mut inventory: ResMut<Inventory>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) { return; }
    if inventory.selected != ItemKind::Block || inventory.blocks == 0 { return; }

    if let Some(cam) = cam_query.iter().next() {
        let forward = cam.forward().as_vec3();
        let start = cam.translation();
        let place_pos = Vec3::new(start.x + forward.x * 1.5, 0.5, start.z + forward.z * 1.5);
        let mesh = meshes.add(Cuboid::new(3.0, 2.0, 3.0));
        let color = materials.add(Color::srgb(0.6, 0.4, 0.2));
        println!("Placing block at {:?}", place_pos);
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(color),
            Transform::from_translation(place_pos),
            Ground{ x: place_pos.x, z: place_pos.z, y: 2.9, width: 1.5, depth: 1.5 },
        ));
        inventory.blocks -= 1;
        println!("Placed a block. Blocks left: {}", inventory.blocks);
    }
}

