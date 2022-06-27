use bevy::prelude::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use spmc::{Receiver, Sender};
use std::collections::HashMap;
use std::io::{Error, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::{thread, time};

const NUMBEROFBEADS: u32 = 10;

#[derive(Serialize, Deserialize, Debug)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConnectionParams {
    status: String,
    connection_type: ConnectionType,
    data: Option<EntityState>,
    boundary: Option<(f32, f32)>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EntityState {
    entity_atrib: HashMap<u32, Vec3>,
}

#[derive(Serialize, Deserialize, Debug)]
enum ConnectionType {
    Init,
    GetEntity,
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").expect("Could not bind");

    let (tx, rx) = spmc::channel();
    let mut clients = vec![];

    thread::spawn(move || {
        let mut beads: EntityState = EntityState {
            entity_atrib: HashMap::new(),
        };
        spawn_entities(&mut beads);
        handle_bead_movement(tx, &mut beads);
    });

    for stream in listener.incoming() {
        let rx = rx.clone();
        match stream {
            Err(e) => {
                eprintln!("failed: {}", e)
            }

            Ok(stream) => {
                clients.push(thread::spawn(move || {
                    handle_connection(stream, &rx).unwrap_or_else(|error| eprintln!("{:?}", error));
                }));
            }
        }
    }
    for client in clients {
        client.join().unwrap();
    }
}

fn spawn_entities(beads: &mut EntityState) {
    let mut count: u32 = 0;
    while count < NUMBEROFBEADS {
        // RANDOM Position
        let mut rng = thread_rng();
        let pos_x: f32 = rng.gen_range(-200.0..200.0);
        let pos_y: f32 = rng.gen_range(-200.0..200.0);

        let translation = Vec3::new(pos_x, pos_y, 0.);

        beads.entity_atrib.insert(count, translation);
        count += 1;
    }
}

fn handle_bead_movement(mut tx: Sender<EntityState>, beads: &mut EntityState) {
    let mut velocity = Velocity {
        x: 0.,
        y: 0.,
        z: 0.,
    };

    loop {
        for (_, translation) in beads.entity_atrib.iter_mut() {
            //RANDOM VELOCITY
            let mut rng = thread_rng();
            let vel_x: f32 = rng.gen_range(-2.0..2.0);
            let vel_y: f32 = rng.gen_range(-2.0..2.0);

            velocity.x = vel_x;
            velocity.y = vel_y;

            translation.x += velocity.x;
            translation.y += velocity.y;
        }

        tx.send(beads.clone()).unwrap();

        thread::sleep(Duration::from_millis(30));
    }
}

fn handle_connection(mut stream: TcpStream, rx: &Receiver<EntityState>) -> Result<(), Error> {
    let ten_millis = time::Duration::from_millis(10);
    loop {
        //Read from client

        //100 beads
        let game_state = rx.recv().unwrap();

        //Read from client
        let mut buffer = [1; 80000];
        let len = stream.read(&mut buffer).unwrap();
        let message = String::from_utf8_lossy(&mut buffer[..len]);
        println!("This is the message{}", message);
        let connection_request: ConnectionParams = serde_json::from_str(&message).unwrap();
        let mut entities_in_frame: EntityState = EntityState {
            entity_atrib: HashMap::new(),
        };

        match connection_request.connection_type {
            //If connection created - Spawn 100 entities and pass IDs
            ConnectionType::Init => {
                //Write to client
                let entity_data = ConnectionParams {
                    status: "Connected".to_string(),
                    connection_type: ConnectionType::GetEntity,
                    data: Some(game_state),
                    boundary: None,
                };
                let serialized_entity_data = serde_json::to_string(&entity_data).unwrap();
                let _ = stream.write(serialized_entity_data.as_bytes());
            }
            ConnectionType::GetEntity => {
                for (entity, translation) in game_state.entity_atrib {
                    if translation.x < connection_request.boundary.unwrap().0
                        || translation.y < connection_request.boundary.unwrap().1
                    {
                        entities_in_frame.entity_atrib.insert(entity, translation);
                    }
                }

                let serialized = serde_json::to_string(&entities_in_frame).unwrap();
                thread::sleep(ten_millis);
                stream.write(serialized.as_bytes())?;
                let _ = stream.flush();
            }
        }
    }
}
