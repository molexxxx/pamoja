//! Generated Node bindings for the LoRaWAN application layer packages: clock
//! synchronization TS003-2.0.0, fragmented data block transport TS004-2.0.0, remote multicast
//! setup TS005-2.0.0, and firmware management TS006-1.0.0.
//!
//! The key derivations and the parity matrix are functions of their arguments. What has to
//! remember something is a class: a clock synchronization package, a firmware manager, a
//! fragmentation session being put back together, and the code taken over a block as it
//! arrives.

use napi::bindgen_prelude::{Buffer, Error, Result, Status};
use napi_derive::napi;

use pamoja_lorawan::packages::clock::{ClockCommand, ClockSync, PORT as CLOCK_PORT};
use pamoja_lorawan::packages::firmware::{
    DeleteStatus, FirmwareCommand, FirmwareManager, Image, UpImageStatus, PORT as FIRMWARE_PORT,
};
use pamoja_lorawan::packages::fragment::{
    data_block_int_key, parity_line, prbs23, BlockMicKey, Defragmenter, FragCommand, Fragmenter,
    Progress, SetupStatus, MAX_FRAGMENTS, PORT as FRAGMENT_PORT,
};
use pamoja_lorawan::packages::multicast::{
    mc_app_s_key, mc_ke_key, mc_key, mc_nwk_s_key, mc_root_key_for, wrap_mc_key, McCommand,
    SessionStatus, PORT as MULTICAST_PORT,
};
use pamoja_lorawan::packages::PackageVersion;
use pamoja_lorawan::Direction;

/// The port clock synchronization is spoken on, TS003-2.0.0.
#[napi]
pub const LORAWAN_CLOCK_PORT: u8 = CLOCK_PORT;

/// The port fragmented data block transport is spoken on, TS004-2.0.0.
#[napi]
pub const LORAWAN_FRAGMENT_PORT: u8 = FRAGMENT_PORT;

/// The port remote multicast setup is spoken on, TS005-2.0.0.
#[napi]
pub const LORAWAN_MULTICAST_PORT: u8 = MULTICAST_PORT;

/// The port firmware management is spoken on, TS006-1.0.0.
#[napi]
pub const LORAWAN_FIRMWARE_PORT: u8 = FIRMWARE_PORT;

/// The most fragments one session carries.
#[napi]
pub const LORAWAN_MAX_FRAGMENTS: u16 = MAX_FRAGMENTS;

/// What a device makes of the firmware upgrade image it holds, TS006-1.0.0 table 10.
#[napi(string_enum)]
pub enum LorawanImageStatus {
    /// It is holding none.
    None,
    /// One is there, but it is corrupt or its signature does not verify.
    Corrupt,
    /// One is there and authentic, but it is not for this hardware.
    WrongHardware,
    /// One is there, and it can be installed.
    Valid,
}

/// Reads the image status the way Rust names it.
fn image_in(status: &LorawanImageStatus) -> UpImageStatus {
    match status {
        LorawanImageStatus::None => UpImageStatus::None,
        LorawanImageStatus::Corrupt => UpImageStatus::Corrupt,
        LorawanImageStatus::WrongHardware => UpImageStatus::WrongHardware,
        LorawanImageStatus::Valid => UpImageStatus::Valid,
    }
}

/// Describes the image status the way JavaScript names it.
fn image_out(status: UpImageStatus) -> LorawanImageStatus {
    match status {
        UpImageStatus::None => LorawanImageStatus::None,
        UpImageStatus::Corrupt => LorawanImageStatus::Corrupt,
        UpImageStatus::WrongHardware => LorawanImageStatus::WrongHardware,
        UpImageStatus::Valid => LorawanImageStatus::Valid,
    }
}

/// Reads a sixteen-byte key, or says which argument was the wrong length.
fn key(bytes: &Buffer, what: &str) -> Result<[u8; 16]> {
    <[u8; 16]>::try_from(bytes.as_ref()).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("{what} must be sixteen bytes, not {}", bytes.len()),
        )
    })
}

/// Derives a device's multicast root key, TS005-2.0.0 section 4.3.
///
/// `lorawan11` picks the scheme: LoRaWAN 1.0.x devices derive from their `GenAppKey`, and
/// 1.1 devices from their `AppKey` under another constant.
#[napi]
pub fn lorawan_mc_root_key(root_key: Buffer, lorawan11: Option<bool>) -> Result<Buffer> {
    let root = key(&root_key, "rootKey")?;
    Ok(mc_root_key_for(&root, lorawan11.unwrap_or(false))
        .to_vec()
        .into())
}

/// Derives the key a multicast group's key travels under, section 4.3.
#[napi]
pub fn lorawan_mc_ke_key(mc_root_key: Buffer) -> Result<Buffer> {
    let root = key(&mc_root_key, "mcRootKey")?;
    Ok(mc_ke_key(&root).to_vec().into())
}

/// Unwraps the group key a setup command carried, section 4.3.
#[napi]
pub fn lorawan_mc_key(mc_ke_key: Buffer, wrapped: Buffer) -> Result<Buffer> {
    let ke = key(&mc_ke_key, "mcKeKey")?;
    let wrapped = key(&wrapped, "wrapped")?;
    Ok(mc_key(&ke, &wrapped).to_vec().into())
}

/// Wraps a group key for a device, which is what a server does before sending it.
#[napi]
pub fn lorawan_wrap_mc_key(mc_ke_key: Buffer, mc_key: Buffer) -> Result<Buffer> {
    let ke = key(&mc_ke_key, "mcKeKey")?;
    let group = key(&mc_key, "mcKey")?;
    Ok(wrap_mc_key(&ke, &group).to_vec().into())
}

/// Derives the key that reads a multicast group's payloads, section 4.3.
#[napi]
pub fn lorawan_mc_app_s_key(mc_key: Buffer, mc_addr: u32) -> Result<Buffer> {
    let group = key(&mc_key, "mcKey")?;
    Ok(mc_app_s_key(&group, mc_addr).to_vec().into())
}

/// Derives the key that verifies a multicast group's frames, section 4.3.
#[napi]
pub fn lorawan_mc_nwk_s_key(mc_key: Buffer, mc_addr: u32) -> Result<Buffer> {
    let group = key(&mc_key, "mcKey")?;
    Ok(mc_nwk_s_key(&group, mc_addr).to_vec().into())
}

/// Derives the key that signs a data block, TS004-2.0.0 section 3.3.
#[napi]
pub fn lorawan_data_block_int_key(root_key: Buffer) -> Result<Buffer> {
    let root = key(&root_key, "rootKey")?;
    Ok(data_block_int_key(&root).to_vec().into())
}

/// Steps the pseudo-random sequence the parity matrix is drawn from, appendix A.1.
#[napi]
pub fn lorawan_frag_prbs23(x: u32) -> u32 {
    prbs23(x)
}

/// Lists the uncoded fragments a coded one is made of, appendix A.1.
///
/// `coded` counts from one past the uncoded fragments: a session's fragment `nbFrag + 1` is
/// coded fragment 1.
#[napi]
pub fn lorawan_frag_parity_line(coded: u16, nb_frag: u16) -> Result<Vec<u16>> {
    if nb_frag == 0 || nb_frag > MAX_FRAGMENTS {
        return Err(Error::new(
            Status::InvalidArg,
            "a session carries 1 to 16383 fragments",
        ));
    }
    let mut line = vec![0u8; usize::from(nb_frag).div_ceil(8)];
    parity_line(coded, nb_frag, &mut line);
    Ok((0..nb_frag)
        .filter(|at| line[usize::from(*at) / 8] & (1 << (at % 8)) != 0)
        .collect())
}

/// How a block is cut into fragments.
#[napi(object)]
pub struct LorawanFragSession {
    /// How many uncoded fragments the block takes.
    pub nb_frag: u16,
    /// How many bytes of padding the last one carries.
    pub padding: u8,
}

/// Says how many fragments a block takes, and how much padding the last one needs.
#[napi]
pub fn lorawan_frag_session(block_len: u32, frag_size: u8) -> Result<LorawanFragSession> {
    let block = vec![0u8; block_len as usize];
    let sender = Fragmenter::new(&block, frag_size)
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
    Ok(LorawanFragSession {
        nb_frag: sender.nb_frag(),
        padding: sender.padding(),
    })
}

/// Builds one fragment of a session out of a block held whole.
///
/// Up to `nbFrag` the fragment is a piece of the block; past that it is a coded fragment,
/// the exclusive-or of a pseudo-random half of the pieces.
#[napi]
pub fn lorawan_frag_fragment(block: Buffer, frag_size: u8, n: u16) -> Result<Buffer> {
    let sender = Fragmenter::new(block.as_ref(), frag_size)
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
    let mut out = vec![0u8; usize::from(frag_size)];
    sender
        .fragment(n, &mut out)
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
    Ok(out.into())
}

/// A fragmentation session being put back together, TS004-2.0.0 appendix A.2.
///
/// The uncoded fragments go straight into the block. A coded fragment is reduced against
/// everything already known and kept only if it says something new, so the working memory is
/// sized by the losses rather than by the block.
#[napi]
pub struct LorawanDefragmenter {
    block: Vec<u8>,
    matrix: Vec<u8>,
    nb_frag: u16,
    frag_size: u8,
    progress: Progress,
    missing: u16,
}

#[napi]
impl LorawanDefragmenter {
    /// Opens a session for a block of `nbFrag` fragments of `fragSize` bytes.
    ///
    /// `maxLost` is the most uncoded fragments to be able to solve for, which decides how
    /// much working memory the session takes.
    #[napi(constructor)]
    pub fn new(nb_frag: u16, frag_size: u8, max_lost: u16) -> Result<LorawanDefragmenter> {
        if nb_frag == 0 || nb_frag > MAX_FRAGMENTS || frag_size == 0 {
            return Err(Error::new(
                Status::InvalidArg,
                "the session is not one this build runs",
            ));
        }
        Ok(LorawanDefragmenter {
            block: vec![0u8; usize::from(nb_frag) * usize::from(frag_size)],
            matrix: vec![0u8; Defragmenter::matrix_len(nb_frag, max_lost.min(nb_frag))],
            nb_frag,
            frag_size,
            progress: Progress::default(),
            missing: nb_frag,
        })
    }

    /// Takes one fragment of the session, counting from one.
    ///
    /// Returns `true` once the block is whole.
    #[napi]
    pub fn fragment(&mut self, n: u16, data: Buffer) -> Result<bool> {
        let mut session = Defragmenter::resumed(
            self.nb_frag,
            self.frag_size,
            &mut self.block,
            &mut self.matrix,
            self.progress,
        )
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
        let outcome = session.fragment(n, data.as_ref());
        self.progress = session.progress();
        self.missing = session.missing();
        outcome.map_err(|error| Error::new(Status::GenericFailure, error.to_string()))
    }

    /// The block, as far as it has been put back together, padding and all.
    #[napi(getter)]
    pub fn block(&self) -> Buffer {
        self.block.clone().into()
    }

    /// Whether the whole block is there.
    #[napi(getter)]
    pub fn done(&self) -> bool {
        self.progress.done
    }

    /// How many fragments arrived, coded, uncoded and repeated.
    #[napi(getter)]
    pub fn received(&self) -> u16 {
        self.progress.received
    }

    /// How many uncoded fragments are still missing.
    #[napi(getter)]
    pub fn missing(&self) -> u16 {
        self.missing
    }
}

/// The code taken over a data block as it arrives, TS004-2.0.0 section 3.3.
#[napi]
pub struct LorawanBlockMic {
    key: [u8; 16],
    session_cnt: u16,
    frag_index: u8,
    descriptor: [u8; 4],
    block_len: u32,
    pieces: Vec<u8>,
}

#[napi]
impl LorawanBlockMic {
    /// Starts a code over one session's block.
    ///
    /// `dataBlockIntKey` is what `lorawanDataBlockIntKey` derived, and the rest are the
    /// fields the session setup carried.
    #[napi(constructor)]
    pub fn new(
        data_block_int_key: Buffer,
        session_cnt: u16,
        frag_index: u8,
        descriptor: Buffer,
        block_len: u32,
    ) -> Result<LorawanBlockMic> {
        let key = key(&data_block_int_key, "dataBlockIntKey")?;
        let descriptor = <[u8; 4]>::try_from(descriptor.as_ref()).map_err(|_| {
            Error::new(
                Status::InvalidArg,
                format!("descriptor must be four bytes, not {}", descriptor.len()),
            )
        })?;
        Ok(LorawanBlockMic {
            key,
            session_cnt,
            frag_index,
            descriptor,
            block_len,
            pieces: Vec::new(),
        })
    }

    /// Adds a piece of the block, in order, without any padding.
    #[napi]
    pub fn update(&mut self, data: Buffer) {
        self.pieces.extend_from_slice(data.as_ref());
    }

    /// Finishes the code, returning the four bytes a session setup carries.
    #[napi]
    pub fn finish(&self) -> Buffer {
        let signer = BlockMicKey::new(&self.key);
        let mut mic = signer.start(
            self.session_cnt,
            self.frag_index,
            self.descriptor,
            self.block_len,
        );
        mic.update(&self.pieces);
        mic.finish().to_vec().into()
    }
}

/// What a downlink on the clock port asked of a device.
#[napi(object)]
pub struct LorawanClockHeard {
    /// The seconds to add to the device's clock, where an answer carried one.
    pub correction: Option<i32>,
    /// Whether the correction was the largest the field carries, so another follows.
    pub more_correction: bool,
    /// How many requests a resynchronization command asked for.
    pub resync: Option<u8>,
    /// Whether the device now owes an answer.
    pub answer_due: bool,
}

/// The clock synchronization package running on a device, TS003-2.0.0.
#[napi]
pub struct LorawanClockSync {
    inner: ClockSync,
}

#[napi]
impl LorawanClockSync {
    /// Starts the package.
    ///
    /// `selfManaged` marks a device that keeps its own periodicity and answers a server that
    /// tries to set one with the not-supported bit.
    #[napi(constructor)]
    pub fn new(self_managed: Option<bool>) -> LorawanClockSync {
        LorawanClockSync {
            inner: if self_managed.unwrap_or(false) {
                ClockSync::self_managed()
            } else {
                ClockSync::new()
            },
        }
    }

    /// Builds the request that asks a server for a correction, section 3.2.
    #[napi]
    pub fn request(&mut self, device_time: u32, ans_required: Option<bool>) -> Result<Buffer> {
        let mut out = [0u8; 8];
        let len = self
            .inner
            .app_time_req(device_time, ans_required.unwrap_or(false), &mut out)
            .map_err(|error| Error::new(Status::GenericFailure, error.to_string()))?;
        Ok(out[..len].to_vec().into())
    }

    /// Reads a downlink on the clock port and acts on it.
    #[napi]
    pub fn heard(&mut self, payload: Buffer) -> Result<LorawanClockHeard> {
        let heard = self
            .inner
            .heard(payload.as_ref())
            .map_err(|error| Error::new(Status::GenericFailure, error.to_string()))?;
        Ok(LorawanClockHeard {
            correction: heard.correction,
            more_correction: heard.more_correction,
            resync: heard.resync,
            answer_due: heard.answer_due,
        })
    }

    /// Writes the answer the device owes, or an empty buffer when it owes none.
    #[napi]
    pub fn answer(&mut self, device_time: u32) -> Result<Buffer> {
        let mut out = [0u8; 8];
        let len = self
            .inner
            .answer(device_time, &mut out)
            .map_err(|error| Error::new(Status::GenericFailure, error.to_string()))?;
        Ok(out[..len].to_vec().into())
    }

    /// The token the next request will carry.
    #[napi(getter)]
    pub fn token(&self) -> u8 {
        self.inner.token()
    }

    /// The seconds between requests, as the server last set them.
    #[napi(getter)]
    pub fn period_s(&self) -> u32 {
        self.inner.period_s()
    }

    /// Whether the device owes its server an answer.
    #[napi(getter)]
    pub fn answer_due(&self) -> bool {
        self.inner.answer_due()
    }
}

/// The firmware management package running on a device, TS006-1.0.0.
#[napi]
pub struct LorawanFirmware {
    inner: FirmwareManager,
}

#[napi]
impl LorawanFirmware {
    /// Starts the package, reporting the versions the device was built with.
    #[napi(constructor)]
    pub fn new(firmware: u32, hardware: u32) -> LorawanFirmware {
        LorawanFirmware {
            inner: FirmwareManager::new(firmware, hardware),
        }
    }

    /// Says what firmware upgrade image the device is holding.
    #[napi]
    pub fn set_image(&mut self, status: LorawanImageStatus, version: Option<u32>) {
        self.inner.set_image(match status {
            LorawanImageStatus::None => None,
            LorawanImageStatus::Valid => Some(Image::valid(version.unwrap_or(0))),
            other => Some(Image::refused(image_in(&other))),
        });
    }

    /// Reads a downlink on the firmware port and writes the answers it calls for.
    ///
    /// `nowS` is what the device believes the time is, in seconds since the GPS epoch; a
    /// device that does not know refuses a reboot set for a moment in time.
    #[napi]
    pub fn heard(&mut self, payload: Buffer, now_s: Option<u32>) -> Result<Buffer> {
        let mut out = [0u8; 64];
        let len = self
            .inner
            .heard_at(payload.as_ref(), now_s, &mut out)
            .map_err(|error| Error::new(Status::GenericFailure, error.to_string()))?;
        Ok(out[..len].to_vec().into())
    }

    /// The moment the device is to reboot, where one was set as a time.
    #[napi(getter)]
    pub fn reboot_at_s(&self) -> Option<u32> {
        self.inner.reboot_at_s()
    }

    /// How long until it reboots, where one was set as a countdown.
    #[napi(getter)]
    pub fn reboot_in_s(&self) -> Option<u32> {
        self.inner.reboot_in_s()
    }

    /// Whether the device was told to reboot at once.
    #[napi(getter)]
    pub fn reboot_now(&self) -> bool {
        self.inner.reboot_now()
    }

    /// What the device would boot into, for an image it can install.
    #[napi(getter)]
    pub fn next_version(&self) -> Option<u32> {
        self.inner
            .image()
            .filter(|image| matches!(image.status, UpImageStatus::Valid))
            .map(|image| image.version)
    }

    /// What the device makes of the image it holds.
    #[napi(getter)]
    pub fn image_status(&self) -> LorawanImageStatus {
        image_out(
            self.inner
                .image()
                .map_or(UpImageStatus::None, |image| image.status),
        )
    }

    /// Forgets the programmed reboot, for a device that has carried it out.
    #[napi]
    pub fn rebooted(&mut self) {
        self.inner.rebooted();
    }
}

/// One command of an application layer package, whichever package it belongs to.
///
/// `port` says which package and `kind` names the command within it; together they decide
/// which of the other fields carry anything. The rest are absent.
#[napi(object)]
pub struct LorawanPackageCommand {
    /// Which package this command belongs to, as its port.
    pub port: u8,
    /// Which command this is, as a name.
    pub kind: String,
    /// Which way it travels.
    pub uplink: bool,
    /// The package identifier a version answer carries.
    pub package: Option<u8>,
    /// The package version it implements.
    pub version: Option<u8>,
    /// A device's own clock, in seconds since the GPS epoch.
    pub device_time: Option<u32>,
    /// The seconds to add to a device's clock.
    pub time_correction: Option<i32>,
    /// The token that pairs a clock answer with its request.
    pub token: Option<u8>,
    /// Whether a clock request must be answered.
    pub ans_required: Option<bool>,
    /// The coded period between clock requests.
    pub period: Option<u8>,
    /// Whether a device manages its own clock periodicity.
    pub not_supported: Option<bool>,
    /// How many requests a resynchronization command asks for.
    pub transmissions: Option<u8>,
    /// The firmware a device reports running.
    pub firmware: Option<u32>,
    /// The hardware it runs on.
    pub hardware: Option<u32>,
    /// The moment or the delay a reboot is set for.
    pub reboot: Option<u32>,
    /// What a device makes of the upgrade image it holds.
    pub image_status: Option<LorawanImageStatus>,
    /// The version it would run once that image is installed.
    pub next_version: Option<u32>,
    /// The version a delete command names.
    pub delete_version: Option<u32>,
    /// Whether a device holds no valid image.
    pub no_valid_image: Option<bool>,
    /// Whether the version named is not the one held.
    pub invalid_version: Option<bool>,
    /// Which fragmentation session, 0 to 3.
    pub frag_index: Option<u8>,
    /// Which multicast groups may feed it, a bit for each.
    pub mc_group_bit_mask: Option<u8>,
    /// How many uncoded fragments a block was cut into.
    pub nb_frag: Option<u16>,
    /// How many bytes each fragment carries.
    pub frag_size: Option<u8>,
    /// Whether a device reports the block once it has it.
    pub ack_reception: Option<bool>,
    /// Which fragmentation algorithm to run.
    pub frag_algo: Option<u8>,
    /// The coded spread of the delay before a device answers.
    pub block_ack_delay: Option<u8>,
    /// How many bytes of padding the last fragment carries.
    pub padding: Option<u8>,
    /// The four bytes a server describes a block with.
    pub descriptor: Option<Buffer>,
    /// The session counter, which must rise for each new block.
    pub session_cnt: Option<u16>,
    /// The code over the block a device checks once it has it all.
    pub mic: Option<Buffer>,
    /// How many fragments arrived, coded, uncoded and repeated.
    pub received: Option<u16>,
    /// How many uncoded fragments are still missing.
    pub missing: Option<u8>,
    /// Whether the block's code did not check out.
    pub mic_error: Option<bool>,
    /// Whether a session ran out of memory to defragment with.
    pub memory_error: Option<bool>,
    /// Whether the session or group named does not exist on the device.
    pub no_session: Option<bool>,
    /// Whether the setup named an algorithm the device does not run.
    pub unsupported_algorithm: Option<bool>,
    /// Whether the setup named an index the device does not keep.
    pub unsupported_index: Option<bool>,
    /// Whether the descriptor is not one the device accepts.
    pub wrong_descriptor: Option<bool>,
    /// Whether the session counter repeats one already used.
    pub session_replay: Option<bool>,
    /// Whether every device answers a status request.
    pub all_participants: Option<bool>,
    /// Which fragment of a session a data fragment carries, counting from one.
    pub fragment_n: Option<u16>,
    /// The bytes a data fragment carries.
    pub data: Option<Buffer>,
    /// Which multicast group, 0 to 3.
    pub mc_group_id: Option<u8>,
    /// The address a group answers to.
    pub mc_addr: Option<u32>,
    /// A group's key, wrapped under the device's key encryption key.
    pub mc_key_encrypted: Option<Buffer>,
    /// The first frame counter a device accepts from a group.
    pub min_mc_fcnt: Option<u32>,
    /// The last one, which ends the group's life.
    pub max_mc_fcnt: Option<u32>,
    /// Which groups a status request or answer covers, a bit for each.
    pub group_mask: Option<u8>,
    /// How many groups a device holds in all.
    pub nb_total_groups: Option<u8>,
    /// Whether a device holds no group by the identifier named.
    pub id_error: Option<bool>,
    /// When a multicast window opens, in seconds since the GPS epoch.
    pub session_time: Option<u32>,
    /// How long it lasts at most, coded.
    pub time_out: Option<u8>,
    /// How often a device opens a ping slot inside a Class B window.
    pub periodicity: Option<u8>,
    /// Where a group listens, in hertz.
    pub dl_frequency_hz: Option<u32>,
    /// The data rate it listens at.
    pub data_rate: Option<u8>,
    /// How many seconds until a window opens.
    pub time_to_start: Option<u32>,
    /// Whether the data rate named is not one the device has.
    pub dr_error: Option<bool>,
    /// Whether the frequency named is not one it can use.
    pub freq_error: Option<bool>,
    /// Whether the window was to start at a time already past.
    pub start_missed: Option<bool>,
}

impl LorawanPackageCommand {
    /// An empty command of one package, which the readers below fill in.
    fn empty(port: u8, kind: &str, uplink: bool) -> LorawanPackageCommand {
        LorawanPackageCommand {
            port,
            kind: kind.to_owned(),
            uplink,
            package: None,
            version: None,
            device_time: None,
            time_correction: None,
            token: None,
            ans_required: None,
            period: None,
            not_supported: None,
            transmissions: None,
            firmware: None,
            hardware: None,
            reboot: None,
            image_status: None,
            next_version: None,
            delete_version: None,
            no_valid_image: None,
            invalid_version: None,
            frag_index: None,
            mc_group_bit_mask: None,
            nb_frag: None,
            frag_size: None,
            ack_reception: None,
            frag_algo: None,
            block_ack_delay: None,
            padding: None,
            descriptor: None,
            session_cnt: None,
            mic: None,
            received: None,
            missing: None,
            mic_error: None,
            memory_error: None,
            no_session: None,
            unsupported_algorithm: None,
            unsupported_index: None,
            wrong_descriptor: None,
            session_replay: None,
            all_participants: None,
            fragment_n: None,
            data: None,
            mc_group_id: None,
            mc_addr: None,
            mc_key_encrypted: None,
            min_mc_fcnt: None,
            max_mc_fcnt: None,
            group_mask: None,
            nb_total_groups: None,
            id_error: None,
            session_time: None,
            time_out: None,
            periodicity: None,
            dl_frequency_hz: None,
            data_rate: None,
            time_to_start: None,
            dr_error: None,
            freq_error: None,
            start_missed: None,
        }
    }
}

/// Reads one command of an application layer package.
///
/// `uplink` says which way the frame carrying it traveled, because the same identifier names
/// a different command in each direction. A data fragment takes the whole message, as
/// TS004-2.0.0 section 3 asks, and its bytes come back on `data`.
#[napi]
pub fn lorawan_package_parse(
    port: u8,
    uplink: bool,
    payload: Buffer,
) -> Result<LorawanPackageCommand> {
    let direction = if uplink {
        Direction::Uplink
    } else {
        Direction::Downlink
    };
    let bytes = payload.as_ref();
    let refused =
        |error: pamoja_lorawan::LorawanError| Error::new(Status::GenericFailure, error.to_string());
    match port {
        CLOCK_PORT => ClockCommand::parse(direction, bytes)
            .map(|(command, _)| clock_out(command))
            .map_err(refused),
        FIRMWARE_PORT => FirmwareCommand::parse(direction, bytes)
            .map(|(command, _)| firmware_out(command))
            .map_err(refused),
        FRAGMENT_PORT => FragCommand::parse(direction, bytes)
            .map(|(command, _)| frag_out(command))
            .map_err(refused),
        MULTICAST_PORT => McCommand::parse(direction, bytes)
            .map(|(command, _)| mc_out(command))
            .map_err(refused),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("port {other} is not an application layer package"),
        )),
    }
}

/// Reads every command in one message, stopping at an identifier the package does not define.
#[napi]
pub fn lorawan_package_parse_all(
    port: u8,
    uplink: bool,
    payload: Buffer,
) -> Result<Vec<LorawanPackageCommand>> {
    let direction = if uplink {
        Direction::Uplink
    } else {
        Direction::Downlink
    };
    let bytes = payload.as_ref();
    let mut at = 0;
    let mut read = Vec::new();
    while at < bytes.len() {
        let taken = match port {
            CLOCK_PORT => ClockCommand::parse(direction, &bytes[at..]).map(|(command, taken)| {
                read.push(clock_out(command));
                taken
            }),
            FIRMWARE_PORT => {
                FirmwareCommand::parse(direction, &bytes[at..]).map(|(command, taken)| {
                    read.push(firmware_out(command));
                    taken
                })
            }
            FRAGMENT_PORT => FragCommand::parse(direction, &bytes[at..]).map(|(command, taken)| {
                read.push(frag_out(command));
                taken
            }),
            MULTICAST_PORT => McCommand::parse(direction, &bytes[at..]).map(|(command, taken)| {
                read.push(mc_out(command));
                taken
            }),
            other => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("port {other} is not an application layer package"),
                ))
            }
        };
        match taken {
            Ok(taken) => at += taken,
            Err(_) => break,
        }
    }
    Ok(read)
}

/// Reads one group record of a multicast status answer, TS005-2.0.0 section 4.2.
#[napi]
pub fn lorawan_package_status_item(payload: Buffer) -> Result<LorawanPackageCommand> {
    McCommand::status_item(payload.as_ref())
        .map(|(command, _)| mc_out(command))
        .map_err(|error| Error::new(Status::GenericFailure, error.to_string()))
}

/// Writes one command of an application layer package.
#[napi]
pub fn lorawan_package_encode(command: LorawanPackageCommand) -> Result<Buffer> {
    let room = command.data.as_ref().map_or(0, |data| data.len()) + 64;
    let mut out = vec![0u8; room];
    let refused =
        |error: pamoja_lorawan::LorawanError| Error::new(Status::GenericFailure, error.to_string());
    let unknown = || {
        Error::new(
            Status::InvalidArg,
            format!(
                "{} on port {} is not a command this build writes",
                command.kind, command.port
            ),
        )
    };
    let written = match command.port {
        CLOCK_PORT => clock_in(&command).ok_or_else(unknown)?.encode(&mut out),
        FIRMWARE_PORT => firmware_in(&command).ok_or_else(unknown)?.encode(&mut out),
        FRAGMENT_PORT => {
            let data = command.data.as_ref().map(|data| data.to_vec());
            frag_in(&command, data.as_deref().unwrap_or(&[]))
                .ok_or_else(unknown)?
                .encode(&mut out)
        }
        MULTICAST_PORT => mc_in(&command).ok_or_else(unknown)?.encode(&mut out),
        other => {
            return Err(Error::new(
                Status::InvalidArg,
                format!("port {other} is not an application layer package"),
            ))
        }
    };
    written
        .map(|len| out[..len].to_vec().into())
        .map_err(refused)
}

/// Reads four bytes from a field that must carry exactly that many.
fn four(bytes: &Option<Buffer>) -> Option<[u8; 4]> {
    <[u8; 4]>::try_from(bytes.as_ref()?.as_ref()).ok()
}

/// Reads sixteen bytes from a field that must carry exactly that many.
fn sixteen(bytes: &Option<Buffer>) -> Option<[u8; 16]> {
    <[u8; 16]>::try_from(bytes.as_ref()?.as_ref()).ok()
}

/// Describes a clock synchronization command the way JavaScript holds it.
fn clock_out(command: ClockCommand) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let mut flat = match command {
        ClockCommand::PackageVersionReq => {
            LorawanPackageCommand::empty(CLOCK_PORT, "packageVersionReq", uplink)
        }
        ClockCommand::PackageVersionAns(_) => {
            LorawanPackageCommand::empty(CLOCK_PORT, "packageVersionAns", uplink)
        }
        ClockCommand::AppTimeReq { .. } => {
            LorawanPackageCommand::empty(CLOCK_PORT, "appTimeReq", uplink)
        }
        ClockCommand::AppTimeAns { .. } => {
            LorawanPackageCommand::empty(CLOCK_PORT, "appTimeAns", uplink)
        }
        ClockCommand::DeviceAppTimePeriodicityReq { .. } => {
            LorawanPackageCommand::empty(CLOCK_PORT, "deviceAppTimePeriodicityReq", uplink)
        }
        ClockCommand::DeviceAppTimePeriodicityAns { .. } => {
            LorawanPackageCommand::empty(CLOCK_PORT, "deviceAppTimePeriodicityAns", uplink)
        }
        ClockCommand::ForceDeviceResyncCmd { .. } => {
            LorawanPackageCommand::empty(CLOCK_PORT, "forceDeviceResyncCmd", uplink)
        }
    };
    match command {
        ClockCommand::PackageVersionAns(version) => {
            flat.package = Some(version.package);
            flat.version = Some(version.version);
        }
        ClockCommand::AppTimeReq {
            device_time,
            ans_required,
            token,
        } => {
            flat.device_time = Some(device_time);
            flat.ans_required = Some(ans_required);
            flat.token = Some(token);
        }
        ClockCommand::AppTimeAns {
            time_correction,
            token,
        } => {
            flat.time_correction = Some(time_correction);
            flat.token = Some(token);
        }
        ClockCommand::DeviceAppTimePeriodicityReq { period } => flat.period = Some(period),
        ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported,
            device_time,
        } => {
            flat.not_supported = Some(not_supported);
            flat.device_time = Some(device_time);
        }
        ClockCommand::ForceDeviceResyncCmd { transmissions } => {
            flat.transmissions = Some(transmissions);
        }
        ClockCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a clock synchronization command out of the record JavaScript holds.
fn clock_in(flat: &LorawanPackageCommand) -> Option<ClockCommand> {
    Some(match flat.kind.as_str() {
        "packageVersionReq" => ClockCommand::PackageVersionReq,
        "packageVersionAns" => ClockCommand::PackageVersionAns(PackageVersion {
            package: flat.package?,
            version: flat.version?,
        }),
        "appTimeReq" => ClockCommand::AppTimeReq {
            device_time: flat.device_time?,
            ans_required: flat.ans_required.unwrap_or(false),
            token: flat.token.unwrap_or(0),
        },
        "appTimeAns" => ClockCommand::AppTimeAns {
            time_correction: flat.time_correction?,
            token: flat.token.unwrap_or(0),
        },
        "deviceAppTimePeriodicityReq" => ClockCommand::DeviceAppTimePeriodicityReq {
            period: flat.period?,
        },
        "deviceAppTimePeriodicityAns" => ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported: flat.not_supported.unwrap_or(false),
            device_time: flat.device_time?,
        },
        "forceDeviceResyncCmd" => ClockCommand::ForceDeviceResyncCmd {
            transmissions: flat.transmissions?,
        },
        _ => return None,
    })
}

/// Describes a firmware management command the way JavaScript holds it.
fn firmware_out(command: FirmwareCommand) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        FirmwareCommand::PackageVersionReq => "packageVersionReq",
        FirmwareCommand::PackageVersionAns(_) => "packageVersionAns",
        FirmwareCommand::DevVersionReq => "devVersionReq",
        FirmwareCommand::DevVersionAns { .. } => "devVersionAns",
        FirmwareCommand::DevRebootTimeReq { .. } => "devRebootTimeReq",
        FirmwareCommand::DevRebootTimeAns { .. } => "devRebootTimeAns",
        FirmwareCommand::DevRebootCountdownReq { .. } => "devRebootCountdownReq",
        FirmwareCommand::DevRebootCountdownAns { .. } => "devRebootCountdownAns",
        FirmwareCommand::DevUpgradeImageReq => "devUpgradeImageReq",
        FirmwareCommand::DevUpgradeImageAns { .. } => "devUpgradeImageAns",
        FirmwareCommand::DevDeleteImageReq { .. } => "devDeleteImageReq",
        FirmwareCommand::DevDeleteImageAns(_) => "devDeleteImageAns",
    };
    let mut flat = LorawanPackageCommand::empty(FIRMWARE_PORT, name, uplink);
    match command {
        FirmwareCommand::PackageVersionAns(version) => {
            flat.package = Some(version.package);
            flat.version = Some(version.version);
        }
        FirmwareCommand::DevVersionAns { firmware, hardware } => {
            flat.firmware = Some(firmware);
            flat.hardware = Some(hardware);
        }
        FirmwareCommand::DevRebootTimeReq { reboot_time }
        | FirmwareCommand::DevRebootTimeAns { reboot_time } => flat.reboot = Some(reboot_time),
        FirmwareCommand::DevRebootCountdownReq { countdown }
        | FirmwareCommand::DevRebootCountdownAns { countdown } => flat.reboot = Some(countdown),
        FirmwareCommand::DevUpgradeImageAns {
            status,
            next_version,
        } => {
            flat.image_status = Some(image_out(status));
            flat.next_version = next_version;
        }
        FirmwareCommand::DevDeleteImageReq { version } => flat.delete_version = Some(version),
        FirmwareCommand::DevDeleteImageAns(status) => {
            flat.no_valid_image = Some(status.no_valid_image);
            flat.invalid_version = Some(status.invalid_version);
        }
        FirmwareCommand::PackageVersionReq
        | FirmwareCommand::DevVersionReq
        | FirmwareCommand::DevUpgradeImageReq => {}
    }
    flat
}

/// Reads a firmware management command out of the record JavaScript holds.
fn firmware_in(flat: &LorawanPackageCommand) -> Option<FirmwareCommand> {
    Some(match flat.kind.as_str() {
        "packageVersionReq" => FirmwareCommand::PackageVersionReq,
        "packageVersionAns" => FirmwareCommand::PackageVersionAns(PackageVersion {
            package: flat.package?,
            version: flat.version?,
        }),
        "devVersionReq" => FirmwareCommand::DevVersionReq,
        "devVersionAns" => FirmwareCommand::DevVersionAns {
            firmware: flat.firmware?,
            hardware: flat.hardware?,
        },
        "devRebootTimeReq" => FirmwareCommand::DevRebootTimeReq {
            reboot_time: flat.reboot?,
        },
        "devRebootTimeAns" => FirmwareCommand::DevRebootTimeAns {
            reboot_time: flat.reboot?,
        },
        "devRebootCountdownReq" => FirmwareCommand::DevRebootCountdownReq {
            countdown: flat.reboot?,
        },
        "devRebootCountdownAns" => FirmwareCommand::DevRebootCountdownAns {
            countdown: flat.reboot?,
        },
        "devUpgradeImageReq" => FirmwareCommand::DevUpgradeImageReq,
        "devUpgradeImageAns" => FirmwareCommand::DevUpgradeImageAns {
            status: image_in(flat.image_status.as_ref()?),
            next_version: flat.next_version,
        },
        "devDeleteImageReq" => FirmwareCommand::DevDeleteImageReq {
            version: flat.delete_version?,
        },
        "devDeleteImageAns" => FirmwareCommand::DevDeleteImageAns(DeleteStatus {
            no_valid_image: flat.no_valid_image.unwrap_or(false),
            invalid_version: flat.invalid_version.unwrap_or(false),
        }),
        _ => return None,
    })
}

/// Describes a fragmentation command the way JavaScript holds it.
fn frag_out(command: FragCommand<'_>) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        FragCommand::PackageVersionReq => "packageVersionReq",
        FragCommand::PackageVersionAns(_) => "packageVersionAns",
        FragCommand::FragSessionStatusReq { .. } => "fragSessionStatusReq",
        FragCommand::FragSessionStatusAns { .. } => "fragSessionStatusAns",
        FragCommand::FragSessionSetupReq { .. } => "fragSessionSetupReq",
        FragCommand::FragSessionSetupAns(_) => "fragSessionSetupAns",
        FragCommand::FragSessionDeleteReq { .. } => "fragSessionDeleteReq",
        FragCommand::FragSessionDeleteAns { .. } => "fragSessionDeleteAns",
        FragCommand::FragDataBlockReceivedReq { .. } => "fragDataBlockReceivedReq",
        FragCommand::FragDataBlockReceivedAns { .. } => "fragDataBlockReceivedAns",
        FragCommand::DataFragment { .. } => "dataFragment",
    };
    let mut flat = LorawanPackageCommand::empty(FRAGMENT_PORT, name, uplink);
    match command {
        FragCommand::PackageVersionAns(version) => {
            flat.package = Some(version.package);
            flat.version = Some(version.version);
        }
        FragCommand::FragSessionStatusReq {
            frag_index,
            all_participants,
        } => {
            flat.frag_index = Some(frag_index);
            flat.all_participants = Some(all_participants);
        }
        FragCommand::FragSessionStatusAns {
            frag_index,
            received,
            missing,
            mic_error,
            memory_error,
            no_session,
        } => {
            flat.frag_index = Some(frag_index);
            flat.received = Some(received);
            flat.missing = Some(missing);
            flat.mic_error = Some(mic_error);
            flat.memory_error = Some(memory_error);
            flat.no_session = Some(no_session);
        }
        FragCommand::FragSessionSetupReq {
            frag_index,
            mc_group_bit_mask,
            nb_frag,
            frag_size,
            ack_reception,
            frag_algo,
            block_ack_delay,
            padding,
            descriptor,
            session_cnt,
            mic,
        } => {
            flat.frag_index = Some(frag_index);
            flat.mc_group_bit_mask = Some(mc_group_bit_mask);
            flat.nb_frag = Some(nb_frag);
            flat.frag_size = Some(frag_size);
            flat.ack_reception = Some(ack_reception);
            flat.frag_algo = Some(frag_algo);
            flat.block_ack_delay = Some(block_ack_delay);
            flat.padding = Some(padding);
            flat.descriptor = Some(descriptor.to_vec().into());
            flat.session_cnt = Some(session_cnt);
            flat.mic = Some(mic.to_vec().into());
        }
        FragCommand::FragSessionSetupAns(status) => {
            flat.frag_index = Some(status.frag_index);
            flat.unsupported_algorithm = Some(status.unsupported_algorithm);
            flat.memory_error = Some(status.not_enough_memory);
            flat.unsupported_index = Some(status.unsupported_index);
            flat.wrong_descriptor = Some(status.wrong_descriptor);
            flat.session_replay = Some(status.session_replay);
        }
        FragCommand::FragSessionDeleteReq { frag_index } => flat.frag_index = Some(frag_index),
        FragCommand::FragSessionDeleteAns {
            frag_index,
            no_session,
        } => {
            flat.frag_index = Some(frag_index);
            flat.no_session = Some(no_session);
        }
        FragCommand::FragDataBlockReceivedReq {
            frag_index,
            mic_error,
        } => {
            flat.frag_index = Some(frag_index);
            flat.mic_error = Some(mic_error);
        }
        FragCommand::FragDataBlockReceivedAns { frag_index } => {
            flat.frag_index = Some(frag_index);
        }
        FragCommand::DataFragment {
            frag_index,
            n,
            data,
        } => {
            flat.frag_index = Some(frag_index);
            flat.fragment_n = Some(n);
            flat.data = Some(data.to_vec().into());
        }
        FragCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a fragmentation command out of the record JavaScript holds.
fn frag_in<'a>(flat: &LorawanPackageCommand, data: &'a [u8]) -> Option<FragCommand<'a>> {
    Some(match flat.kind.as_str() {
        "packageVersionReq" => FragCommand::PackageVersionReq,
        "packageVersionAns" => FragCommand::PackageVersionAns(PackageVersion {
            package: flat.package?,
            version: flat.version?,
        }),
        "fragSessionStatusReq" => FragCommand::FragSessionStatusReq {
            frag_index: flat.frag_index?,
            all_participants: flat.all_participants.unwrap_or(false),
        },
        "fragSessionStatusAns" => FragCommand::FragSessionStatusAns {
            frag_index: flat.frag_index?,
            received: flat.received?,
            missing: flat.missing.unwrap_or(0),
            mic_error: flat.mic_error.unwrap_or(false),
            memory_error: flat.memory_error.unwrap_or(false),
            no_session: flat.no_session.unwrap_or(false),
        },
        "fragSessionSetupReq" => FragCommand::FragSessionSetupReq {
            frag_index: flat.frag_index?,
            mc_group_bit_mask: flat.mc_group_bit_mask.unwrap_or(0),
            nb_frag: flat.nb_frag?,
            frag_size: flat.frag_size?,
            ack_reception: flat.ack_reception.unwrap_or(false),
            frag_algo: flat.frag_algo.unwrap_or(0),
            block_ack_delay: flat.block_ack_delay.unwrap_or(0),
            padding: flat.padding.unwrap_or(0),
            descriptor: four(&flat.descriptor)?,
            session_cnt: flat.session_cnt.unwrap_or(0),
            mic: four(&flat.mic)?,
        },
        "fragSessionSetupAns" => FragCommand::FragSessionSetupAns(SetupStatus {
            unsupported_algorithm: flat.unsupported_algorithm.unwrap_or(false),
            not_enough_memory: flat.memory_error.unwrap_or(false),
            unsupported_index: flat.unsupported_index.unwrap_or(false),
            wrong_descriptor: flat.wrong_descriptor.unwrap_or(false),
            session_replay: flat.session_replay.unwrap_or(false),
            frag_index: flat.frag_index?,
        }),
        "fragSessionDeleteReq" => FragCommand::FragSessionDeleteReq {
            frag_index: flat.frag_index?,
        },
        "fragSessionDeleteAns" => FragCommand::FragSessionDeleteAns {
            frag_index: flat.frag_index?,
            no_session: flat.no_session.unwrap_or(false),
        },
        "fragDataBlockReceivedReq" => FragCommand::FragDataBlockReceivedReq {
            frag_index: flat.frag_index?,
            mic_error: flat.mic_error.unwrap_or(false),
        },
        "fragDataBlockReceivedAns" => FragCommand::FragDataBlockReceivedAns {
            frag_index: flat.frag_index?,
        },
        "dataFragment" => FragCommand::DataFragment {
            frag_index: flat.frag_index?,
            n: flat.fragment_n?,
            data,
        },
        _ => return None,
    })
}

/// Describes a multicast setup command the way JavaScript holds it.
fn mc_out(command: McCommand) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        McCommand::PackageVersionReq => "packageVersionReq",
        McCommand::PackageVersionAns(_) => "packageVersionAns",
        McCommand::McGroupStatusReq { .. } => "mcGroupStatusReq",
        McCommand::McGroupStatusAns { .. } => "mcGroupStatusAns",
        McCommand::McGroupStatusItem { .. } => "mcGroupStatusItem",
        McCommand::McGroupSetupReq { .. } => "mcGroupSetupReq",
        McCommand::McGroupSetupAns { .. } => "mcGroupSetupAns",
        McCommand::McGroupDeleteReq { .. } => "mcGroupDeleteReq",
        McCommand::McGroupDeleteAns { .. } => "mcGroupDeleteAns",
        McCommand::McClassCSessionReq { .. } => "mcClassCSessionReq",
        McCommand::McClassCSessionAns { .. } => "mcClassCSessionAns",
        McCommand::McClassBSessionReq { .. } => "mcClassBSessionReq",
        McCommand::McClassBSessionAns { .. } => "mcClassBSessionAns",
    };
    let mut flat = LorawanPackageCommand::empty(MULTICAST_PORT, name, uplink);
    match command {
        McCommand::PackageVersionAns(version) => {
            flat.package = Some(version.package);
            flat.version = Some(version.version);
        }
        McCommand::McGroupStatusReq { req_group_mask } => flat.group_mask = Some(req_group_mask),
        McCommand::McGroupStatusAns {
            ans_group_mask,
            nb_total_groups,
        } => {
            flat.group_mask = Some(ans_group_mask);
            flat.nb_total_groups = Some(nb_total_groups);
        }
        McCommand::McGroupStatusItem {
            mc_group_id,
            mc_addr,
        } => {
            flat.mc_group_id = Some(mc_group_id);
            flat.mc_addr = Some(mc_addr);
        }
        McCommand::McGroupSetupReq {
            mc_group_id,
            mc_addr,
            mc_key_encrypted,
            min_mc_fcnt,
            max_mc_fcnt,
        } => {
            flat.mc_group_id = Some(mc_group_id);
            flat.mc_addr = Some(mc_addr);
            flat.mc_key_encrypted = Some(mc_key_encrypted.to_vec().into());
            flat.min_mc_fcnt = Some(min_mc_fcnt);
            flat.max_mc_fcnt = Some(max_mc_fcnt);
        }
        McCommand::McGroupSetupAns {
            mc_group_id,
            id_error,
        } => {
            flat.mc_group_id = Some(mc_group_id);
            flat.id_error = Some(id_error);
        }
        McCommand::McGroupDeleteReq { mc_group_id } => flat.mc_group_id = Some(mc_group_id),
        McCommand::McGroupDeleteAns {
            mc_group_id,
            group_undefined,
        } => {
            flat.mc_group_id = Some(mc_group_id);
            flat.no_session = Some(group_undefined);
        }
        McCommand::McClassCSessionReq {
            mc_group_id,
            session_time,
            time_out,
            dl_frequency_hz,
            data_rate,
        } => {
            flat.mc_group_id = Some(mc_group_id);
            flat.session_time = Some(session_time);
            flat.time_out = Some(time_out);
            flat.dl_frequency_hz = Some(dl_frequency_hz);
            flat.data_rate = Some(data_rate);
        }
        McCommand::McClassBSessionReq {
            mc_group_id,
            session_time,
            time_out,
            periodicity,
            dl_frequency_hz,
            data_rate,
        } => {
            flat.mc_group_id = Some(mc_group_id);
            flat.session_time = Some(session_time);
            flat.time_out = Some(time_out);
            flat.periodicity = Some(periodicity);
            flat.dl_frequency_hz = Some(dl_frequency_hz);
            flat.data_rate = Some(data_rate);
        }
        McCommand::McClassCSessionAns {
            status,
            time_to_start,
        }
        | McCommand::McClassBSessionAns {
            status,
            time_to_start,
        } => {
            flat.mc_group_id = Some(status.mc_group_id);
            flat.dr_error = Some(status.dr_error);
            flat.freq_error = Some(status.freq_error);
            flat.no_session = Some(status.group_undefined);
            flat.start_missed = Some(status.start_missed);
            flat.time_to_start = time_to_start;
        }
        McCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a multicast setup command out of the record JavaScript holds.
fn mc_in(flat: &LorawanPackageCommand) -> Option<McCommand> {
    let status = SessionStatus {
        mc_group_id: flat.mc_group_id.unwrap_or(0),
        dr_error: flat.dr_error.unwrap_or(false),
        freq_error: flat.freq_error.unwrap_or(false),
        group_undefined: flat.no_session.unwrap_or(false),
        start_missed: flat.start_missed.unwrap_or(false),
    };
    Some(match flat.kind.as_str() {
        "packageVersionReq" => McCommand::PackageVersionReq,
        "packageVersionAns" => McCommand::PackageVersionAns(PackageVersion {
            package: flat.package?,
            version: flat.version?,
        }),
        "mcGroupStatusReq" => McCommand::McGroupStatusReq {
            req_group_mask: flat.group_mask?,
        },
        "mcGroupStatusAns" => McCommand::McGroupStatusAns {
            ans_group_mask: flat.group_mask?,
            nb_total_groups: flat.nb_total_groups.unwrap_or(0),
        },
        "mcGroupStatusItem" => McCommand::McGroupStatusItem {
            mc_group_id: flat.mc_group_id?,
            mc_addr: flat.mc_addr?,
        },
        "mcGroupSetupReq" => McCommand::McGroupSetupReq {
            mc_group_id: flat.mc_group_id?,
            mc_addr: flat.mc_addr?,
            mc_key_encrypted: sixteen(&flat.mc_key_encrypted)?,
            min_mc_fcnt: flat.min_mc_fcnt.unwrap_or(0),
            max_mc_fcnt: flat.max_mc_fcnt.unwrap_or(0),
        },
        "mcGroupSetupAns" => McCommand::McGroupSetupAns {
            mc_group_id: flat.mc_group_id?,
            id_error: flat.id_error.unwrap_or(false),
        },
        "mcGroupDeleteReq" => McCommand::McGroupDeleteReq {
            mc_group_id: flat.mc_group_id?,
        },
        "mcGroupDeleteAns" => McCommand::McGroupDeleteAns {
            mc_group_id: flat.mc_group_id?,
            group_undefined: flat.no_session.unwrap_or(false),
        },
        "mcClassCSessionReq" => McCommand::McClassCSessionReq {
            mc_group_id: flat.mc_group_id?,
            session_time: flat.session_time?,
            time_out: flat.time_out.unwrap_or(0),
            dl_frequency_hz: flat.dl_frequency_hz?,
            data_rate: flat.data_rate.unwrap_or(0),
        },
        "mcClassCSessionAns" => McCommand::McClassCSessionAns {
            status,
            time_to_start: flat.time_to_start,
        },
        "mcClassBSessionReq" => McCommand::McClassBSessionReq {
            mc_group_id: flat.mc_group_id?,
            session_time: flat.session_time?,
            time_out: flat.time_out.unwrap_or(0),
            periodicity: flat.periodicity.unwrap_or(0),
            dl_frequency_hz: flat.dl_frequency_hz?,
            data_rate: flat.data_rate.unwrap_or(0),
        },
        "mcClassBSessionAns" => McCommand::McClassBSessionAns {
            status,
            time_to_start: flat.time_to_start,
        },
        _ => return None,
    })
}
