use serde::{Deserialize, Serialize};
use strum::Display;

macro_rules! new_type {
    ($name:ident { $($(#[$type_attr:meta])* $field_name:ident : $field_type:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            $($(#[$type_attr])* pub $field_name: $field_type,)*
        }
    };
}

macro_rules! new_enum {
    ($enum_name:ident { $($variant_name:ident $(($($variant_field:ident : $variant_field_type:ty),* $(,)?)?)?),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Display)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
        pub enum $enum_name {
            $(
                $variant_name $({
                    $(
                        $variant_field: $variant_field_type,
                    )*
                })*,
            )*
        }
    };
}

new_enum![StatePresence { Home, Away, Auto }];

new_type![DeviceConnectionState {
    value: bool,
    timestamp: String,
}];

new_type! [Device {
    device_type: String,
    serial_no: String,
    short_serial_no: String,
    current_fw_version: String,
    connection_state: DeviceConnectionState,
    child_lock_enabled: Option<bool>,
}];

new_type![State {
    presence: StatePresence,
    presence_locked: bool,
}];

new_type![IncidentDetection {
    enabled: bool,
    supported: bool,
}];

new_type![BasicHome {
    id: u32,
    name: String,
}];

new_type![Home {
    #[serde(flatten)]
    basic: BasicHome,
    date_time_zone: String,
    date_created: String,
    temperature_unit: String,
    partner: Option<String>,
    simple_smart_schedule_enabled: bool,
    away_radius_in_meters: f32,
    installation_completed: bool,
    incident_detection: IncidentDetection,
    zones_count: u32,
    christmas_mode_enabled: bool,
    show_auto_assist_reminders: bool,
    consent_grant_skippable: bool,
    is_air_comfort_eligible: bool,
    is_balance_ac_eligible: bool,
    is_balance_hp_eligible: bool,
    is_energy_iq_eligible: bool,
    is_heat_source_installed: bool,
}];

new_type![User {
    id: String,
    name: String,
    email: String,
    username: String,
    homes: Vec<BasicHome>,
}];
