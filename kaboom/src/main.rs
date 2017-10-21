extern crate planetkit as pk;
extern crate shred;
extern crate specs;
extern crate rand;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate clap;
extern crate piston_window;

mod fighter;
mod game_state;
mod game_system;
mod planet;
mod message;

use message::Message;
use clap::{AppSettings, Arg, SubCommand};

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

    let (mut app, mut window) = pk::simple::new_empty(add_systems);

    // Should we start a server or connect to one?
    // NLL SVP.
    {
        use std::net::SocketAddr;
        use piston_window::AdvancedWindow;
        use pk::net::ServerResource;

        // Systems we added will have ensured ServerResource is present.
        let server_resource = app.world_mut().write_resource::<ServerResource<Message>>();
        let mut server = server_resource.server.lock().expect("Failed to lock server");
        if let Some(_matches) = matches.subcommand_matches("listen") {
            window.set_title("Kaboom (server)".to_string());
            // TODO: make port configurable
            server.start_listen(62831);
        } else if let Some(matches) = matches.subcommand_matches("connect") {
            window.set_title("Kaboom (client)".to_string());
            // TODO: make port configurable
            let connect_addr = matches.value_of("SERVER_ADDRESS").unwrap();
            let connect_addr: SocketAddr = connect_addr.parse().expect("Invalid SERVER_ADDRESS");
            server.connect(connect_addr);
        }
    }

    create_entities(app.world_mut());
    app.run(&mut window);
}

fn add_systems(
    logger: &slog::Logger,
    world: &mut specs::World,
    dispatcher_builder: specs::DispatcherBuilder<'static, 'static>,
) -> specs::DispatcherBuilder<'static, 'static> {
    let game_system = game_system::GameSystem::new(logger, world);
    let recv_system = pk::net::RecvSystem::<Message>::new(logger, world);
    let send_system = pk::net::SendSystem::<Message>::new(logger, world);

    dispatcher_builder
        .add(game_system, "woolgather_game", &[])
        .add(recv_system, "net_recv", &[])
        .add(send_system, "net_send", &[])
}

fn create_entities(world: &mut specs::World) {
    // TODO: move all this into GameSystem.

    use pk::cell_dweller::ActiveCellDweller;

    // Create the globe first, because we'll need it to figure out where
    // to place the player character.
    let globe_entity = planet::create_now(world);

    // Create the player character.
    let fighter_entity = fighter::create_now(world, globe_entity);
    // Set our new shepherd player character as the currently controlled cell dweller.
    world.write_resource::<ActiveCellDweller>().maybe_entity = Some(fighter_entity);

    // Create basic third-person following camera.
    pk::simple::create_simple_chase_camera_now(world, fighter_entity);
}
