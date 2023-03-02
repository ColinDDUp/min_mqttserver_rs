pub mod mqtt_server;
pub mod mqtt_routes;
pub mod mqtt_entities;

use std::sync::{Arc, Mutex};

// use macro_attr::{self, Builder};
// #[route(0x0c)]
// fn dso()
// {
//     println!("dso");
// }
use mqtt_server::{MqttServer,MqttServerEntity};
fn main() {
    let server=Arc::new(Mutex::new(MqttServerEntity::init()));
    MqttServer::binding().run(server);
}
