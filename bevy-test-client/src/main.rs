use crate::components::Ball;
use crate::components::Velocity;
use bevy::prelude::*;
use components::CustomID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

mod components;

const BALL_SPRITE: &str = "ball.png";
const SERVER_ADDRESS: &str = "localhost:8000";

struct WinSize {
    w: f32,
    h: f32,
}

struct GameTextures {
    ball: Handle<Image>,
}

struct Connection {
    stream: TcpStream,
}

#[derive(Serialize, Deserialize, Debug)]
enum ConnectionType {
    Init,
    GetEntity,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConnectionParams {
    status: String,
    connection_type: ConnectionType,
    data: Option<EntityState>,
    boundary: Option<(f32, f32)>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct EntityState {
    entity_atrib: HashMap<u32, Vec3>,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(WindowDescriptor {
            title: "Client-Server Simulation".to_string(),
            width: 500.0,
            height: 500.0,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_system)
        .add_startup_system_to_stage(StartupStage::PostStartup, ball_spawn_system)
        .add_system(movement_update_system)
        .run();
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut windows: ResMut<Windows>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let window = windows.get_primary_mut().unwrap();
    let (win_h, win_w) = (window.width(), window.height());

    //add window size resource
    let win_size = WinSize { w: win_w, h: win_h };
    commands.insert_resource(win_size);

    let game_textures = GameTextures {
        ball: asset_server.load(BALL_SPRITE),
    };

    commands.insert_resource(game_textures);

    //Add connection as resource
    let connection = Connection {
        stream: TcpStream::connect(SERVER_ADDRESS).unwrap(),
    };

    commands.insert_resource(connection);
}

fn ball_spawn_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    mut connection: ResMut<Connection>,
) {
    // Connect to server and notify that client is connected
    let message: ConnectionParams = ConnectionParams {
        status: "Connected".to_string(),
        connection_type: ConnectionType::Init,
        data: None,
        boundary: None,
    };
    let serialized = serde_json::to_string(&message).unwrap();
    let _ = connection.stream.write(serialized.as_bytes());

    //Read entities from Server
    let mut buffer = [1; 80000];
    let len = connection.stream.read(&mut buffer).unwrap();
    let message = String::from_utf8_lossy(&mut buffer[..len]);
    let deserialized_entity_state: ConnectionParams = serde_json::from_str(&message).unwrap();

    for (server_entity, server_translation) in
        deserialized_entity_state.data.unwrap().entity_atrib.iter()
    {
        commands
            .spawn_bundle(SpriteBundle {
                texture: game_textures.ball.clone(),
                transform: Transform {
                    translation: *server_translation,
                    scale: Vec3::new(0.05, 0.05, 0.),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Velocity {
                x: 0.,
                y: 0.,
                z: 0.,
            })
            .insert(Ball)
            .insert(CustomID(*server_entity));
    }
}

fn movement_update_system(
    window_size: Res<WinSize>,
    mut connection: ResMut<Connection>,
    mut query: Query<(Entity, &mut CustomID, &mut Transform), With<Ball>>,
) {
    let boundary = (window_size.w, window_size.h);

    let boundary_request = ConnectionParams {
        connection_type: ConnectionType::GetEntity,
        status: "Connected".to_string(),
        data: None,
        boundary: Some(boundary),
    };

    // println!("{:?}", window_size.w);

    //Send the boundary to server here
    let serialized_boundary = serde_json::to_string(&boundary_request).unwrap();
    let _ = connection.stream.write(serialized_boundary.as_bytes());

    //Read entities from Server
    let mut buffer = [1; 8000];
    let len = connection.stream.read(&mut buffer).unwrap();
    let message = String::from_utf8_lossy(&mut buffer[..len]);

    if message.len() > 3 {
        println!("THIS IS THE MESSAGE ############## {}", message);

        let entities_within_bounds: EntityState = serde_json::from_str(&message).unwrap();
        for (_, customid, mut transform) in query.iter_mut() {
            if !entities_within_bounds
                .entity_atrib
                .contains_key(&customid.0)
            {
                transform.translation.x = 5000.;
            }
        }

        for (server_entity, server_translation) in entities_within_bounds.entity_atrib.iter() {
            for (_, customid, mut transform) in query.iter_mut() {
                //Client current entities
                if server_entity == &customid.0 {
                    transform.translation = *server_translation;
                }
            }
        }
    } else {
    }
}
