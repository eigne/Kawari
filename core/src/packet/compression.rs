use binrw::binrw;
use std::io::Cursor;

use binrw::{BinRead, BinResult};

use crate::packet::{PacketHeader, PacketSegment};

use super::{ReadWriteIpcSegment, SegmentData, parsing::ConnectionState};

/// Various types of compression available for packets.
#[binrw]
#[brw(repr = u8)]
#[derive(Debug, PartialEq, Default)]
pub enum CompressionType {
    /// The packet data is uncompressed.
    #[default]
    Uncompressed = 0,
    /// The packet data is compressed with ZLib.
    ZLib = 1,
    /// The packet data is compressed with Oodle.
    Oodle = 2,
}

#[binrw::parser(reader, endian)]
pub(crate) fn decompress<T: ReadWriteIpcSegment>(
    header: &PacketHeader,
    state: &mut ConnectionState,
) -> BinResult<Vec<PacketSegment<T>>> {
    let mut segments: Vec<PacketSegment<T>> = Vec::with_capacity(header.segment_count as usize);

    let size = header.size as usize - std::mem::size_of::<PacketHeader>();

    let mut data = vec![0; size];
    reader.read_exact(&mut data)?;

    let data = match header.compression_type {
        CompressionType::Uncompressed => data,
        CompressionType::ZLib => unimplemented!(),
        CompressionType::Oodle => {
            let ConnectionState::Zone {
                serverbound_oodle, ..
            } = state
            else {
                panic!(
                    "Unexpected connection type! It needs to be Zone when using Oodle compression."
                );
            };

            serverbound_oodle.decode(data, header.uncompressed_size)
        }
    };

    if header.compression_type == CompressionType::Oodle {
        assert_eq!(
            data.len(),
            header.uncompressed_size as usize,
            "Decompressed data does not match the expected length!"
        );
    }

    let mut cursor = Cursor::new(&data);

    for _ in 0..header.segment_count {
        let current_position = cursor.position();
        let segment: PacketSegment<T> = PacketSegment::read_options(&mut cursor, endian, (state,))?;

        let is_unknown = match &segment.data {
            SegmentData::Ipc(data) => data.get_name() == "Unknown",
            _ => false,
        };

        if !is_unknown {
            let new_position = cursor.position();
            let expected_size = segment.calc_size() as u64;
            let actual_size = new_position - current_position;

            if expected_size != actual_size {
                tracing::warn!(
                    "The segment {:#?} does not match the size in calc_size()! (expected {expected_size} got {actual_size})",
                    segment
                );
            }
        }

        segments.push(segment);
    }

    Ok(segments)
}

#[cfg(feature = "server")]
pub(crate) fn compress<T: ReadWriteIpcSegment>(
    state: &mut ConnectionState,
    compression_type: &CompressionType,
    segments: &[PacketSegment<T>],
) -> (Vec<u8>, usize) {
    use super::{IPC_HEADER_SIZE, scramble_packet};
    use binrw::BinWrite;

    let mut segments_buffer = Vec::new();
    for segment in segments {
        let mut buffer = Vec::new();

        // write to buffer
        {
            let old_size = buffer.len();

            {
                let mut cursor = Cursor::new(&mut buffer);
                segment.write_le_args(&mut cursor, (state,)).unwrap();
            }

            let is_unknown = match &segment.data {
                SegmentData::Ipc(data) => data.get_name() == "Unknown",
                _ => false,
            };

            if !is_unknown {
                let new_size = buffer.len();
                let written_len = new_size - old_size;

                let expected_size = segment.calc_size() as usize;
                let size_matches = expected_size == written_len;
                if !size_matches {
                    // This WILL break the client in unexpected ways (especially when using Oodle compression) and has to be fixed immediately.
                    tracing::warn!(
                        "{:#?} does not match the size that was actually written! (expected: {}, written: {})",
                        segment,
                        expected_size,
                        written_len
                    );
                    panic!();
                }
            }
        }

        // obsfucate if needed
        if let ConnectionState::Zone {
            scrambler_keys: Some(keys),
            ..
        } = state
            && let SegmentData::Ipc(data) = &segment.data
        {
            let opcode = data.get_opcode();
            let base_key = keys.get_base_key(opcode);
            let opcode_based_key = keys.get_opcode_based_key(opcode);

            scramble_packet(
                data.get_name(),
                base_key,
                opcode_based_key,
                &mut buffer[IPC_HEADER_SIZE as usize..],
            );
        }

        segments_buffer.append(&mut buffer);
    }

    let segments_buffer_len = segments_buffer.len();

    match compression_type {
        CompressionType::Uncompressed => (segments_buffer, segments_buffer_len),
        CompressionType::ZLib => unimplemented!(),
        CompressionType::Oodle => {
            let ConnectionState::Zone {
                clientbound_oodle, ..
            } = state
            else {
                panic!(
                    "Unexpected connection state, it needs to be Zone when using Oodle compression!"
                );
            };

            (
                clientbound_oodle.encode(segments_buffer),
                segments_buffer_len,
            )
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::ipc::zone::{AoeActionResult, ServerZoneIpcData, ServerZoneIpcSegment};
    use crate::packet::SegmentType;

    #[test]
    fn aoe_effect8_serializes_at_declared_wire_size() {
        let segment = PacketSegment {
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ServerZoneIpcSegment::new(
                ServerZoneIpcData::AoeEffect8(AoeActionResult {
                    action_id: 9620,
                    spell_id: 9620,
                    effect_count: 8,
                    ..Default::default()
                }),
            )),
            ..Default::default()
        };
        let expected = segment.calc_size() as usize;
        let (bytes, length) = compress(
            &mut ConnectionState::None,
            &CompressionType::Uncompressed,
            &[segment],
        );
        assert_eq!(expected, 668); // 16-byte segment + 16-byte IPC + 636-byte payload
        assert_eq!(length, expected);
        assert_eq!(bytes.len(), expected);
        assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize, expected);
    }
}
