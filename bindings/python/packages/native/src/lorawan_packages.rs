//! Generated Python bindings for the LoRaWAN application layer packages: clock
//! synchronization TS003-2.0.0, fragmented data block transport TS004-2.0.0, remote multicast
//! setup TS005-2.0.0, and firmware management TS006-1.0.0.
//!
//! The key derivations and the parity matrix are functions of their arguments. What has to
//! remember something is a class: a clock synchronization package, a firmware manager, a
//! fragmentation session being put back together, and the code taken over a block as it
//! arrives. Coded fields cross as lowercase names, so `pamoja.lorawan` can give them string
//! enums.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_lorawan::packages::clock::{ClockCommand, ClockSync, PORT as CLOCK_PORT};
use pamoja_lorawan::packages::firmware::{
    DeleteStatus, FirmwareCommand, FirmwareManager, Image, UpImageStatus, PORT as FIRMWARE_PORT,
};
use pamoja_lorawan::packages::fragment::{
    data_block_int_key, parity_line, prbs23, BlockMicKey, Defragmenter as CoreDefragmenter,
    FragCommand, Fragmenter, Progress, SetupStatus, MAX_FRAGMENTS, PORT as FRAGMENT_PORT,
};
use pamoja_lorawan::packages::multicast::{
    mc_app_s_key, mc_ke_key, mc_key, mc_nwk_s_key, mc_root_key_for, wrap_mc_key, McCommand,
    SessionStatus, PORT as MULTICAST_PORT,
};
use pamoja_lorawan::packages::PackageVersion;
use pamoja_lorawan::Direction;

use crate::PamojaError;

/// The ports and limits the four packages publish, as the module constants they are published
/// under.
pub const CONSTANTS: [(&str, u32); 5] = [
    ("LORAWAN_CLOCK_PORT", CLOCK_PORT as u32),
    ("LORAWAN_FRAGMENT_PORT", FRAGMENT_PORT as u32),
    ("LORAWAN_MULTICAST_PORT", MULTICAST_PORT as u32),
    ("LORAWAN_FIRMWARE_PORT", FIRMWARE_PORT as u32),
    ("LORAWAN_MAX_FRAGMENTS", MAX_FRAGMENTS as u32),
];

/// Reads a sixteen-byte key, or says which argument was the wrong length.
fn key(bytes: &[u8], what: &str) -> PyResult<[u8; 16]> {
    <[u8; 16]>::try_from(bytes).map_err(|_| {
        PamojaError::new_err(format!("{what} must be sixteen bytes, not {}", bytes.len()))
    })
}

/// Reads the image status the way Rust names it.
fn image_in(status: &str) -> PyResult<UpImageStatus> {
    match status {
        "none" => Ok(UpImageStatus::None),
        "corrupt" => Ok(UpImageStatus::Corrupt),
        "wrong_hardware" => Ok(UpImageStatus::WrongHardware),
        "valid" => Ok(UpImageStatus::Valid),
        other => Err(PamojaError::new_err(format!(
            "{other} is not an image status"
        ))),
    }
}

/// Names the image status the way Python holds it.
fn image_out(status: UpImageStatus) -> String {
    match status {
        UpImageStatus::None => "none",
        UpImageStatus::Corrupt => "corrupt",
        UpImageStatus::WrongHardware => "wrong_hardware",
        UpImageStatus::Valid => "valid",
    }
    .to_owned()
}

/// Derives a device's multicast root key, TS005-2.0.0 section 4.3.
///
/// `lorawan11` picks the scheme: LoRaWAN 1.0.x devices derive from their `GenAppKey`, and 1.1
/// devices from their `AppKey` under another constant.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (root_key, lorawan11 = false))]
pub fn lorawan_mc_root_key<'py>(
    py: Python<'py>,
    root_key: Vec<u8>,
    lorawan11: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let root = key(&root_key, "root_key")?;
    Ok(PyBytes::new(py, &mc_root_key_for(&root, lorawan11)))
}

/// Derives the key a multicast group's key travels under, section 4.3.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_mc_ke_key<'py>(
    py: Python<'py>,
    mc_root_key: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    let root = key(&mc_root_key, "mc_root_key")?;
    Ok(PyBytes::new(py, &mc_ke_key(&root)))
}

/// Unwraps the group key a setup command carried, section 4.3.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_mc_key<'py>(
    py: Python<'py>,
    mc_ke_key: Vec<u8>,
    wrapped: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    let ke = key(&mc_ke_key, "mc_ke_key")?;
    let wrapped = key(&wrapped, "wrapped")?;
    Ok(PyBytes::new(py, &mc_key(&ke, &wrapped)))
}

/// Wraps a group key for a device, which is what a server does before sending it.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_wrap_mc_key<'py>(
    py: Python<'py>,
    mc_ke_key: Vec<u8>,
    mc_key: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    let ke = key(&mc_ke_key, "mc_ke_key")?;
    let group = key(&mc_key, "mc_key")?;
    Ok(PyBytes::new(py, &wrap_mc_key(&ke, &group)))
}

/// Derives the key that reads a multicast group's payloads, section 4.3.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_mc_app_s_key<'py>(
    py: Python<'py>,
    mc_key: Vec<u8>,
    mc_addr: u32,
) -> PyResult<Bound<'py, PyBytes>> {
    let group = key(&mc_key, "mc_key")?;
    Ok(PyBytes::new(py, &mc_app_s_key(&group, mc_addr)))
}

/// Derives the key that verifies a multicast group's frames, section 4.3.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_mc_nwk_s_key<'py>(
    py: Python<'py>,
    mc_key: Vec<u8>,
    mc_addr: u32,
) -> PyResult<Bound<'py, PyBytes>> {
    let group = key(&mc_key, "mc_key")?;
    Ok(PyBytes::new(py, &mc_nwk_s_key(&group, mc_addr)))
}

/// Derives the key that signs a data block, TS004-2.0.0 section 3.3.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_data_block_int_key<'py>(
    py: Python<'py>,
    root_key: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    let root = key(&root_key, "root_key")?;
    Ok(PyBytes::new(py, &data_block_int_key(&root)))
}

/// Steps the pseudo-random sequence the parity matrix is drawn from, appendix A.1.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_frag_prbs23(x: u32) -> u32 {
    prbs23(x)
}

/// Lists the uncoded fragments a coded one is made of, appendix A.1.
///
/// `coded` counts from one past the uncoded fragments: a session's fragment `nb_frag + 1` is
/// coded fragment 1.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_frag_parity_line(coded: u16, nb_frag: u16) -> PyResult<Vec<u16>> {
    if nb_frag == 0 || nb_frag > MAX_FRAGMENTS {
        return Err(PamojaError::new_err(
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
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanFragSession {
    /// How many uncoded fragments the block takes.
    #[pyo3(get)]
    nb_frag: u16,
    /// How many bytes of padding the last one carries.
    #[pyo3(get)]
    padding: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanFragSession {
    fn __repr__(&self) -> String {
        format!(
            "LorawanFragSession(nb_frag={}, padding={})",
            self.nb_frag, self.padding
        )
    }
}

/// Says how many fragments a block takes, and how much padding the last one needs.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_frag_session(block_len: usize, frag_size: u8) -> PyResult<LorawanFragSession> {
    let block = vec![0u8; block_len];
    let sender = Fragmenter::new(&block, frag_size)
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    Ok(LorawanFragSession {
        nb_frag: sender.nb_frag(),
        padding: sender.padding(),
    })
}

/// Builds one fragment of a session out of a block held whole.
///
/// Up to `nb_frag` the fragment is a piece of the block; past that it is a coded fragment, the
/// exclusive-or of a pseudo-random half of the pieces.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_frag_fragment<'py>(
    py: Python<'py>,
    block: Vec<u8>,
    frag_size: u8,
    n: u16,
) -> PyResult<Bound<'py, PyBytes>> {
    let sender = Fragmenter::new(&block, frag_size)
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    let mut out = vec![0u8; usize::from(frag_size)];
    sender
        .fragment(n, &mut out)
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    Ok(PyBytes::new(py, &out))
}

/// A fragmentation session being put back together, TS004-2.0.0 appendix A.2.
///
/// The uncoded fragments go straight into the block. A coded fragment is reduced against
/// everything already known and kept only if it says something new, so the working memory is
/// sized by the losses rather than by the block.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanDefragmenter {
    block: Vec<u8>,
    matrix: Vec<u8>,
    nb_frag: u16,
    frag_size: u8,
    progress: Progress,
    missing: u16,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanDefragmenter {
    /// Opens a session for a block of `nb_frag` fragments of `frag_size` bytes.
    ///
    /// `max_lost` is the most uncoded fragments to be able to solve for, which decides how
    /// much working memory the session takes.
    #[new]
    fn new(nb_frag: u16, frag_size: u8, max_lost: u16) -> PyResult<Self> {
        if nb_frag == 0 || nb_frag > MAX_FRAGMENTS || frag_size == 0 {
            return Err(PamojaError::new_err(
                "the session is not one this build runs",
            ));
        }
        Ok(Self {
            block: vec![0u8; usize::from(nb_frag) * usize::from(frag_size)],
            matrix: vec![0u8; CoreDefragmenter::matrix_len(nb_frag, max_lost.min(nb_frag))],
            nb_frag,
            frag_size,
            progress: Progress::default(),
            missing: nb_frag,
        })
    }

    /// Takes one fragment of the session, counting from one.
    ///
    /// Returns `True` once the block is whole.
    fn fragment(&mut self, n: u16, data: Vec<u8>) -> PyResult<bool> {
        let mut session = CoreDefragmenter::resumed(
            self.nb_frag,
            self.frag_size,
            &mut self.block,
            &mut self.matrix,
            self.progress,
        )
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
        let outcome = session.fragment(n, &data);
        self.progress = session.progress();
        self.missing = session.missing();
        outcome.map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// The block, as far as it has been put back together, padding and all.
    #[getter]
    fn block<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.block)
    }

    /// Whether the whole block is there.
    #[getter]
    fn done(&self) -> bool {
        self.progress.done
    }

    /// How many fragments arrived, coded, uncoded and repeated.
    #[getter]
    fn received(&self) -> u16 {
        self.progress.received
    }

    /// How many uncoded fragments are still missing.
    #[getter]
    fn missing(&self) -> u16 {
        self.missing
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanDefragmenter(nb_frag={}, frag_size={}, received={}, missing={})",
            self.nb_frag, self.frag_size, self.progress.received, self.missing
        )
    }
}

/// The code taken over a data block as it arrives, TS004-2.0.0 section 3.3.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanBlockMic {
    key: [u8; 16],
    session_cnt: u16,
    frag_index: u8,
    descriptor: [u8; 4],
    block_len: u32,
    pieces: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanBlockMic {
    /// Starts a code over one session's block.
    ///
    /// `data_block_int_key` is what :func:`lorawan_data_block_int_key` derived, and the rest
    /// are the fields the session setup carried.
    #[new]
    fn new(
        data_block_int_key: Vec<u8>,
        session_cnt: u16,
        frag_index: u8,
        descriptor: Vec<u8>,
        block_len: u32,
    ) -> PyResult<Self> {
        let key = key(&data_block_int_key, "data_block_int_key")?;
        let descriptor = <[u8; 4]>::try_from(descriptor.as_slice()).map_err(|_| {
            PamojaError::new_err(format!(
                "descriptor must be four bytes, not {}",
                descriptor.len()
            ))
        })?;
        Ok(Self {
            key,
            session_cnt,
            frag_index,
            descriptor,
            block_len,
            pieces: Vec::new(),
        })
    }

    /// Adds a piece of the block, in order, without any padding.
    fn update(&mut self, data: Vec<u8>) {
        self.pieces.extend_from_slice(&data);
    }

    /// Finishes the code, returning the four bytes a session setup carries.
    fn finish<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let signer = BlockMicKey::new(&self.key);
        let mut mic = signer.start(
            self.session_cnt,
            self.frag_index,
            self.descriptor,
            self.block_len,
        );
        mic.update(&self.pieces);
        PyBytes::new(py, &mic.finish())
    }
}

/// What a device made of a downlink on the clock port.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanClockHeard {
    /// The seconds to add to the device's clock, where an answer carried one.
    #[pyo3(get)]
    correction: Option<i32>,
    /// Whether the correction was the largest the field carries, so another follows.
    #[pyo3(get)]
    more_correction: bool,
    /// How many requests a resynchronization command asked for.
    #[pyo3(get)]
    resync: Option<u8>,
    /// Whether the device now owes an answer.
    #[pyo3(get)]
    answer_due: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanClockHeard {
    fn __repr__(&self) -> String {
        let flag = |on: bool| if on { "True" } else { "False" };
        format!(
            "LorawanClockHeard(correction={:?}, more_correction={}, resync={:?}, answer_due={})",
            self.correction,
            flag(self.more_correction),
            self.resync,
            flag(self.answer_due)
        )
    }
}

/// The clock synchronization package running on a device, TS003-2.0.0.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanClockSync {
    inner: ClockSync,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanClockSync {
    /// Starts the package.
    ///
    /// `self_managed` marks a device that keeps its own periodicity and answers a server that
    /// tries to set one with the not-supported bit.
    #[new]
    #[pyo3(signature = (self_managed = false))]
    fn new(self_managed: bool) -> Self {
        Self {
            inner: if self_managed {
                ClockSync::self_managed()
            } else {
                ClockSync::new()
            },
        }
    }

    /// Builds the request that asks a server for a correction, section 3.2.
    #[pyo3(signature = (device_time, ans_required = false))]
    fn request<'py>(
        &mut self,
        py: Python<'py>,
        device_time: u32,
        ans_required: bool,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mut out = [0u8; 8];
        let len = self
            .inner
            .app_time_req(device_time, ans_required, &mut out)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &out[..len]))
    }

    /// Reads a downlink on the clock port and acts on it.
    fn heard(&mut self, payload: Vec<u8>) -> PyResult<LorawanClockHeard> {
        let heard = self
            .inner
            .heard(&payload)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(LorawanClockHeard {
            correction: heard.correction,
            more_correction: heard.more_correction,
            resync: heard.resync,
            answer_due: heard.answer_due,
        })
    }

    /// Writes the answer the device owes, or an empty value when it owes none.
    fn answer<'py>(&mut self, py: Python<'py>, device_time: u32) -> PyResult<Bound<'py, PyBytes>> {
        let mut out = [0u8; 8];
        let len = self
            .inner
            .answer(device_time, &mut out)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &out[..len]))
    }

    /// The token the next request will carry.
    #[getter]
    fn token(&self) -> u8 {
        self.inner.token()
    }

    /// The seconds between requests, as the server last set them.
    #[getter]
    fn period_s(&self) -> u32 {
        self.inner.period_s()
    }

    /// Whether the device owes its server an answer.
    #[getter]
    fn answer_due(&self) -> bool {
        self.inner.answer_due()
    }
}

/// The firmware management package running on a device, TS006-1.0.0.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanFirmware {
    inner: FirmwareManager,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanFirmware {
    /// Starts the package, reporting the versions the device was built with.
    #[new]
    fn new(firmware: u32, hardware: u32) -> Self {
        Self {
            inner: FirmwareManager::new(firmware, hardware),
        }
    }

    /// Says what firmware upgrade image the device is holding.
    #[pyo3(signature = (status, version = None))]
    fn set_image(&mut self, status: &str, version: Option<u32>) -> PyResult<()> {
        self.inner.set_image(match image_in(status)? {
            UpImageStatus::None => None,
            UpImageStatus::Valid => Some(Image::valid(version.unwrap_or(0))),
            other => Some(Image::refused(other)),
        });
        Ok(())
    }

    /// Reads a downlink on the firmware port and writes the answers it calls for.
    ///
    /// `now_s` is what the device believes the time is, in seconds since the GPS epoch; a
    /// device that does not know refuses a reboot set for a moment in time.
    #[pyo3(signature = (payload, now_s = None))]
    fn heard<'py>(
        &mut self,
        py: Python<'py>,
        payload: Vec<u8>,
        now_s: Option<u32>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mut out = [0u8; 64];
        let len = self
            .inner
            .heard_at(&payload, now_s, &mut out)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &out[..len]))
    }

    /// The moment the device is to reboot, where one was set as a time.
    #[getter]
    fn reboot_at_s(&self) -> Option<u32> {
        self.inner.reboot_at_s()
    }

    /// How long until it reboots, where one was set as a countdown.
    #[getter]
    fn reboot_in_s(&self) -> Option<u32> {
        self.inner.reboot_in_s()
    }

    /// Whether the device was told to reboot at once.
    #[getter]
    fn reboot_now(&self) -> bool {
        self.inner.reboot_now()
    }

    /// What the device would boot into, for an image it can install.
    #[getter]
    fn next_version(&self) -> Option<u32> {
        self.inner
            .image()
            .filter(|image| matches!(image.status, UpImageStatus::Valid))
            .map(|image| image.version)
    }

    /// What the device makes of the image it holds.
    #[getter]
    fn image_status(&self) -> String {
        image_out(
            self.inner
                .image()
                .map_or(UpImageStatus::None, |image| image.status),
        )
    }

    /// Forgets the programmed reboot, for a device that has carried it out.
    fn rebooted(&mut self) {
        self.inner.rebooted();
    }
}

/// One command of an application layer package, whichever package it belongs to.
///
/// `port` says which package and `kind` names the command within it; together they decide
/// which of the other fields carry anything. The rest are `None`.
#[gen_stub_pyclass]
#[pyclass]
#[derive(Default)]
pub struct LorawanPackageCommand {
    /// Which package this command belongs to, as its port.
    #[pyo3(get)]
    port: u8,
    /// Which command this is, as a name.
    #[pyo3(get)]
    kind: String,
    /// Which way it travels.
    #[pyo3(get)]
    uplink: bool,
    /// The package identifier a version answer carries.
    #[pyo3(get)]
    package: Option<u8>,
    /// The package version it implements.
    #[pyo3(get)]
    version: Option<u8>,
    /// A device's own clock, in seconds since the GPS epoch.
    #[pyo3(get)]
    device_time: Option<u32>,
    /// The seconds to add to a device's clock.
    #[pyo3(get)]
    time_correction: Option<i32>,
    /// The token that pairs a clock answer with its request.
    #[pyo3(get)]
    token: Option<u8>,
    /// Whether a clock request must be answered.
    #[pyo3(get)]
    ans_required: Option<bool>,
    /// The coded period between clock requests.
    #[pyo3(get)]
    period: Option<u8>,
    /// Whether a device manages its own clock periodicity.
    #[pyo3(get)]
    not_supported: Option<bool>,
    /// How many requests a resynchronization command asks for.
    #[pyo3(get)]
    transmissions: Option<u8>,
    /// The firmware a device reports running.
    #[pyo3(get)]
    firmware: Option<u32>,
    /// The hardware it runs on.
    #[pyo3(get)]
    hardware: Option<u32>,
    /// The moment or the delay a reboot is set for.
    #[pyo3(get)]
    reboot: Option<u32>,
    /// What a device makes of the upgrade image it holds.
    #[pyo3(get)]
    image_status: Option<String>,
    /// The version it would run once that image is installed.
    #[pyo3(get)]
    next_version: Option<u32>,
    /// The version a delete command names.
    #[pyo3(get)]
    delete_version: Option<u32>,
    /// Whether a device holds no valid image.
    #[pyo3(get)]
    no_valid_image: Option<bool>,
    /// Whether the version named is not the one held.
    #[pyo3(get)]
    invalid_version: Option<bool>,
    /// Which fragmentation session, 0 to 3.
    #[pyo3(get)]
    frag_index: Option<u8>,
    /// Which multicast groups may feed it, a bit for each.
    #[pyo3(get)]
    mc_group_bit_mask: Option<u8>,
    /// How many uncoded fragments a block was cut into.
    #[pyo3(get)]
    nb_frag: Option<u16>,
    /// How many bytes each fragment carries.
    #[pyo3(get)]
    frag_size: Option<u8>,
    /// Whether a device reports the block once it has it.
    #[pyo3(get)]
    ack_reception: Option<bool>,
    /// Which fragmentation algorithm to run.
    #[pyo3(get)]
    frag_algo: Option<u8>,
    /// The coded spread of the delay before a device answers.
    #[pyo3(get)]
    block_ack_delay: Option<u8>,
    /// How many bytes of padding the last fragment carries.
    #[pyo3(get)]
    padding: Option<u8>,
    /// The four bytes a server describes a block with.
    #[pyo3(get)]
    descriptor: Option<Vec<u8>>,
    /// The session counter, which must rise for each new block.
    #[pyo3(get)]
    session_cnt: Option<u16>,
    /// The code over the block a device checks once it has it all.
    #[pyo3(get)]
    mic: Option<Vec<u8>>,
    /// How many fragments arrived, coded, uncoded and repeated.
    #[pyo3(get)]
    received: Option<u16>,
    /// How many uncoded fragments are still missing.
    #[pyo3(get)]
    missing: Option<u8>,
    /// Whether the block's code did not check out.
    #[pyo3(get)]
    mic_error: Option<bool>,
    /// Whether a session ran out of memory to defragment with.
    #[pyo3(get)]
    memory_error: Option<bool>,
    /// Whether the session or group named does not exist on the device.
    #[pyo3(get)]
    no_session: Option<bool>,
    /// Whether the setup named an algorithm the device does not run.
    #[pyo3(get)]
    unsupported_algorithm: Option<bool>,
    /// Whether the setup named an index the device does not keep.
    #[pyo3(get)]
    unsupported_index: Option<bool>,
    /// Whether the descriptor is not one the device accepts.
    #[pyo3(get)]
    wrong_descriptor: Option<bool>,
    /// Whether the session counter repeats one already used.
    #[pyo3(get)]
    session_replay: Option<bool>,
    /// Whether every device answers a status request.
    #[pyo3(get)]
    all_participants: Option<bool>,
    /// Which fragment of a session a data fragment carries, counting from one.
    #[pyo3(get)]
    fragment_n: Option<u16>,
    /// The bytes a data fragment carries.
    #[pyo3(get)]
    data: Option<Vec<u8>>,
    /// Which multicast group, 0 to 3.
    #[pyo3(get)]
    mc_group_id: Option<u8>,
    /// The address a group answers to.
    #[pyo3(get)]
    mc_addr: Option<u32>,
    /// A group's key, wrapped under the device's key encryption key.
    #[pyo3(get)]
    mc_key_encrypted: Option<Vec<u8>>,
    /// The first frame counter a device accepts from a group.
    #[pyo3(get)]
    min_mc_fcnt: Option<u32>,
    /// The last one, which ends the group's life.
    #[pyo3(get)]
    max_mc_fcnt: Option<u32>,
    /// Which groups a status request or answer covers, a bit for each.
    #[pyo3(get)]
    group_mask: Option<u8>,
    /// How many groups a device holds in all.
    #[pyo3(get)]
    nb_total_groups: Option<u8>,
    /// Whether a device holds no group by the identifier named.
    #[pyo3(get)]
    id_error: Option<bool>,
    /// When a multicast window opens, in seconds since the GPS epoch.
    #[pyo3(get)]
    session_time: Option<u32>,
    /// How long it lasts at most, coded.
    #[pyo3(get)]
    time_out: Option<u8>,
    /// How often a device opens a ping slot inside a Class B window.
    #[pyo3(get)]
    periodicity: Option<u8>,
    /// Where a group listens, in hertz.
    #[pyo3(get)]
    dl_frequency_hz: Option<u32>,
    /// The data rate it listens at.
    #[pyo3(get)]
    data_rate: Option<u8>,
    /// How many seconds until a window opens.
    #[pyo3(get)]
    time_to_start: Option<u32>,
    /// Whether the data rate named is not one the device has.
    #[pyo3(get)]
    dr_error: Option<bool>,
    /// Whether the frequency named is not one it can use.
    #[pyo3(get)]
    freq_error: Option<bool>,
    /// Whether the window was to start at a time already past.
    #[pyo3(get)]
    start_missed: Option<bool>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanPackageCommand {
    /// Builds a command to write out.
    ///
    /// The port and the name decide which command this is, and therefore which of the other
    /// arguments are read. The rest may be left off.
    #[new]
    #[pyo3(signature = (
        port,
        kind,
        uplink = false,
        package = None,
        version = None,
        device_time = None,
        time_correction = None,
        token = None,
        ans_required = None,
        period = None,
        not_supported = None,
        transmissions = None,
        firmware = None,
        hardware = None,
        reboot = None,
        image_status = None,
        next_version = None,
        delete_version = None,
        no_valid_image = None,
        invalid_version = None,
        frag_index = None,
        mc_group_bit_mask = None,
        nb_frag = None,
        frag_size = None,
        ack_reception = None,
        frag_algo = None,
        block_ack_delay = None,
        padding = None,
        descriptor = None,
        session_cnt = None,
        mic = None,
        received = None,
        missing = None,
        mic_error = None,
        memory_error = None,
        no_session = None,
        unsupported_algorithm = None,
        unsupported_index = None,
        wrong_descriptor = None,
        session_replay = None,
        all_participants = None,
        fragment_n = None,
        data = None,
        mc_group_id = None,
        mc_addr = None,
        mc_key_encrypted = None,
        min_mc_fcnt = None,
        max_mc_fcnt = None,
        group_mask = None,
        nb_total_groups = None,
        id_error = None,
        session_time = None,
        time_out = None,
        periodicity = None,
        dl_frequency_hz = None,
        data_rate = None,
        time_to_start = None,
        dr_error = None,
        freq_error = None,
        start_missed = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        port: u8,
        kind: &str,
        uplink: bool,
        package: Option<u8>,
        version: Option<u8>,
        device_time: Option<u32>,
        time_correction: Option<i32>,
        token: Option<u8>,
        ans_required: Option<bool>,
        period: Option<u8>,
        not_supported: Option<bool>,
        transmissions: Option<u8>,
        firmware: Option<u32>,
        hardware: Option<u32>,
        reboot: Option<u32>,
        image_status: Option<String>,
        next_version: Option<u32>,
        delete_version: Option<u32>,
        no_valid_image: Option<bool>,
        invalid_version: Option<bool>,
        frag_index: Option<u8>,
        mc_group_bit_mask: Option<u8>,
        nb_frag: Option<u16>,
        frag_size: Option<u8>,
        ack_reception: Option<bool>,
        frag_algo: Option<u8>,
        block_ack_delay: Option<u8>,
        padding: Option<u8>,
        descriptor: Option<Vec<u8>>,
        session_cnt: Option<u16>,
        mic: Option<Vec<u8>>,
        received: Option<u16>,
        missing: Option<u8>,
        mic_error: Option<bool>,
        memory_error: Option<bool>,
        no_session: Option<bool>,
        unsupported_algorithm: Option<bool>,
        unsupported_index: Option<bool>,
        wrong_descriptor: Option<bool>,
        session_replay: Option<bool>,
        all_participants: Option<bool>,
        fragment_n: Option<u16>,
        data: Option<Vec<u8>>,
        mc_group_id: Option<u8>,
        mc_addr: Option<u32>,
        mc_key_encrypted: Option<Vec<u8>>,
        min_mc_fcnt: Option<u32>,
        max_mc_fcnt: Option<u32>,
        group_mask: Option<u8>,
        nb_total_groups: Option<u8>,
        id_error: Option<bool>,
        session_time: Option<u32>,
        time_out: Option<u8>,
        periodicity: Option<u8>,
        dl_frequency_hz: Option<u32>,
        data_rate: Option<u8>,
        time_to_start: Option<u32>,
        dr_error: Option<bool>,
        freq_error: Option<bool>,
        start_missed: Option<bool>,
    ) -> LorawanPackageCommand {
        LorawanPackageCommand {
            port,
            kind: kind.to_owned(),
            uplink,
            package,
            version,
            device_time,
            time_correction,
            token,
            ans_required,
            period,
            not_supported,
            transmissions,
            firmware,
            hardware,
            reboot,
            image_status,
            next_version,
            delete_version,
            no_valid_image,
            invalid_version,
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
            received,
            missing,
            mic_error,
            memory_error,
            no_session,
            unsupported_algorithm,
            unsupported_index,
            wrong_descriptor,
            session_replay,
            all_participants,
            fragment_n,
            data,
            mc_group_id,
            mc_addr,
            mc_key_encrypted,
            min_mc_fcnt,
            max_mc_fcnt,
            group_mask,
            nb_total_groups,
            id_error,
            session_time,
            time_out,
            periodicity,
            dl_frequency_hz,
            data_rate,
            time_to_start,
            dr_error,
            freq_error,
            start_missed,
        }
    }

    /// Writes this command out, which is what :func:`lorawan_package_encode` does.
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        lorawan_package_encode(py, self)
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanPackageCommand(port={}, kind={:?}, uplink={})",
            self.port,
            self.kind,
            if self.uplink { "True" } else { "False" }
        )
    }
}

impl LorawanPackageCommand {
    /// An empty command of one package, which the readers below fill in.
    fn empty(port: u8, kind: &str, uplink: bool) -> LorawanPackageCommand {
        LorawanPackageCommand {
            port,
            kind: kind.to_owned(),
            uplink,
            ..LorawanPackageCommand::default()
        }
    }
}

/// Reads one command of an application layer package.
///
/// `uplink` says which way the frame carrying it traveled, because the same identifier names a
/// different command in each direction. A data fragment takes the whole message, as
/// TS004-2.0.0 section 3 asks, and its bytes come back on `data`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_package_parse(
    port: u8,
    uplink: bool,
    payload: Vec<u8>,
) -> PyResult<LorawanPackageCommand> {
    let direction = if uplink {
        Direction::Uplink
    } else {
        Direction::Downlink
    };
    let refused = |error: pamoja_lorawan::LorawanError| PamojaError::new_err(error.to_string());
    match port {
        CLOCK_PORT => ClockCommand::parse(direction, &payload)
            .map(|(command, _)| clock_out(command))
            .map_err(refused),
        FIRMWARE_PORT => FirmwareCommand::parse(direction, &payload)
            .map(|(command, _)| firmware_out(command))
            .map_err(refused),
        FRAGMENT_PORT => FragCommand::parse(direction, &payload)
            .map(|(command, _)| frag_out(command))
            .map_err(refused),
        MULTICAST_PORT => McCommand::parse(direction, &payload)
            .map(|(command, _)| mc_out(command))
            .map_err(refused),
        other => Err(PamojaError::new_err(format!(
            "port {other} is not an application layer package"
        ))),
    }
}

/// Reads every command in one message, stopping at an identifier the package does not define.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_package_parse_all(
    port: u8,
    uplink: bool,
    payload: Vec<u8>,
) -> PyResult<Vec<LorawanPackageCommand>> {
    let direction = if uplink {
        Direction::Uplink
    } else {
        Direction::Downlink
    };
    let mut at = 0;
    let mut read = Vec::new();
    while at < payload.len() {
        let taken = match port {
            CLOCK_PORT => ClockCommand::parse(direction, &payload[at..]).map(|(command, taken)| {
                read.push(clock_out(command));
                taken
            }),
            FIRMWARE_PORT => {
                FirmwareCommand::parse(direction, &payload[at..]).map(|(command, taken)| {
                    read.push(firmware_out(command));
                    taken
                })
            }
            FRAGMENT_PORT => {
                FragCommand::parse(direction, &payload[at..]).map(|(command, taken)| {
                    read.push(frag_out(command));
                    taken
                })
            }
            MULTICAST_PORT => {
                McCommand::parse(direction, &payload[at..]).map(|(command, taken)| {
                    read.push(mc_out(command));
                    taken
                })
            }
            other => {
                return Err(PamojaError::new_err(format!(
                    "port {other} is not an application layer package"
                )))
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
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_package_status_item(payload: Vec<u8>) -> PyResult<LorawanPackageCommand> {
    McCommand::status_item(&payload)
        .map(|(command, _)| mc_out(command))
        .map_err(|error| PamojaError::new_err(error.to_string()))
}

/// Writes one command of an application layer package.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_package_encode<'py>(
    py: Python<'py>,
    command: &LorawanPackageCommand,
) -> PyResult<Bound<'py, PyBytes>> {
    let data = command.data.clone().unwrap_or_default();
    let mut out = vec![0u8; data.len() + 64];
    let unknown = || {
        PamojaError::new_err(format!(
            "{} on port {} is not a command this build writes",
            command.kind, command.port
        ))
    };
    let written = match command.port {
        CLOCK_PORT => clock_in(command)?.ok_or_else(unknown)?.encode(&mut out),
        FIRMWARE_PORT => firmware_in(command)?.ok_or_else(unknown)?.encode(&mut out),
        FRAGMENT_PORT => frag_in(command, &data)
            .ok_or_else(unknown)?
            .encode(&mut out),
        MULTICAST_PORT => mc_in(command).ok_or_else(unknown)?.encode(&mut out),
        other => {
            return Err(PamojaError::new_err(format!(
                "port {other} is not an application layer package"
            )))
        }
    };
    let len = written.map_err(|error| PamojaError::new_err(error.to_string()))?;
    Ok(PyBytes::new(py, &out[..len]))
}

/// Reads four bytes from a field that must carry exactly that many.
fn four(bytes: &Option<Vec<u8>>) -> Option<[u8; 4]> {
    <[u8; 4]>::try_from(bytes.as_ref()?.as_slice()).ok()
}

/// Reads sixteen bytes from a field that must carry exactly that many.
fn sixteen(bytes: &Option<Vec<u8>>) -> Option<[u8; 16]> {
    <[u8; 16]>::try_from(bytes.as_ref()?.as_slice()).ok()
}

/// Describes a clock synchronization command the way Python holds it.
fn clock_out(command: ClockCommand) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        ClockCommand::PackageVersionReq => "package_version_req",
        ClockCommand::PackageVersionAns(_) => "package_version_ans",
        ClockCommand::AppTimeReq { .. } => "app_time_req",
        ClockCommand::AppTimeAns { .. } => "app_time_ans",
        ClockCommand::DeviceAppTimePeriodicityReq { .. } => "device_app_time_periodicity_req",
        ClockCommand::DeviceAppTimePeriodicityAns { .. } => "device_app_time_periodicity_ans",
        ClockCommand::ForceDeviceResyncCmd { .. } => "force_device_resync_cmd",
    };
    let mut flat = LorawanPackageCommand::empty(CLOCK_PORT, name, uplink);
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

/// Reads a clock synchronization command out of the record Python holds.
fn clock_in(flat: &LorawanPackageCommand) -> PyResult<Option<ClockCommand>> {
    let missing = |what: &str| PamojaError::new_err(format!("{} needs {what}", flat.kind));
    Ok(Some(match flat.kind.as_str() {
        "package_version_req" => ClockCommand::PackageVersionReq,
        "package_version_ans" => ClockCommand::PackageVersionAns(PackageVersion {
            package: flat.package.ok_or_else(|| missing("package"))?,
            version: flat.version.ok_or_else(|| missing("version"))?,
        }),
        "app_time_req" => ClockCommand::AppTimeReq {
            device_time: flat.device_time.ok_or_else(|| missing("device_time"))?,
            ans_required: flat.ans_required.unwrap_or(false),
            token: flat.token.unwrap_or(0),
        },
        "app_time_ans" => ClockCommand::AppTimeAns {
            time_correction: flat
                .time_correction
                .ok_or_else(|| missing("time_correction"))?,
            token: flat.token.unwrap_or(0),
        },
        "device_app_time_periodicity_req" => ClockCommand::DeviceAppTimePeriodicityReq {
            period: flat.period.ok_or_else(|| missing("period"))?,
        },
        "device_app_time_periodicity_ans" => ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported: flat.not_supported.unwrap_or(false),
            device_time: flat.device_time.ok_or_else(|| missing("device_time"))?,
        },
        "force_device_resync_cmd" => ClockCommand::ForceDeviceResyncCmd {
            transmissions: flat.transmissions.ok_or_else(|| missing("transmissions"))?,
        },
        _ => return Ok(None),
    }))
}

/// Describes a firmware management command the way Python holds it.
fn firmware_out(command: FirmwareCommand) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        FirmwareCommand::PackageVersionReq => "package_version_req",
        FirmwareCommand::PackageVersionAns(_) => "package_version_ans",
        FirmwareCommand::DevVersionReq => "dev_version_req",
        FirmwareCommand::DevVersionAns { .. } => "dev_version_ans",
        FirmwareCommand::DevRebootTimeReq { .. } => "dev_reboot_time_req",
        FirmwareCommand::DevRebootTimeAns { .. } => "dev_reboot_time_ans",
        FirmwareCommand::DevRebootCountdownReq { .. } => "dev_reboot_countdown_req",
        FirmwareCommand::DevRebootCountdownAns { .. } => "dev_reboot_countdown_ans",
        FirmwareCommand::DevUpgradeImageReq => "dev_upgrade_image_req",
        FirmwareCommand::DevUpgradeImageAns { .. } => "dev_upgrade_image_ans",
        FirmwareCommand::DevDeleteImageReq { .. } => "dev_delete_image_req",
        FirmwareCommand::DevDeleteImageAns(_) => "dev_delete_image_ans",
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

/// Reads a firmware management command out of the record Python holds.
fn firmware_in(flat: &LorawanPackageCommand) -> PyResult<Option<FirmwareCommand>> {
    let missing = |what: &str| PamojaError::new_err(format!("{} needs {what}", flat.kind));
    Ok(Some(match flat.kind.as_str() {
        "package_version_req" => FirmwareCommand::PackageVersionReq,
        "package_version_ans" => FirmwareCommand::PackageVersionAns(PackageVersion {
            package: flat.package.ok_or_else(|| missing("package"))?,
            version: flat.version.ok_or_else(|| missing("version"))?,
        }),
        "dev_version_req" => FirmwareCommand::DevVersionReq,
        "dev_version_ans" => FirmwareCommand::DevVersionAns {
            firmware: flat.firmware.ok_or_else(|| missing("firmware"))?,
            hardware: flat.hardware.ok_or_else(|| missing("hardware"))?,
        },
        "dev_reboot_time_req" => FirmwareCommand::DevRebootTimeReq {
            reboot_time: flat.reboot.ok_or_else(|| missing("reboot"))?,
        },
        "dev_reboot_time_ans" => FirmwareCommand::DevRebootTimeAns {
            reboot_time: flat.reboot.ok_or_else(|| missing("reboot"))?,
        },
        "dev_reboot_countdown_req" => FirmwareCommand::DevRebootCountdownReq {
            countdown: flat.reboot.ok_or_else(|| missing("reboot"))?,
        },
        "dev_reboot_countdown_ans" => FirmwareCommand::DevRebootCountdownAns {
            countdown: flat.reboot.ok_or_else(|| missing("reboot"))?,
        },
        "dev_upgrade_image_req" => FirmwareCommand::DevUpgradeImageReq,
        "dev_upgrade_image_ans" => FirmwareCommand::DevUpgradeImageAns {
            status: image_in(
                flat.image_status
                    .as_deref()
                    .ok_or_else(|| missing("image_status"))?,
            )?,
            next_version: flat.next_version,
        },
        "dev_delete_image_req" => FirmwareCommand::DevDeleteImageReq {
            version: flat
                .delete_version
                .ok_or_else(|| missing("delete_version"))?,
        },
        "dev_delete_image_ans" => FirmwareCommand::DevDeleteImageAns(DeleteStatus {
            no_valid_image: flat.no_valid_image.unwrap_or(false),
            invalid_version: flat.invalid_version.unwrap_or(false),
        }),
        _ => return Ok(None),
    }))
}

/// Describes a fragmentation command the way Python holds it.
fn frag_out(command: FragCommand<'_>) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        FragCommand::PackageVersionReq => "package_version_req",
        FragCommand::PackageVersionAns(_) => "package_version_ans",
        FragCommand::FragSessionStatusReq { .. } => "frag_session_status_req",
        FragCommand::FragSessionStatusAns { .. } => "frag_session_status_ans",
        FragCommand::FragSessionSetupReq { .. } => "frag_session_setup_req",
        FragCommand::FragSessionSetupAns(_) => "frag_session_setup_ans",
        FragCommand::FragSessionDeleteReq { .. } => "frag_session_delete_req",
        FragCommand::FragSessionDeleteAns { .. } => "frag_session_delete_ans",
        FragCommand::FragDataBlockReceivedReq { .. } => "frag_data_block_received_req",
        FragCommand::FragDataBlockReceivedAns { .. } => "frag_data_block_received_ans",
        FragCommand::DataFragment { .. } => "data_fragment",
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
            flat.descriptor = Some(descriptor.to_vec());
            flat.session_cnt = Some(session_cnt);
            flat.mic = Some(mic.to_vec());
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
            flat.data = Some(data.to_vec());
        }
        FragCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a fragmentation command out of the record Python holds.
fn frag_in<'a>(flat: &LorawanPackageCommand, data: &'a [u8]) -> Option<FragCommand<'a>> {
    Some(match flat.kind.as_str() {
        "package_version_req" => FragCommand::PackageVersionReq,
        "package_version_ans" => FragCommand::PackageVersionAns(PackageVersion {
            package: flat.package?,
            version: flat.version?,
        }),
        "frag_session_status_req" => FragCommand::FragSessionStatusReq {
            frag_index: flat.frag_index?,
            all_participants: flat.all_participants.unwrap_or(false),
        },
        "frag_session_status_ans" => FragCommand::FragSessionStatusAns {
            frag_index: flat.frag_index?,
            received: flat.received?,
            missing: flat.missing.unwrap_or(0),
            mic_error: flat.mic_error.unwrap_or(false),
            memory_error: flat.memory_error.unwrap_or(false),
            no_session: flat.no_session.unwrap_or(false),
        },
        "frag_session_setup_req" => FragCommand::FragSessionSetupReq {
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
        "frag_session_setup_ans" => FragCommand::FragSessionSetupAns(SetupStatus {
            unsupported_algorithm: flat.unsupported_algorithm.unwrap_or(false),
            not_enough_memory: flat.memory_error.unwrap_or(false),
            unsupported_index: flat.unsupported_index.unwrap_or(false),
            wrong_descriptor: flat.wrong_descriptor.unwrap_or(false),
            session_replay: flat.session_replay.unwrap_or(false),
            frag_index: flat.frag_index?,
        }),
        "frag_session_delete_req" => FragCommand::FragSessionDeleteReq {
            frag_index: flat.frag_index?,
        },
        "frag_session_delete_ans" => FragCommand::FragSessionDeleteAns {
            frag_index: flat.frag_index?,
            no_session: flat.no_session.unwrap_or(false),
        },
        "frag_data_block_received_req" => FragCommand::FragDataBlockReceivedReq {
            frag_index: flat.frag_index?,
            mic_error: flat.mic_error.unwrap_or(false),
        },
        "frag_data_block_received_ans" => FragCommand::FragDataBlockReceivedAns {
            frag_index: flat.frag_index?,
        },
        "data_fragment" => FragCommand::DataFragment {
            frag_index: flat.frag_index?,
            n: flat.fragment_n?,
            data,
        },
        _ => return None,
    })
}

/// Describes a multicast setup command the way Python holds it.
fn mc_out(command: McCommand) -> LorawanPackageCommand {
    let uplink = matches!(command.direction(), Direction::Uplink);
    let name = match command {
        McCommand::PackageVersionReq => "package_version_req",
        McCommand::PackageVersionAns(_) => "package_version_ans",
        McCommand::McGroupStatusReq { .. } => "mc_group_status_req",
        McCommand::McGroupStatusAns { .. } => "mc_group_status_ans",
        McCommand::McGroupStatusItem { .. } => "mc_group_status_item",
        McCommand::McGroupSetupReq { .. } => "mc_group_setup_req",
        McCommand::McGroupSetupAns { .. } => "mc_group_setup_ans",
        McCommand::McGroupDeleteReq { .. } => "mc_group_delete_req",
        McCommand::McGroupDeleteAns { .. } => "mc_group_delete_ans",
        McCommand::McClassCSessionReq { .. } => "mc_class_c_session_req",
        McCommand::McClassCSessionAns { .. } => "mc_class_c_session_ans",
        McCommand::McClassBSessionReq { .. } => "mc_class_b_session_req",
        McCommand::McClassBSessionAns { .. } => "mc_class_b_session_ans",
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
            flat.mc_key_encrypted = Some(mc_key_encrypted.to_vec());
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

/// Reads a multicast setup command out of the record Python holds.
fn mc_in(flat: &LorawanPackageCommand) -> Option<McCommand> {
    let status = SessionStatus {
        mc_group_id: flat.mc_group_id.unwrap_or(0),
        dr_error: flat.dr_error.unwrap_or(false),
        freq_error: flat.freq_error.unwrap_or(false),
        group_undefined: flat.no_session.unwrap_or(false),
        start_missed: flat.start_missed.unwrap_or(false),
    };
    Some(match flat.kind.as_str() {
        "package_version_req" => McCommand::PackageVersionReq,
        "package_version_ans" => McCommand::PackageVersionAns(PackageVersion {
            package: flat.package?,
            version: flat.version?,
        }),
        "mc_group_status_req" => McCommand::McGroupStatusReq {
            req_group_mask: flat.group_mask?,
        },
        "mc_group_status_ans" => McCommand::McGroupStatusAns {
            ans_group_mask: flat.group_mask?,
            nb_total_groups: flat.nb_total_groups.unwrap_or(0),
        },
        "mc_group_status_item" => McCommand::McGroupStatusItem {
            mc_group_id: flat.mc_group_id?,
            mc_addr: flat.mc_addr?,
        },
        "mc_group_setup_req" => McCommand::McGroupSetupReq {
            mc_group_id: flat.mc_group_id?,
            mc_addr: flat.mc_addr?,
            mc_key_encrypted: sixteen(&flat.mc_key_encrypted)?,
            min_mc_fcnt: flat.min_mc_fcnt.unwrap_or(0),
            max_mc_fcnt: flat.max_mc_fcnt.unwrap_or(0),
        },
        "mc_group_setup_ans" => McCommand::McGroupSetupAns {
            mc_group_id: flat.mc_group_id?,
            id_error: flat.id_error.unwrap_or(false),
        },
        "mc_group_delete_req" => McCommand::McGroupDeleteReq {
            mc_group_id: flat.mc_group_id?,
        },
        "mc_group_delete_ans" => McCommand::McGroupDeleteAns {
            mc_group_id: flat.mc_group_id?,
            group_undefined: flat.no_session.unwrap_or(false),
        },
        "mc_class_c_session_req" => McCommand::McClassCSessionReq {
            mc_group_id: flat.mc_group_id?,
            session_time: flat.session_time?,
            time_out: flat.time_out.unwrap_or(0),
            dl_frequency_hz: flat.dl_frequency_hz?,
            data_rate: flat.data_rate.unwrap_or(0),
        },
        "mc_class_c_session_ans" => McCommand::McClassCSessionAns {
            status,
            time_to_start: flat.time_to_start,
        },
        "mc_class_b_session_req" => McCommand::McClassBSessionReq {
            mc_group_id: flat.mc_group_id?,
            session_time: flat.session_time?,
            time_out: flat.time_out.unwrap_or(0),
            periodicity: flat.periodicity.unwrap_or(0),
            dl_frequency_hz: flat.dl_frequency_hz?,
            data_rate: flat.data_rate.unwrap_or(0),
        },
        "mc_class_b_session_ans" => McCommand::McClassBSessionAns {
            status,
            time_to_start: flat.time_to_start,
        },
        _ => return None,
    })
}
