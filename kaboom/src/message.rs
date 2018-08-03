use pk::net::GameMessage;

use pk::cell_dweller::CellDwellerMessage;

use player::PlayerMessage;

use weapon::WeaponMessage;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum Message {
    CellDweller(CellDwellerMessage),
    Player(PlayerMessage),
    Weapon(WeaponMessage),
}
impl GameMessage for Message {}
