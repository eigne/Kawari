use binrw::binrw;
use physis::sestring::SeString;

use crate::common::MESSAGE_MAX_LENGTH;
use crate::ipc::chat::ChatChannel;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendPartyMessage {
    pub chatchannel: ChatChannel,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[brw(pad_size_to = MESSAGE_MAX_LENGTH)]
    pub message: SeString,
}
