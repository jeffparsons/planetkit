use pk::net::GameMessage;

use pk::cell_dweller::CellDwellerMessage;

use ::player::PlayerMessage;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum Message {
    CellDweller(CellDwellerMessage),
    Player(PlayerMessage),
}
impl GameMessage for Message{}
