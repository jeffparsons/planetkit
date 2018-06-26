mod shoot_system;
mod explode_system;
mod grenade;
mod recv_system;

pub use self::shoot_system::ShootSystem;
pub use self::recv_system::RecvSystem;
pub use self::explode_system::ExplodeSystem;
pub use self::shoot_system::ShootEvent;
pub use self::shoot_system::ShootInputAdapter;
pub use self::grenade::Grenade;

use std::collections::vec_deque::VecDeque;

use pk::net::RecvMessage;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum WeaponMessage {
    ShootGrenade(ShootGrenadeMessage),
    NewGrenade(NewGrenadeMessage),
    // TODO: Can this become a generic when the specs
    // release with 'saveload' comes along?
    // DeleteGrenade(...),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct ShootGrenadeMessage {
    fired_by_player_id: ::player::PlayerId,
    fired_by_cell_dweller_entity_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct NewGrenadeMessage {
    fired_by_player_id: ::player::PlayerId,
    fired_by_cell_dweller_entity_id: u64,
}

/// `World`-global resource for inbound weapon-related network messages.
#[derive(Default)]
pub struct RecvMessageQueue {
    pub queue: VecDeque<RecvMessage<WeaponMessage>>,
}
