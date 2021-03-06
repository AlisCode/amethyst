//! Demonstrates how to load and render sprites.
//!
//! Sprites are from <https://opengameart.org/content/bat-32x32>.

extern crate amethyst;
extern crate amethyst_animation;
#[macro_use]
extern crate log;
extern crate ron;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod animation;
mod png_loader;
mod sprite;
mod sprite_sheet_loader;

use amethyst::assets::{AssetStorage, Loader};
use amethyst::core::cgmath::{Matrix4, Transform as CgTransform, Vector3};
use amethyst::core::transform::{GlobalTransform, Transform, TransformBundle};
use amethyst::ecs::Entity;
use amethyst::input::InputBundle;
use amethyst::prelude::*;
use amethyst::renderer::{Camera, ColorMask, DisplayConfig, DrawFlat, Event, KeyboardInput,
                         Material, MaterialDefaults, Mesh, Pipeline, PosTex, Projection,
                         RenderBundle, ScreenDimensions, Stage, VirtualKeyCode, WindowEvent, ALPHA};
use amethyst::ui::{DrawUi, UiBundle};
use amethyst_animation::{get_animation_set, AnimationBundle, AnimationCommand, EndControl,
                         MaterialTextureSet};

const BACKGROUND_COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black

#[derive(Debug, Default)]
struct Example {
    /// The bat entities.
    entities: Vec<Entity>,
}

impl State for Example {
    fn on_start(&mut self, mut world: &mut World) {
        initialise_camera(world);

        let sprite_sheet_texture = png_loader::load("texture/bat.32x32.png", world);

        let sprite_w = 32.;
        let sprite_h = 32.;
        let sprite_sheet_definition =
            sprite::SpriteSheetDefinition::new(sprite_w, sprite_h, 2, 6, false);

        let sprite_sheet_index = 0;
        let sprite_sheet = sprite_sheet_loader::load(sprite_sheet_index, &sprite_sheet_definition);

        let sprite_sheet_material = {
            let mat_defaults = world.read_resource::<MaterialDefaults>();
            Material {
                albedo: sprite_sheet_texture.clone(),
                ..mat_defaults.0.clone()
            }
        };

        // Load animations
        let grey_bat_animation = animation::grey_bat(&sprite_sheet, &mut world);
        let brown_bat_animation = animation::brown_bat(&sprite_sheet, &mut world);

        // Calculate offset to centre all sprites
        //
        // The X offset needs to be multiplied because we are drawing the sprites across the window;
        // we don't need to multiply the Y offset because we are only drawing the sprites in 1 row.
        let sprite_count = sprite_sheet.sprites.len();
        let sprite_offset_x = sprite_count as f32 * sprite_w / 2.;
        let sprite_offset_y = sprite_h / 2.;

        let (width, height) = {
            let dim = world.read_resource::<ScreenDimensions>();
            (dim.width(), dim.height())
        };
        // This `Transform` moves the sprites to the middle of the window
        let mut common_transform = Transform::default();
        common_transform.translation = Vector3::new(
            width / 2. - sprite_offset_x,
            height / 2. - sprite_offset_y,
            0.,
        );

        // Store sprite sheet texture in the world's `MaterialTextureSet` resource (singleton hash
        // map)
        world
            .write_resource::<MaterialTextureSet>()
            .insert(sprite_sheet_index, sprite_sheet_texture);

        // Create an entity per sprite.
        for i in 0..sprite_count {
            let mut sprite_transform = Transform::default();
            sprite_transform.translation = Vector3::new(i as f32 * sprite_w, 0., 0.);

            // This combines multiple `Transform`ations.
            // You need to `use amethyst::core::cgmath::Transform`;
            sprite_transform.concat_self(&common_transform);

            let mesh = {
                let loader = world.read_resource::<Loader>();
                loader.load_from_data(
                    create_mesh_vertices(sprite_w, sprite_h).into(),
                    (),
                    &world.read_resource::<AssetStorage<Mesh>>(),
                )
            };

            let animation = if i < (sprite_count >> 1) {
                grey_bat_animation.clone()
            } else {
                brown_bat_animation.clone()
            };

            let entity = world
                .create_entity()
                // The default `Material`, whose textures will be swapped based on the animation.
                .with(sprite_sheet_material.clone())
                // The `Animation` defines the mutation of the `MaterialAnimation`.
                .with(animation.clone())
                // Shift sprite to some part of the window
                .with(sprite_transform)
                // This defines the coordinates in the world, where the sprites should be drawn
                // relative to the entity
                .with(mesh)
                // Used by the engine to compute and store the rendered position.
                .with(GlobalTransform::default())
                .build();

            // We also need to trigger the animation, not just attach it to the entity
            let mut animation_control_set_storage = world.write();
            let animation_set =
                get_animation_set::<u32, Material>(&mut animation_control_set_storage, entity);
            let animation_id = 0;
            animation_set.add_animation(
                animation_id,
                &animation,
                EndControl::Loop(None),
                1., // Rate at which the animation plays
                AnimationCommand::Start,
            );

            // Store the entity
            self.entities.push(entity);
        }
    }

    fn handle_event(&mut self, _: &mut World, event: Event) -> Trans {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                }
                | WindowEvent::Closed => Trans::Quit,
                _ => Trans::None,
            },
            _ => Trans::None,
        }
    }
}

/// This method initialises a camera which will view our sprite.
fn initialise_camera(world: &mut World) -> Entity {
    let (width, height) = {
        let dim = world.read_resource::<ScreenDimensions>();
        (dim.width(), dim.height())
    };
    world
        .create_entity()
        .with(Camera::from(Projection::orthographic(
            0.0,
            width,
            height,
            0.0,
        )))
        .with(GlobalTransform(Matrix4::from_translation(
            Vector3::new(0.0, 0.0, 1.0).into(),
        )))
        .build()
}

fn run() -> Result<(), amethyst::Error> {
    let path = format!(
        "{}/examples/sprites/resources/display_config.ron",
        env!("CARGO_MANIFEST_DIR")
    );

    let assets_directory = format!("{}/examples/assets/", env!("CARGO_MANIFEST_DIR"));
    let config = DisplayConfig::load(&path);

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target(BACKGROUND_COLOUR, 1.0)
            .with_pass(DrawFlat::<PosTex>::new().with_transparency(ColorMask::all(), ALPHA, None))
            .with_pass(DrawUi::new()),
    );

    let mut game = Application::build(assets_directory, Example::default())?
        // RenderBundle gives us a window
        .with_bundle(RenderBundle::new(pipe, Some(config)))?
        // UiBundle relies on this as some Ui objects take input
        .with_bundle(InputBundle::<String, String>::new())?
        // Draws textures
        .with_bundle(UiBundle::<String, String>::new())?
        // Provides sprite animation
        .with_bundle(AnimationBundle::<u32, Material>::new(
            "animation_control_system",
            "sampler_interpolation_system",
        ))?
        // Handles transformations of textures
        .with_bundle(
            TransformBundle::new()
                .with_dep(&["animation_control_system", "sampler_interpolation_system"]),
        )?
        .build()?;

    game.run();

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        error!("Failed to execute example: {}", e);
        ::std::process::exit(1);
    }
}

/// Returns a set of vertices that make up a rectangular mesh of the given size.
///
/// This function expects pixel coordinates -- starting from the top left of the image. X increases
/// to the right, Y increases downwards.
///
/// # Parameters
///
/// * `sprite_w`: Width of each sprite, excluding the border pixel if any.
/// * `sprite_h`: Height of each sprite, excluding the border pixel if any.
fn create_mesh_vertices(sprite_w: f32, sprite_h: f32) -> Vec<PosTex> {
    let tex_coord_left = 0.;
    let tex_coord_right = 1.;
    // Inverse the pixel coordinates when transforming them into texture coordinates, because the
    // render passes' Y axis is 0 from the bottom of the image, and increases to 1.0 at the top of
    // the image.
    let tex_coord_top = 0.;
    let tex_coord_bottom = 1.;

    vec![
        PosTex {
            position: [0., 0., 0.],
            tex_coord: [tex_coord_left, tex_coord_top],
        },
        PosTex {
            position: [sprite_w, 0., 0.],
            tex_coord: [tex_coord_right, tex_coord_top],
        },
        PosTex {
            position: [0., sprite_h, 0.],
            tex_coord: [tex_coord_left, tex_coord_bottom],
        },
        PosTex {
            position: [sprite_w, sprite_h, 0.],
            tex_coord: [tex_coord_right, tex_coord_bottom],
        },
        PosTex {
            position: [0., sprite_h, 0.],
            tex_coord: [tex_coord_left, tex_coord_bottom],
        },
        PosTex {
            position: [sprite_w, 0., 0.],
            tex_coord: [tex_coord_right, tex_coord_top],
        },
    ]
}
