use specs::Priority;

// TODO: express this using constraints instead of magic numbers.

pub const CD_MOVEMENT: Priority = 110;
pub const CD_MINING: Priority = 100;
pub const CD_PHYSICS: Priority = 90;
pub const CHUNK_VIEW: Priority = 50;
