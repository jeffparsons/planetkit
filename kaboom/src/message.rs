use crate::pk::net::GameMessage;

use crate::pk::cell_dweller::CellDwellerMessage;

use crate::player::PlayerMessage;

use crate::weapon::WeaponMessage;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum Message {
    CellDweller(CellDwellerMessage),
    Player(PlayerMessage),
    Weapon(WeaponMessage),
}
impl GameMessage for Message {}
