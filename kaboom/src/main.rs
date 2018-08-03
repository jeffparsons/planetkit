extern crate planetkit as pk;
extern crate rand;
extern crate shred;
extern crate specs;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;
extern crate piston;
extern crate piston_window;
extern crate serde;

mod client_state;
mod death_system;
mod fighter;
mod game_state;
mod game_system;
mod health;
mod message;
mod planet;
mod player;
mod recv_demux_system;
mod send_mux_system;
mod weapon;

use std::sync::mpsc;

use clap::{AppSettings, Arg, SubCommand};
use message::Message;
use recv_demux_system::RecvDemuxSystem;
use send_mux_system::SendMuxSystem;

fn main() {
    let matches = clap::App::new("Kaboom")
        .author("Jeff Parsons <jeff@parsons.io>")
        .about("Blow stuff up!")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("connect")
                .about("connect to a server")
                .arg(
                    Arg::with_name("SERVER_ADDRESS")
                        .help("The IP or hostname and port of the server to connect to")
                        .required(true)
                        .index(1)
                )
        )
        .subcommand(
            SubCommand::with_name("listen")
                .about("start a server, and play")
        )
        // TODO: dedicated server, and helper script
        // to launch a dedicated server then connect
        // a client to it.
        .get_matches();

    // Set up input adapters.
    let (shoot_input_sender, shoot_input_receiver) = mpsc::channel();
    let shoot_input_adapter = Box::new(weapon::ShootInputAdapter::new(shoot_input_sender));

    let mut app = pk::AppBuilder::new()
        .with_networking::<Message>()
        .with_common_systems()
        .with_systems(
            |logger: &slog::Logger,
             world: &mut specs::World,
             dispatcher_builder: specs::DispatcherBuilder<'static, 'static>| {
                add_systems(logger, world, dispatcher_builder, shoot_input_receiver)
            },
        )
        .build_gui();

    app.add_input_adapter(shoot_input_adapter);

    // Should we start a server or connect to one?
    // NLL SVP.
    {
        use piston_window::AdvancedWindow;
        use pk::net::ServerResource;
        use std::net::SocketAddr;

        // Systems we added will have ensured ServerResource is present.
        let (world, window) = app.world_and_window_mut();
        let server_resource = world.write_resource::<ServerResource<Message>>();
        let mut server = server_resource
            .server
            .lock()
            .expect("Failed to lock server");
        if let Some(_matches) = matches.subcommand_matches("listen") {
            window.set_title("Kaboom (server)".to_string());
            // TODO: make port configurable
            server.start_listen(62831);

            // Let the game know it's in charge of the world.
            let mut node_resource = world.write_resource::<pk::net::NodeResource>();
            node_resource.is_master = true;
        } else if let Some(matches) = matches.subcommand_matches("connect") {
            window.set_title("Kaboom (client)".to_string());
            // TODO: make port configurable
            let connect_addr = matches.value_of("SERVER_ADDRESS").unwrap();
            let connect_addr: SocketAddr = connect_addr.parse().expect("Invalid SERVER_ADDRESS");
            server.connect(connect_addr);
        }
    }

    app.run();
}

fn add_systems(
    logger: &slog::Logger,
    world: &mut specs::World,
    dispatcher_builder: specs::DispatcherBuilder<'static, 'static>,
    shoot_input_receiver: mpsc::Receiver<weapon::ShootEvent>,
) -> specs::DispatcherBuilder<'static, 'static> {
    // TODO: systems should register these.
    world.register::<weapon::Grenade>();
    world.register::<fighter::Fighter>();
    world.register::<::health::Health>();

    let game_system = game_system::GameSystem::new(logger);
    let new_peer_system = pk::net::NewPeerSystem::<Message>::new(logger, world);
    let recv_system = pk::net::RecvSystem::<Message>::new(logger, world);
    let recv_demux_system = RecvDemuxSystem::new(logger, world);
    let cd_recv_system = pk::cell_dweller::RecvSystem::new(logger);
    let weapon_recv_system = weapon::RecvSystem::new(logger);
    let shoot_system = weapon::ShootSystem::new(shoot_input_receiver, logger);
    let explode_system = weapon::ExplodeSystem::new(logger);
    let death_system = death_system::DeathSystem::new(logger);
    let velocity_system = pk::physics::VelocitySystem::new(logger);
    let gravity_system = pk::physics::GravitySystem::new(logger);
    let physics_system = pk::physics::PhysicsSystem::new();
    let send_mux_system = SendMuxSystem::new(logger);
    let send_system = pk::net::SendSystem::<Message>::new(logger, world);

    // TODO: these barriers are probably a bad idea;
    // we should be perfectly happy to render while we're sending
    // things over the network. Maybe consider "dummy systems"
    // used as lifecycle hooks instead.
    dispatcher_builder
        .with(game_system, "kaboom_game", &[])
        .with(new_peer_system, "new_peer_system", &[])
        .with(recv_system, "net_recv", &[])
        .with(recv_demux_system, "recv_demux", &["net_recv"])
        .with_barrier()
        .with(cd_recv_system, "cd_recv", &[])
        .with(weapon_recv_system, "weapon_recv", &[])
        .with(shoot_system, "shoot_grenade", &[])
        .with(explode_system, "explode_grenade", &[])
        .with(death_system, "death", &[])
        .with(gravity_system, "gravity", &[])
        .with(velocity_system, "velocity", &["gravity"])
        // TODO: move gravity into nphysics as a force.
        .with(physics_system, "physics", &["gravity"])
        // TODO: explicitly add all systems here,
        // instead of whatever "simple" wants to throw at you.
        // At the moment they might execute in an order that
        // could add unnecessary latency to receiving/sending messages.
        .with_barrier()
        .with(send_mux_system, "send_mux", &[])
        .with(send_system, "net_send", &["send_mux"])
}
