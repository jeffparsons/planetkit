use std::sync::mpsc;
use specs;
use specs::{Fetch, LazyUpdate, Entities, ReadStorage, WriteStorage};
use slog::Logger;
use piston::input::Input;

use pk::cell_dweller::{
    CellDweller,
    ActiveCellDweller,
    SendMessageQueue,
};
use pk::types::*;
use pk::input_adapter;
use pk::render;
use pk::physics::Velocity;
use pk::Spatial;

pub struct ShootInputAdapter {
    sender: mpsc::Sender<ShootEvent>,
}

impl ShootInputAdapter {
    pub fn new(sender: mpsc::Sender<ShootEvent>) -> ShootInputAdapter {
        ShootInputAdapter { sender: sender }
    }
}

impl input_adapter::InputAdapter for ShootInputAdapter {
    fn handle(&self, input_event: &Input) {
        use piston::input::{Button, ButtonState};
        use piston::input::keyboard::Key;

        if let &Input::Button(button_args) = input_event {
            if let Button::Keyboard(key) = button_args.button {
                let is_down = match button_args.state {
                    ButtonState::Press => true,
                    ButtonState::Release => false,
                };
                match key {
                    Key::Space => self.sender.send(ShootEvent(is_down)).unwrap(),
                    _ => (),
                }
            }
        }
    }
}

pub struct ShootEvent(bool);

pub struct ShootSystem {
    input_receiver: mpsc::Receiver<ShootEvent>,
    log: Logger,
    shoot: bool,
}

impl ShootSystem {
    pub fn new(
        world: &mut specs::World,
        input_receiver: mpsc::Receiver<ShootEvent>,
        parent_log: &Logger,
    ) -> ShootSystem {
        use pk::AutoResource;
        SendMessageQueue::ensure(world);
        ActiveCellDweller::ensure_registered(world);

        ShootSystem {
            input_receiver: input_receiver,
            log: parent_log.new(o!()),
            shoot: false,
        }
    }

    fn consume_input(&mut self) {
        loop {
            match self.input_receiver.try_recv() {
                Ok(ShootEvent(b)) => self.shoot = b,
                Err(_) => return,
            }
        }
    }
}

impl<'a> specs::System<'a> for ShootSystem {
    type SystemData = (
        Fetch<'a, TimeDeltaResource>,
        Entities<'a>,
        Fetch<'a, LazyUpdate>,
        ReadStorage<'a, CellDweller>,
        Fetch<'a, ActiveCellDweller>,
        WriteStorage<'a, Spatial>,
    );

    fn run(&mut self, data: Self::SystemData) {
        self.consume_input();
        let (
            _dt,
            entities,
            updater,
            cell_dwellers,
            active_cell_dweller_resource,
            spatials,
        ) = data;

        if self.shoot {
            self.shoot = false;

            info!(self.log, "Bang!");

            // TODO: send this as a network message instead:

            // Make visual appearance of bullet.
            // For now this is just an axes mesh.
            let mut bullet_visual = render::Visual::new_empty();
            bullet_visual.proto_mesh = Some(render::make_axes_mesh());

            // Place the bullet in the same location as the player,
            // relative to the same globe.
            let active_cell_dweller_entity = match active_cell_dweller_resource.maybe_entity {
                Some(entity) => entity,
                None => {
                    warn!(self.log, "Trying to shoot without an active CellDweller");
                    return
                },
            };
            let cd = cell_dwellers.get(active_cell_dweller_entity).expect(
                "Someone deleted the controlled entity's CellDweller",
            );
            let cd_spatial = spatials.get(active_cell_dweller_entity).expect(
                "Someone deleted the controlled entity's Spatial",
            );
            // Get the associated globe entity, complaining loudly if we fail.
            let globe_entity = match cd.globe_entity {
                Some(globe_entity) => globe_entity,
                None => {
                    warn!(
                        self.log,
                        "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!"
                    );
                    return;
                }
            };
            // Put bullet where player is.
            let bullet_spatial = Spatial::new(
                globe_entity,
                cd_spatial.local_transform(),
            );

            // Start with an arbitrary small velocity, based on what
            // direction the cell dweller is facing.
            //
            // (TODO: turn panic into error log.)
            //
            // NOTE: the `unwrap` here is not the normal meaning of unwrap;
            // in this case it is a totally innocuous function for extracting
            // the interior value of a unit vector.
            let dir = &cd_spatial.local_transform().rotation;
            let cd_relative_velocity = (Vec3::z_axis().unwrap() + Vec3::y_axis().unwrap()) * 7.0;
            let bullet_velocity = Velocity::new(dir * cd_relative_velocity);

            // Build the entity.
            let entity = entities.create();
            updater.insert(entity, bullet_visual);
            updater.insert(entity, bullet_spatial);
            updater.insert(entity, bullet_velocity);
        }
    }
}
