//! Tests for fragmented data block transport, against the text of TS004-2.0.0, the command
//! bytes ChirpStack's fragmentation package is tested with, and an independent rendering of
//! the parity matrix the specification's appendix describes.

use super::fragment::{
    data_block_int_key, parity_line, prbs23, BlockMicKey, Defragmenter, FragCommand, FragError,
    Fragmenter, SetupStatus,
};
use super::PackageVersion;
use crate::{Direction, LorawanError};

/// Reads a command and checks it writes back to the same bytes.
fn round_trip(direction: Direction, bytes: &[u8], expected: FragCommand<'_>) {
    let (read, taken) = FragCommand::parse(direction, bytes).expect("it reads");
    assert_eq!(read, expected, "reading {bytes:02x?}");
    assert_eq!(taken, bytes.len(), "the whole command was read");
    let mut out = [0u8; 32];
    let len = read.encode(&mut out).expect("it writes");
    assert_eq!(&out[..len], bytes, "writing {expected:?}");
}

#[test]
fn the_commands_match_the_bytes_chirpstack_tests_its_own_package_with() {
    round_trip(Direction::Downlink, &[0x00], FragCommand::PackageVersionReq);
    round_trip(
        Direction::Uplink,
        &[0x00, 0x03, 0x02],
        FragCommand::PackageVersionAns(PackageVersion {
            package: 3,
            version: 2,
        }),
    );
    round_trip(
        Direction::Downlink,
        &[0x01, 0x05],
        FragCommand::FragSessionStatusReq {
            frag_index: 2,
            all_participants: true,
        },
    );
    round_trip(
        Direction::Uplink,
        &[0x01, 0x05, 0x00, 0xC4, 0x80],
        FragCommand::FragSessionStatusAns {
            frag_index: 3,
            received: 1024,
            missing: 128,
            mic_error: false,
            memory_error: true,
            no_session: true,
        },
    );
    round_trip(
        Direction::Downlink,
        &[
            0x02, 0x31, 0x00, 0x04, 0x80, 0x0D, 0x40, 0x01, 0x02, 0x03, 0x04, 0x80, 0x00, 0x01,
            0x02, 0x03, 0x04,
        ],
        FragCommand::FragSessionSetupReq {
            frag_index: 3,
            mc_group_bit_mask: 0b0001,
            nb_frag: 1024,
            frag_size: 128,
            ack_reception: false,
            frag_algo: 1,
            block_ack_delay: 5,
            padding: 64,
            descriptor: [0x01, 0x02, 0x03, 0x04],
            session_cnt: 128,
            mic: [0x01, 0x02, 0x03, 0x04],
        },
    );
    round_trip(
        Direction::Uplink,
        &[0x02, 0x8B],
        FragCommand::FragSessionSetupAns(SetupStatus {
            unsupported_algorithm: true,
            not_enough_memory: true,
            unsupported_index: false,
            wrong_descriptor: true,
            session_replay: false,
            frag_index: 2,
        }),
    );
    round_trip(
        Direction::Downlink,
        &[0x03, 0x03],
        FragCommand::FragSessionDeleteReq { frag_index: 3 },
    );
    round_trip(
        Direction::Uplink,
        &[0x03, 0x07],
        FragCommand::FragSessionDeleteAns {
            frag_index: 3,
            no_session: true,
        },
    );
    round_trip(
        Direction::Uplink,
        &[0x04, 0x06],
        FragCommand::FragDataBlockReceivedReq {
            frag_index: 2,
            mic_error: true,
        },
    );
    round_trip(
        Direction::Downlink,
        &[0x04, 0x02],
        FragCommand::FragDataBlockReceivedAns { frag_index: 2 },
    );
    round_trip(
        Direction::Downlink,
        &[0x08, 0x00, 0x84, 0x01, 0x02, 0x03, 0x04],
        FragCommand::DataFragment {
            frag_index: 2,
            n: 1024,
            data: &[0x01, 0x02, 0x03, 0x04],
        },
    );
}

#[test]
fn a_refused_setup_says_which_bits_refused_it() {
    let accepted = SetupStatus {
        frag_index: 1,
        ..SetupStatus::default()
    };
    assert!(accepted.accepted());
    let refused = SetupStatus {
        session_replay: true,
        ..accepted
    };
    assert!(!refused.accepted());
    let mut out = [0u8; 4];
    let len = FragCommand::FragSessionSetupAns(refused)
        .encode(&mut out)
        .expect("it writes");
    assert_eq!(&out[..len], &[0x02, 0x50], "the replay bit and the index");
}

#[test]
fn the_sequence_behind_the_matrix_steps_as_the_specification_says() {
    // Feeding a one in shifts it out and folds it back at the top of the register.
    let mut x = 1;
    let mut chain = [0u32; 5];
    for slot in &mut chain {
        x = prbs23(x);
        *slot = x;
    }
    assert_eq!(
        chain,
        [4_194_304, 2_097_152, 1_048_576, 524_288, 262_144],
        "the first five steps"
    );
}

#[test]
fn a_parity_row_is_half_ones_and_matches_an_independent_rendering() {
    // Columns taken from a separate rendering of the same algorithm, which agrees with
    // Semtech's LoRa Basics Modem and ChirpStack on seeding by the coded index.
    let cases: [(u16, u16, &[usize]); 5] = [
        (1, 4, &[0, 2]),
        (3, 4, &[1, 3]),
        (1, 10, &[1, 2, 3, 5, 9]),
        (2, 10, &[0, 2, 4, 5, 9]),
        (3, 10, &[1, 3, 5, 6, 7]),
    ];
    for (coded, nb_frag, columns) in cases {
        let mut line = [0u8; 4];
        let ones = parity_line(coded, nb_frag, &mut line);
        assert_eq!(ones, usize::from(nb_frag / 2), "half the columns are ones");
        let got: Vec<usize> = (0..usize::from(nb_frag))
            .filter(|at| line[at / 8] & (1 << (at % 8)) != 0)
            .collect();
        assert_eq!(got.as_slice(), columns, "coded {coded} of {nb_frag}");
    }
}

#[test]
fn a_block_goes_across_whole_when_nothing_is_lost() {
    let block: [u8; 100] = core::array::from_fn(|i| (i * 31 + 7) as u8);
    let sender = Fragmenter::new(&block, 10).expect("a session");
    assert_eq!((sender.nb_frag(), sender.padding()), (10, 0));

    let mut store = [0u8; 100];
    let mut matrix = [0u8; 64];
    let mut receiver = Defragmenter::new(10, 10, &mut store, &mut matrix).expect("a session");
    let mut piece = [0u8; 10];
    for n in 1..=10 {
        sender.fragment(n, &mut piece).expect("a fragment");
        receiver.fragment(n, &piece).expect("it is taken");
    }
    assert!(receiver.done());
    assert_eq!(receiver.missing(), 0);
    assert_eq!(receiver.block(), block);
}

#[test]
fn a_padded_block_comes_back_with_its_padding_at_the_end() {
    let block = b"twenty-six bytes of payload";
    let sender = Fragmenter::new(block, 8).expect("a session");
    assert_eq!((sender.nb_frag(), sender.padding()), (4, 5));

    let mut store = [0u8; 32];
    let mut matrix = [0u8; 64];
    let mut receiver = Defragmenter::new(4, 8, &mut store, &mut matrix).expect("a session");
    let mut piece = [0u8; 8];
    for n in 1..=4 {
        sender.fragment(n, &mut piece).expect("a fragment");
        receiver.fragment(n, &piece).expect("it is taken");
    }
    assert!(receiver.done());
    assert_eq!(&receiver.block()[..block.len()], block);
    assert_eq!(
        &receiver.block()[block.len()..],
        &[0; 5],
        "the padding the setup names"
    );
}

/// Runs a whole session, dropping the fragments a caller names, and says how many it took.
fn session_with_losses(nb_frag: u16, frag_size: u8, drop: &[u16]) -> Option<usize> {
    let block: Vec<u8> = (0..usize::from(nb_frag) * usize::from(frag_size))
        .map(|i| (i * 37 + 11) as u8)
        .collect();
    let sender = Fragmenter::new(&block, frag_size).expect("a session");
    let mut store = [0u8; 4096];
    let mut matrix = [0u8; 4096];
    let mut receiver =
        Defragmenter::new(nb_frag, frag_size, &mut store, &mut matrix).expect("a session");

    let mut piece = [0u8; 64];
    for n in 1..=nb_frag * 3 {
        if drop.contains(&n) {
            continue;
        }
        sender
            .fragment(n, &mut piece[..usize::from(frag_size)])
            .expect("a fragment");
        if receiver
            .fragment(n, &piece[..usize::from(frag_size)])
            .expect("it is taken")
        {
            assert_eq!(receiver.block(), block.as_slice(), "the block came back");
            return Some(usize::from(n));
        }
    }
    None
}

#[test]
fn coded_fragments_stand_in_for_the_ones_that_were_lost() {
    // A tenth of a forty-fragment session is lost, which the coded fragments make up for.
    let took = session_with_losses(40, 16, &[3, 11, 19, 27]).expect("the block came back");
    assert!(
        took > 40,
        "it took coded fragments to finish, {took} of them in all"
    );
    assert!(took <= 52, "and not many: {took}");
}

#[test]
fn a_heavy_loss_still_rebuilds_the_block_given_enough_redundancy() {
    // Half of the uncoded fragments never arrive.
    let lost: Vec<u16> = (1..=32u16).filter(|n| n % 2 == 0).collect();
    let took = session_with_losses(32, 8, &lost).expect("the block came back");
    assert!(took > 32, "the coded fragments did the work, {took} in all");
}

#[test]
fn a_session_that_loses_more_than_there_is_room_for_says_so() {
    let block = [0xABu8; 64];
    let sender = Fragmenter::new(&block, 8).expect("a session");
    let mut store = [0u8; 64];
    // Room for one loss only.
    let mut matrix = [0u8; Defragmenter::matrix_len(8, 1)];
    let mut receiver = Defragmenter::new(8, 8, &mut store, &mut matrix).expect("a session");
    assert_eq!(receiver.max_lost(), 1);

    let mut piece = [0u8; 8];
    for n in [1u16, 2, 5, 6, 7, 8] {
        sender.fragment(n, &mut piece).expect("a fragment");
        receiver.fragment(n, &piece).expect("it is taken");
    }
    sender.fragment(9, &mut piece).expect("a coded fragment");
    assert_eq!(
        receiver.fragment(9, &piece),
        Err(FragError::Memory),
        "two were lost and there was room for one"
    );
}

#[test]
fn a_session_reports_what_it_is_still_waiting_for() {
    let block = [0x5Au8; 80];
    let sender = Fragmenter::new(&block, 8).expect("a session");
    let mut store = [0u8; 80];
    let mut matrix = [0u8; 256];
    let mut receiver = Defragmenter::new(10, 8, &mut store, &mut matrix).expect("a session");
    assert_eq!(receiver.missing(), 10, "nothing has arrived");

    let mut piece = [0u8; 8];
    for n in [1u16, 2, 3] {
        sender.fragment(n, &mut piece).expect("a fragment");
        receiver.fragment(n, &piece).expect("it is taken");
    }
    assert_eq!(receiver.missing(), 7);
    assert_eq!(receiver.received(), 3);

    for n in [5u16, 6, 7, 8, 9, 10] {
        sender.fragment(n, &mut piece).expect("a fragment");
        receiver.fragment(n, &piece).expect("it is taken");
    }
    assert_eq!(receiver.missing(), 1, "the fourth never came");
    assert!(!receiver.done());
}

#[test]
fn a_session_refuses_what_it_cannot_run() {
    let mut store = [0u8; 16];
    let mut matrix = [0u8; 64];
    assert_eq!(
        Defragmenter::new(0, 8, &mut store, &mut matrix).map(|_| ()),
        Err(FragError::Session),
        "a session of no fragments"
    );
    assert_eq!(
        Defragmenter::new(4, 0, &mut store, &mut matrix).map(|_| ()),
        Err(FragError::Session),
        "fragments of no size"
    );
    assert_eq!(
        Defragmenter::new(100, 8, &mut store, &mut matrix).map(|_| ()),
        Err(FragError::Storage),
        "a block that does not fit the storage"
    );
    assert_eq!(
        Fragmenter::new(&[0u8; 8], 0).map(|_| ()),
        Err(LorawanError::MalformedFrame)
    );
}

#[test]
fn the_block_code_is_taken_over_the_block_the_setup_described() {
    // The key is the device's own, derived once and used for nothing else.
    let key = data_block_int_key(&[0x2B; 16]);
    assert_ne!(key, [0x2B; 16]);

    let block: [u8; 48] = core::array::from_fn(|i| (i * 5 + 1) as u8);
    let signer = BlockMicKey::new(&key);
    let mut whole = signer.start(7, 2, [0xDE, 0xAD, 0xBE, 0xEF], block.len() as u32);
    whole.update(&block);
    let mic = whole.finish();

    // The same block handed over in pieces, as a device reassembles it, signs the same.
    let mut in_pieces = signer.start(7, 2, [0xDE, 0xAD, 0xBE, 0xEF], block.len() as u32);
    for chunk in block.chunks(7) {
        in_pieces.update(chunk);
    }
    assert_eq!(in_pieces.finish(), mic);

    // Any change to what the setup described changes the code.
    let mut other_session = signer.start(8, 2, [0xDE, 0xAD, 0xBE, 0xEF], block.len() as u32);
    other_session.update(&block);
    assert_ne!(other_session.finish(), mic);

    let mut other_descriptor = signer.start(7, 2, [0xDE, 0xAD, 0xBE, 0xEE], block.len() as u32);
    other_descriptor.update(&block);
    assert_ne!(other_descriptor.finish(), mic);
}
