use binrw::binrw;
use physis::sestring::SeString;

use crate::common::MESSAGE_MAX_LENGTH;
use crate::ipc::chat::ChatChannel;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendCWLinkshellMessage {
    #[brw(pad_after = 1)] // Seems to be empty/zeroes
    pub chatchannel: ChatChannel,

    #[brw(pad_after = 7)] // Seems to be empty/zeroes
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[brw(pad_size_to = MESSAGE_MAX_LENGTH)]
    pub message: SeString,
}
