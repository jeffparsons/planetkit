use pk::net::GameMessage;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Message {}
impl GameMessage for Message{}
