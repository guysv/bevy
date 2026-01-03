//! Illustrates how to use a custom cursor image with a texture atlas and
//! animation.

use std::time::Duration;

use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (setup_cursor_icon, setup_cameras, setup_instructions, setup_sprite),
        )
        .add_systems(
            Update,
            (
                execute_animation,
                toggle_texture_atlas,
                toggle_flip_x,
                toggle_flip_y,
                cycle_rect,
                toggle_minimap_camera,
                despawn_temporary_text,
            ),
        )
        .run();
}

fn setup_cursor_icon(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    window: Single<Entity, With<Window>>,
) {
    // Only set up custom cursor if ENABLE_CUSTOM_CURSOR_AT_STARTUP env var is set.
    // If not set, the default system cursor will be used.
    if std::env::var("ENABLE_CUSTOM_CURSOR_AT_STARTUP").is_err() {
        return;
    }

    let layout =
        TextureAtlasLayout::from_grid(UVec2::splat(64), 20, 10, Some(UVec2::splat(5)), None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    let animation_config = AnimationConfig::new(0, 199, 1, 4);

    commands.entity(*window).insert((
        CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            // Image to use as the cursor.
            handle: asset_server
                .load("cursors/kenney_crosshairPack/Tilesheet/crosshairs_tilesheet_white.png"),
            // Optional texture atlas allows you to pick a section of the image
            // and animate it.
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: animation_config.first_sprite_index,
            }),
            flip_x: false,
            flip_y: false,
            // Optional section of the image to use as the cursor.
            rect: None,
            // The hotspot is the point in the cursor image that will be
            // positioned at the mouse cursor's position.
            hotspot: (0, 0),
        })),
        animation_config,
    ));
}

fn setup_cameras(mut commands: Commands) {
    let projection = OrthographicProjection::default_2d();
    commands.spawn((
        Camera2d,
        MainCamera,
        Camera {
            order: 0,
            ..default()
        },
        Projection::Orthographic(projection),
        Transform::from_xyz(220.0, 140.0, 0.0),
        IsDefaultUiCamera,
    ));

    // Spawn minimap camera at startup if SPAWN_MINIMAP_CAMERA_AT_STARTUP env var is set.
    // This reproduces a bug where sprite picking fails when the minimap camera exists
    // at startup. The sprite picking backend uses `.find()` to select a camera, which
    // may select the minimap camera (order 1) instead of the main camera (order 0).
    // Since the minimap camera uses RenderLayers::layer(1), it won't see sprites on layer 0,
    // causing picking to fail.
    if std::env::var("SPAWN_MINIMAP_CAMERA_AT_STARTUP").is_ok() {
        commands.spawn((
            Camera2d,
            MinimapCamera,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::Custom(Color::srgba(0.243, 0.361, 0.522, 0.82)),
                ..default()
            },
            RenderLayers::layer(1),
        ));
    }
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        Text::new(
            "Press T to toggle the cursor's `texture_atlas`.\n
Press X to toggle the cursor's `flip_x` setting.\n
Press Y to toggle the cursor's `flip_y` setting.\n
Press R to cycle through the sections of the cursor's image using `rect`.\n
Press C to toggle the minimap camera.",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct MinimapCamera;

#[derive(Component)]
struct ClickableSprite {
    color_index: u8,
}

#[derive(Component)]
struct TemporaryText {
    timer: Timer,
}

#[derive(Component)]
struct AnimationConfig {
    first_sprite_index: usize,
    last_sprite_index: usize,
    increment: usize,
    fps: u8,
    frame_timer: Timer,
}

impl AnimationConfig {
    fn new(first: usize, last: usize, increment: usize, fps: u8) -> Self {
        Self {
            first_sprite_index: first,
            last_sprite_index: last,
            increment,
            fps,
            frame_timer: Self::timer_from_fps(fps),
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(Duration::from_secs_f32(1.0 / (fps as f32)), TimerMode::Once)
    }
}

/// This system loops through all the sprites in the [`CursorIcon`]'s
/// [`TextureAtlas`], from [`AnimationConfig`]'s `first_sprite_index` to
/// `last_sprite_index`.
fn execute_animation(time: Res<Time>, mut query: Query<(&mut AnimationConfig, &mut CursorIcon)>) {
    for (mut config, mut cursor_icon) in &mut query {
        if let CursorIcon::Custom(CustomCursor::Image(ref mut image)) = *cursor_icon {
            config.frame_timer.tick(time.delta());

            if config.frame_timer.is_finished()
                && let Some(atlas) = image.texture_atlas.as_mut()
            {
                atlas.index += config.increment;

                if atlas.index > config.last_sprite_index {
                    atlas.index = config.first_sprite_index;
                }

                config.frame_timer = AnimationConfig::timer_from_fps(config.fps);
            }
        }
    }
}

fn toggle_texture_atlas(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut CursorIcon, With<Window>>,
    mut cached_atlas: Local<Option<TextureAtlas>>, // this lets us restore the previous value
) {
    if input.just_pressed(KeyCode::KeyT) {
        for mut cursor_icon in &mut query {
            if let CursorIcon::Custom(CustomCursor::Image(ref mut image)) = *cursor_icon {
                match image.texture_atlas.take() {
                    Some(a) => {
                        // Save the current texture atlas.
                        *cached_atlas = Some(a.clone());
                    }
                    None => {
                        // Restore the cached texture atlas.
                        if let Some(cached_a) = cached_atlas.take() {
                            image.texture_atlas = Some(cached_a);
                        }
                    }
                }
            }
        }
    }
}

fn toggle_flip_x(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut CursorIcon, With<Window>>,
) {
    if input.just_pressed(KeyCode::KeyX) {
        for mut cursor_icon in &mut query {
            if let CursorIcon::Custom(CustomCursor::Image(ref mut image)) = *cursor_icon {
                image.flip_x = !image.flip_x;
            }
        }
    }
}

fn toggle_flip_y(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut CursorIcon, With<Window>>,
) {
    if input.just_pressed(KeyCode::KeyY) {
        for mut cursor_icon in &mut query {
            if let CursorIcon::Custom(CustomCursor::Image(ref mut image)) = *cursor_icon {
                image.flip_y = !image.flip_y;
            }
        }
    }
}

/// This system alternates the [`CursorIcon`]'s `rect` field between `None` and
/// 4 sections/rectangles of the cursor's image.
fn cycle_rect(input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut CursorIcon, With<Window>>) {
    if !input.just_pressed(KeyCode::KeyR) {
        return;
    }

    const RECT_SIZE: u32 = 32; // half the size of a tile in the texture atlas

    const SECTIONS: [Option<URect>; 5] = [
        Some(URect {
            min: UVec2::ZERO,
            max: UVec2::splat(RECT_SIZE),
        }),
        Some(URect {
            min: UVec2::new(RECT_SIZE, 0),
            max: UVec2::new(2 * RECT_SIZE, RECT_SIZE),
        }),
        Some(URect {
            min: UVec2::new(0, RECT_SIZE),
            max: UVec2::new(RECT_SIZE, 2 * RECT_SIZE),
        }),
        Some(URect {
            min: UVec2::new(RECT_SIZE, RECT_SIZE),
            max: UVec2::splat(2 * RECT_SIZE),
        }),
        None, // reset to None
    ];

    for mut cursor_icon in &mut query {
        if let CursorIcon::Custom(CustomCursor::Image(ref mut image)) = *cursor_icon {
            let next_rect = SECTIONS
                .iter()
                .cycle()
                .skip_while(|&&corner| corner != image.rect)
                .nth(1) // move to the next element
                .unwrap_or(&None);

            image.rect = *next_rect;
        }
    }
}

fn toggle_minimap_camera(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    minimap_camera_query: Query<Entity, With<MinimapCamera>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        let camera_exists = minimap_camera_query.single().is_ok();
        let message = if camera_exists {
            let camera_entity = minimap_camera_query.single().unwrap();
            commands.entity(camera_entity).despawn();
            info!("Minimap camera despawned - picking should work now");
            "Second Camera: DESPAWNED"
        } else {
            commands.spawn((
                Camera2d,
                MinimapCamera,
                Camera {
                    order: 1,
                    clear_color: ClearColorConfig::Custom(Color::srgba(0.243, 0.361, 0.522, 0.82)),
                    ..default()
                },
                RenderLayers::layer(1),
            ));
            info!("Minimap camera spawned - picking might break");
            "Second Camera: SPAWNED"
        };

        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(50.0),
                left: Val::Px(50.0),
                ..default()
            },
            Text::new(message),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            TemporaryText {
                timer: Timer::from_seconds(1.0, TimerMode::Once),
            },
        ));
    }
}

fn setup_sprite(mut commands: Commands) {
    commands
        .spawn((
            Sprite::from_color(Color::srgb(1.0, 0.0, 0.0), Vec2::new(100.0, 100.0)),
            Transform::from_xyz(220.0, 140.0, 1.0),
            Pickable::default(),
            ClickableSprite { color_index: 0 },
        ))
        .observe(|_trigger: On<Pointer<Over>>| {
            info!("Sprite: Mouse Over");
        })
        .observe(|_trigger: On<Pointer<Out>>| {
            info!("Sprite: Mouse Out");
        })
        .observe(
            |_trigger: On<Pointer<Press>>,
             mut sprite: Single<&mut Sprite>,
             mut clickable: Single<&mut ClickableSprite>| {
                info!("Sprite: Clicked! Changing color...");
                clickable.color_index = (clickable.color_index + 1) % 3;
                let new_color = match clickable.color_index {
                    0 => Color::srgb(1.0, 0.0, 0.0),
                    1 => Color::srgb(0.0, 1.0, 0.0),
                    2 => Color::srgb(0.0, 0.0, 1.0),
                    _ => Color::srgb(1.0, 0.0, 0.0),
                };
                sprite.color = new_color;
            },
        )
        .observe(|_trigger: On<Pointer<Release>>| {
            info!("Sprite: Released");
        });
}

fn despawn_temporary_text(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TemporaryText)>,
    time: Res<Time>,
) {
    for (entity, mut temp_text) in query.iter_mut() {
        temp_text.timer.tick(time.delta());
        if temp_text.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
