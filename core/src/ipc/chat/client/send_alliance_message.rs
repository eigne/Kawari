use binrw::binrw;
use physis::sestring::SeString;

use crate::common::MESSAGE_MAX_LENGTH;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendAllianceMessage {
    #[brw(pad_before = 4)] // Seems to be empty/zeroes
    unk: u8, // Unknown, observed as 1

    #[brw(pad_after = 3)] // Seems to be empty/zeroes
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[brw(pad_size_to = MESSAGE_MAX_LENGTH)]
    pub message: SeString, // NOTE: This is a SeString due to the fact that SEString macros can appear in its contents.
}
