//! Python bindings for the pamoja SDK, generated with PyO3.
//!
//! This crate is the generated low-level surface (the contract tier): it exposes
//! the Rust core and capability crates to Python one-to-one. A hand-written,
//! idiomatic facade wraps it for everyday use; see the `pamoja` Python package.
//!
//! The native module is imported as `pamoja._native` and re-exported verbatim at
//! `pamoja.raw`.

use pyo3::prelude::*;
use pyo3_stub_gen::{define_stub_info_gatherer, derive::gen_stub_pyfunction};

#[cfg(feature = "actuators")]
mod actuators;
#[cfg(feature = "audit")]
mod audit;
#[cfg(feature = "bus")]
mod bus;
#[cfg(feature = "can")]
mod can;
#[cfg(feature = "coap")]
mod coap;
#[cfg(feature = "codec")]
mod codec;
#[cfg(feature = "gpio")]
mod gpio;
#[cfg(feature = "kit")]
mod kit;
#[cfg(feature = "ladder")]
mod ladder;
#[cfg(feature = "loopback")]
mod loopback;
#[cfg(feature = "lora")]
mod lora;
#[cfg(feature = "lora")]
mod lora_region;
#[cfg(feature = "lorawan")]
mod lorawan;
#[cfg(feature = "mavlink")]
mod mavlink;
#[cfg(feature = "mavlink")]
mod mavlink_protocol;
#[cfg(feature = "mavlink")]
mod mavlink_schema;
#[cfg(feature = "mesh")]
mod mesh;
#[cfg(feature = "modbus")]
mod modbus;
#[cfg(feature = "mqtt")]
mod mqtt;
#[cfg(feature = "power")]
mod power;
#[cfg(feature = "profile")]
mod profile;
#[cfg(feature = "ros2")]
mod ros2;
#[cfg(feature = "routing")]
mod routing;
#[cfg(feature = "security")]
mod security;
#[cfg(feature = "sensors")]
mod sensors;
#[cfg(feature = "serial")]
mod serial;
#[cfg(feature = "session")]
mod session;
#[cfg(feature = "sim")]
mod sim;
#[cfg(feature = "sync")]
mod sync;
#[cfg(feature = "telemetry")]
mod telemetry;
#[cfg(feature = "mqtt")]
mod transport;
#[cfg(feature = "update")]
mod update;
#[cfg(feature = "zenoh")]
mod zenoh;

pyo3::create_exception!(
    pamoja,
    PamojaError,
    pyo3::exceptions::PyException,
    "Raised when a pamoja operation fails."
);

/// Returns the version of the native pamoja module.
#[gen_stub_pyfunction]
#[pyfunction]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The generated low-level Python surface for the pamoja core.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add("PamojaError", m.py().get_type::<PamojaError>())?;
    #[cfg(feature = "mqtt")]
    {
        m.add_class::<mqtt::MqttClient>()?;
        m.add_class::<mqtt::MqttMessage>()?;
    }
    #[cfg(feature = "security")]
    {
        m.add_class::<security::DeviceIdentity>()?;
        m.add_function(wrap_pyfunction!(security::verify, m)?)?;
        m.add_function(wrap_pyfunction!(security::verify_message, m)?)?;
        m.add_function(wrap_pyfunction!(security::fingerprint, m)?)?;
    }
    #[cfg(feature = "codec")]
    {
        m.add_class::<codec::Quantizer>()?;
        m.add_function(wrap_pyfunction!(codec::json_to_cbor_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(codec::cbor_to_json_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(codec::encode_delta_samples, m)?)?;
        m.add_function(wrap_pyfunction!(codec::decode_delta_samples, m)?)?;
    }
    #[cfg(feature = "kit")]
    {
        m.add_class::<kit::Smoother>()?;
        m.add_class::<kit::Pid>()?;
        m.add_class::<kit::Thermostat>()?;
        m.add_class::<kit::Depletion>()?;
        m.add_class::<kit::Kalman>()?;
        m.add_class::<kit::Debounce>()?;
        m.add_class::<kit::Ramp>()?;
        m.add_class::<kit::Surge>()?;
        m.add_class::<kit::Calibration>()?;
        m.add_class::<kit::Geofence>()?;
        m.add_function(wrap_pyfunction!(kit::distance_between, m)?)?;
        m.add_function(wrap_pyfunction!(kit::bearing_between, m)?)?;
        m.add_function(wrap_pyfunction!(kit::deadband, m)?)?;
        m.add_class::<kit::Window>()?;
        m.add_class::<kit::Median>()?;
        m.add_class::<kit::Trend>()?;
        m.add_class::<kit::Anomaly>()?;
        m.add_function(wrap_pyfunction!(kit::window_capacity, m)?)?;
    }
    #[cfg(feature = "serial")]
    {
        m.add_class::<serial::SlipDecoder>()?;
        m.add_class::<serial::CobsDecoder>()?;
        m.add_function(wrap_pyfunction!(serial::slip_encode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::slip_decode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::cobs_encode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::cobs_decode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::serial_framing_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(serial::slip_max_encoded_len, m)?)?;
        m.add_function(wrap_pyfunction!(serial::cobs_max_encoded_len, m)?)?;
    }
    #[cfg(feature = "modbus")]
    {
        m.add_class::<modbus::ModbusFrame>()?;
        m.add_function(wrap_pyfunction!(modbus::modbus_crc16, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_coils, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_discrete_inputs, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_holding_registers, m)?)?;
        m.add_function(wrap_pyfunction!(
            modbus::modbus_read_holding_registers_reply,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_input_registers_reply, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_input_registers, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_write_single_coil, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_write_single_register, m)?)?;
        m.add_function(wrap_pyfunction!(
            modbus::modbus_write_multiple_registers,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_write_multiple_coils, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_raw, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_parse_frame, m)?)?;
    }
    #[cfg(feature = "can")]
    {
        m.add_class::<can::CanFrame>()?;
        m.add_class::<can::CanSignals>()?;
        m.add_class::<can::J1939Message>()?;
        m.add_function(wrap_pyfunction!(can::can_frame, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_fd_frame, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_remote_frame, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_len_to_dlc, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_dlc_to_len, m)?)?;
        m.add_function(wrap_pyfunction!(can::j1939_decode, m)?)?;
        m.add_function(wrap_pyfunction!(can::j1939_compose, m)?)?;
        m.add_function(wrap_pyfunction!(can::j1939_broadcast, m)?)?;
        m.add_function(wrap_pyfunction!(can::j1939_limits, m)?)?;
    }
    #[cfg(feature = "gpio")]
    {
        m.add_class::<gpio::SpiClock>()?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_frame, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_frame_len, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_is_reserved, m)?)?;
        m.add("I2C_RESERVED_FROM", pamoja_gpio::i2c::RESERVED_FROM)?;
        m.add("I2C_RESERVED_BELOW", pamoja_gpio::i2c::RESERVED_BELOW)?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_is_general_call, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::spi_mode_clock, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::spi_mode_from_clock, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_level_inverted, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_level_from_bool, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_edge_triggered_by, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_polarity_level, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_polarity_is_asserted, m)?)?;
    }
    #[cfg(feature = "sensors")]
    {
        m.add_class::<sensors::Bme280Calibration>()?;
        m.add_class::<sensors::Bme280Measurement>()?;
        m.add_class::<sensors::Ds18b20Reading>()?;
        m.add_class::<sensors::Ads1115Config>()?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_parse_scratchpad, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_build_scratchpad, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_crc8, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_micro_celsius, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_celsius, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_config_byte, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_resolution_bits, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_step_micro_celsius, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ds18b20_max_conversion_micros, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_calibration, m)?)?;
        m.add_function(wrap_pyfunction!(
            sensors::ina219_minimum_current_lsb_microamps,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_shunt_register, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_bus_register, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_current_register, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_power_register, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_shunt_microvolts, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_bus_millivolts, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_conversion_ready, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_math_overflow, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_current_microamps, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ina219_power_microwatts, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ads1115_config_bits, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ads1115_config_from_bits, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ads1115_full_scale_microvolts, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ads1115_samples_per_second, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ads1115_to_nanovolts, m)?)?;
        m.add_function(wrap_pyfunction!(sensors::ads1115_to_volts, m)?)?;
    }
    #[cfg(feature = "actuators")]
    {
        m.add_class::<actuators::Stepper>()?;
        m.add_function(wrap_pyfunction!(actuators::pca9685_limits, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::pca9685_channel_register, m)?)?;
        m.add_function(wrap_pyfunction!(
            actuators::pca9685_prescale_for_frequency,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(
            actuators::pca9685_frequency_for_prescale,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(actuators::pwm_from_counts, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::pwm_duty, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::pwm_servo, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::pwm_counts, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::pwm_full_on, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::pwm_full_off, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::stepper_step_count, m)?)?;
        m.add_function(wrap_pyfunction!(actuators::stepper_steps_for_degrees, m)?)?;
    }
    #[cfg(feature = "lora")]
    {
        m.add_class::<lora::LoraLink>()?;
        m.add_class::<lora_region::ChannelPlan>()?;
        m.add_class::<lora_region::ChannelPlanBuilder>()?;
        m.add_class::<lora_region::LoraDataRate>()?;
        m.add_class::<lora_region::LoraMaxPayload>()?;
        m.add_class::<lora_region::LoraChannelBlock>()?;
        m.add_class::<lora_region::LoraSubBand>()?;
        m.add_class::<lora_region::LoraBeacon>()?;
        m.add_class::<lora_region::LoraPlanInfo>()?;
    }
    #[cfg(feature = "mesh")]
    {
        m.add_class::<mesh::MeshFrame>()?;
        m.add_class::<mesh::SeenPackets>()?;
        m.add_function(wrap_pyfunction!(mesh::mesh_frame, m)?)?;
        m.add_function(wrap_pyfunction!(mesh::mesh_broadcast_frame, m)?)?;
        m.add_function(wrap_pyfunction!(mesh::mesh_parse_frame, m)?)?;
        m.add_function(wrap_pyfunction!(mesh::mesh_relayed, m)?)?;
        m.add_function(wrap_pyfunction!(mesh::mesh_crc16, m)?)?;
        m.add_function(wrap_pyfunction!(mesh::mesh_limits, m)?)?;
    }
    #[cfg(feature = "routing")]
    {
        m.add_class::<routing::Router>()?;
        m.add_class::<routing::Route>()?;
        m.add_class::<routing::ForwardDecision>()?;
        m.add_function(wrap_pyfunction!(routing::routing_default_capacity, m)?)?;
    }
    #[cfg(feature = "audit")]
    {
        m.add_class::<audit::AuditEntry>()?;
        m.add_class::<audit::AuditLog>()?;
        m.add_class::<audit::AuditVerifier>()?;
        m.add_function(wrap_pyfunction!(audit::verify_audit_chain, m)?)?;
    }
    #[cfg(feature = "session")]
    {
        m.add_class::<session::AgreementKey>()?;
        m.add_class::<session::Session>()?;
        m.add_class::<session::SealedMessage>()?;
        m.add_function(wrap_pyfunction!(session::hmac_sha256_digest, m)?)?;
        m.add_function(wrap_pyfunction!(session::hkdf_sha256_expand, m)?)?;
    }
    #[cfg(feature = "update")]
    {
        m.add_class::<update::Manifest>()?;
        m.add_class::<update::Delegation>()?;
        m.add_class::<update::SlotRecord>()?;
        m.add_class::<update::BootDecision>()?;
        m.add_class::<update::Progress>()?;
        m.add_class::<update::ImageVerifier>()?;
        m.add_class::<update::Updater>()?;
        m.add_function(wrap_pyfunction!(update::encode_manifest, m)?)?;
        m.add_function(wrap_pyfunction!(update::decode_manifest, m)?)?;
        m.add_function(wrap_pyfunction!(update::image_digest, m)?)?;
        m.add_function(wrap_pyfunction!(update::sign_manifest, m)?)?;
        m.add_function(wrap_pyfunction!(update::verify_envelope, m)?)?;
        m.add_function(wrap_pyfunction!(update::envelope_body, m)?)?;
        m.add_function(wrap_pyfunction!(update::sign_delegation, m)?)?;
        m.add_function(wrap_pyfunction!(update::open_delegation, m)?)?;
        m.add_function(wrap_pyfunction!(update::update_structure_version, m)?)?;
        m.add_function(wrap_pyfunction!(update::update_format_raw, m)?)?;
    }
    #[cfg(feature = "power")]
    {
        m.add_class::<power::DutyCycle>()?;
        m.add_class::<power::PowerPlan>()?;
    }
    #[cfg(feature = "coap")]
    {
        m.add_class::<coap::CoapClient>()?;
    }
    #[cfg(feature = "loopback")]
    {
        m.add_class::<loopback::LoopbackBroker>()?;
        m.add_class::<loopback::LoopbackTransport>()?;
    }
    #[cfg(feature = "sync")]
    {
        m.add_class::<sync::Store>()?;
    }
    #[cfg(feature = "ladder")]
    {
        m.add_class::<ladder::Ladder>()?;
    }
    #[cfg(feature = "bus")]
    {
        m.add_class::<bus::EventBus>()?;
    }
    #[cfg(feature = "sim")]
    {
        m.add_class::<sim::SimulatedSensor>()?;
        m.add_class::<sim::Replay>()?;
        m.add_class::<sim::RecordingActuatorHandle>()?;
        m.add_class::<sim::SimulatedRobot>()?;
        m.add_class::<sim::Pose>()?;
    }
    #[cfg(feature = "mqtt")]
    {
        m.add_class::<transport::PyTransport>()?;
        m.add_class::<transport::Message>()?;
    }
    #[cfg(feature = "telemetry")]
    {
        m.add_class::<telemetry::Reporter>()?;
        m.add_class::<telemetry::Snapshot>()?;
        m.add_function(wrap_pyfunction!(telemetry::link_cost_threshold, m)?)?;
    }
    #[cfg(feature = "lorawan")]
    {
        m.add_class::<lorawan::LorawanSession>()?;
        m.add_class::<lorawan::LorawanDevice>()?;
        m.add_class::<lorawan::LorawanJoinAccept>()?;
        m.add_class::<lorawan::LorawanRxData>()?;
        m.add_class::<lorawan::LorawanHeader>()?;
        m.add_class::<lorawan::LorawanJoinRequest>()?;
        m.add_class::<lorawan::LorawanGrant>()?;
        m.add_function(wrap_pyfunction!(lorawan::lorawan_parse_header, m)?)?;
        m.add_function(wrap_pyfunction!(lorawan::lorawan_parse_join_request, m)?)?;
    }
    #[cfg(feature = "mavlink")]
    {
        m.add_class::<mavlink::MavlinkHeader>()?;
        m.add_class::<mavlink::MavlinkFrame>()?;
        m.add_class::<mavlink::MavlinkParser>()?;
        m.add_class::<mavlink::MavlinkSigner>()?;
        m.add_class::<mavlink::MavlinkVerifier>()?;
        m.add_class::<mavlink::Dialect>()?;
        m.add_function(wrap_pyfunction!(mavlink::mavlink_crc16_mcrf4xx, m)?)?;
        m.add_function(wrap_pyfunction!(mavlink::mavlink_message_crc_extra, m)?)?;
        m.add_function(wrap_pyfunction!(mavlink::mavlink_known_crc_extra, m)?)?;
        m.add_function(wrap_pyfunction!(
            mavlink::mavlink_timestamp_from_unix_micros,
            m
        )?)?;
        m.add_class::<mavlink_schema::MavlinkFieldInfo>()?;
        m.add_class::<mavlink_schema::MessageSchema>()?;
        m.add_class::<mavlink_schema::MessageSchemaBuilder>()?;
        m.add_class::<mavlink_schema::MavlinkMessage>()?;
        m.add_function(wrap_pyfunction!(mavlink_schema::mavlink_known_messages, m)?)?;
        m.add_class::<mavlink_protocol::ReceiverStep>()?;
        m.add_class::<mavlink_protocol::MissionReceiver>()?;
        m.add_class::<mavlink_protocol::SenderStep>()?;
        m.add_class::<mavlink_protocol::MissionSender>()?;
        m.add_class::<mavlink_protocol::AckOutcome>()?;
        m.add_class::<mavlink_protocol::CommandProtocol>()?;
        m.add_function(wrap_pyfunction!(
            mavlink_protocol::mavlink_offboard_type_mask,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(
            mavlink_protocol::mavlink_offboard_local_position,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(
            mavlink_protocol::mavlink_offboard_local_velocity,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(
            mavlink_protocol::mavlink_offboard_global_position,
            m
        )?)?;
    }
    #[cfg(feature = "profile")]
    {
        m.add_class::<profile::Profile>()?;
        m.add_class::<profile::Controller>()?;
        m.add_class::<profile::ControlPolicy>()?;
        m.add_class::<profile::PowerScheduleSpec>()?;
        m.add_class::<profile::AlertReport>()?;
        m.add_class::<profile::Reaction>()?;
    }
    #[cfg(feature = "ros2")]
    {
        m.add_class::<ros2::CdrWriter>()?;
        m.add_class::<ros2::CdrReader>()?;
        m.add_function(wrap_pyfunction!(ros2::ros2_is_valid_name, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_is_fully_qualified, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_entity_kind_prefix, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_dds_topic, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_percent_mangle, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_dds_type_name, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_type_hash_digest, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_entity_key, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_twist_to_cdr, m)?)?;
        m.add_function(wrap_pyfunction!(ros2::ros2_twist_from_cdr, m)?)?;
    }
    #[cfg(feature = "zenoh")]
    {
        m.add_function(wrap_pyfunction!(zenoh::keyexpr_is_valid, m)?)?;
        m.add_function(wrap_pyfunction!(zenoh::keyexpr_is_canon, m)?)?;
        m.add_function(wrap_pyfunction!(zenoh::keyexpr_canonize, m)?)?;
        m.add_function(wrap_pyfunction!(zenoh::keyexpr_matches, m)?)?;
    }
    Ok(())
}

define_stub_info_gatherer!(stub_info);
