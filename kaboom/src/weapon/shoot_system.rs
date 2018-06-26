use std::sync::mpsc;
use specs;
use specs::{Read, ReadExpect, WriteExpect, ReadStorage, WriteStorage};
use slog::Logger;
use piston::input::Input;

use pk::cell_dweller::ActiveCellDweller;
use pk::types::*;
use pk::input_adapter;
use pk::net::{SendMessageQueue, Destination, Transport, SendMessage, NetMarker};

use super::{ShootGrenadeMessage, WeaponMessage};
use ::fighter::Fighter;
use ::client_state::ClientState;
use ::message::Message;

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
        input_receiver: mpsc::Receiver<ShootEvent>,
        parent_log: &Logger,
    ) -> ShootSystem {
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
        Read<'a, TimeDeltaResource>,
        Read<'a, ActiveCellDweller>,
        WriteStorage<'a, Fighter>,
        ReadExpect<'a, ClientState>,
        WriteExpect<'a, SendMessageQueue<Message>>,
        ReadStorage<'a, NetMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        self.consume_input();
        let (
            dt,
            active_cell_dweller_resource,
            mut fighters,
            client_state,
            mut send_message_queue,
            net_markers,
        ) = data;

        // Find the active fighter, even if we're not currently trying to shoot;
        // we might need to count down the time until we can next shoot.
        // If there isn't one, then just silently move on.
        let active_cell_dweller_entity = match active_cell_dweller_resource.maybe_entity {
            Some(entity) => entity,
            None => return,
        };

        if !fighters.get(active_cell_dweller_entity).is_some() {
            // This entity hasn't been realised yet;
            // can't do anything else with it this frame.
            // TODO: isn't this was `is_alive` is supposed to achieve?
            // And yet it doesn't seem to...
            return;
        }

        // Assume it is a fighter, because those are the only cell dwellers
        // you're allowed to control in this game.
        let active_fighter = fighters.get_mut(active_cell_dweller_entity).expect("Cell dweller should have had a fighter attached!");

        // Count down until we're allowed to shoot next.
        if active_fighter.seconds_until_next_shot > 0.0 {
            active_fighter.seconds_until_next_shot = (active_fighter.seconds_until_next_shot - dt.0).max(0.0);
        }
        let still_waiting_to_shoot = active_fighter.seconds_until_next_shot > 0.0;

        if self.shoot && ! still_waiting_to_shoot{
            self.shoot = false;

            let fired_by_player_id = client_state.player_id.expect("There should be a current player.");
            let fired_by_cell_dweller_entity_id = net_markers.get(active_cell_dweller_entity).expect("Active cell dweller should have global identity").id;

            // Place the bullet in the same location as the player,
            // relative to the same globe.
            info!(self.log, "Fire!");

            // Ask the server/master to spawn a grenade.
            // (TODO: really need to decide on termonology around server/master/client/peer/etc.)
            send_message_queue.queue.push_back(
                SendMessage {
                    destination: Destination::Master,
                    game_message: Message::Weapon(
                        WeaponMessage::ShootGrenade(
                            ShootGrenadeMessage {
                                fired_by_player_id: fired_by_player_id,
                                fired_by_cell_dweller_entity_id: fired_by_cell_dweller_entity_id,
                            }
                        )
                    ),
                    transport: Transport::UDP,
                }
            );

            // Reset time until we can shoot again.
            active_fighter.seconds_until_next_shot = active_fighter.seconds_between_shots;
        }
    }
}
