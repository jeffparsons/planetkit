use pk::net::GameMessage;

use pk::cell_dweller::CellDwellerMessage;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum Message {
    CellDweller(CellDwellerMessage),
}
impl GameMessage for Message{}
