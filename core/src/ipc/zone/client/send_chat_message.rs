use binrw::binrw;
use physis::sestring::SeString;

use crate::common::{MESSAGE_MAX_LENGTH, ObjectId, Position};
use crate::ipc::chat::ChatChannelType;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendChatMessage {
    #[brw(pad_before = 4)] // empty
    pub actor_id: ObjectId,

    pub pos: Position,
    pub rotation: f32,

    pub channel: ChatChannelType,

    #[brw(pad_after = 6)] // seems to be junk?
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[brw(pad_size_to = MESSAGE_MAX_LENGTH)]
    pub message: SeString,
}
