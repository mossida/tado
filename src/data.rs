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
    ($(#[$attr:meta])* $enum_name:ident { $($variant:ident $( { $($field:ident : $field_type:ty),* } )? ),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Display)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
        $(#[$attr])*
        pub enum $enum_name {
            $(
                $variant $( { $($field : $field_type),* } )? ,
            )*
        }
    };
}

new_enum![StatePresence { Home, Away, Auto }];

new_enum![ZoneType {
    Heating,
    HotWater,
    AirConditioning
}];

new_type![Temperature {
    celsius: f32,
    fahrenheit: f32,
}];

new_type![DeviceConnectionState {
    value: bool,
    timestamp: String,
}];

new_type![DeviceUsageEntry {
    r#type: String,
    device: Device,
}];

new_type![DeviceUsage {
    entries: Vec<DeviceUsageEntry>
}];

new_type! [Device {
    device_type: String,
    serial_no: String,
    short_serial_no: String,
    current_fw_version: String,
    connection_state: DeviceConnectionState,
    child_lock_enabled: Option<bool>,
}];

new_type![MobileDeviceLocationBearingFromHome {
    degrees: f32,
    radians: f32,
}];

new_type![MobileDeviceLocation {
    stale: bool,
    at_home: bool,
    bearing_from_home: MobileDeviceLocationBearingFromHome,
    relative_distance_from_home_fence: f32,
}];

new_type![MobileDevicePushNotifications {
    low_battery_reminder: bool,
    away_mode_reminder: bool,
    home_mode_reminder: bool,
    open_window_reminder: bool,
    energy_savings_report_reminder: bool,
    incident_detection: bool,
}];

new_type![MobileDeviceSettings {
    geo_tracking_enabled: bool,
    on_demand_log_retrieval_enabled: bool,
    push_notifications: MobileDevicePushNotifications,
}];

new_type![MobileDeviceMetadata {
    platform: String,
    os_version: String,
    model: String,
    locale: String,
}];

new_type![MobileDevice {
    id: u32,
    name: String,
    settings: MobileDeviceSettings,
    location: Option<MobileDeviceLocation>,
    device_metadata: MobileDeviceMetadata,
}];

new_type![HomeState {
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
    mobile_devices: Vec<MobileDevice>,
}];

new_type![ZoneDazzleMode {
    enabled: bool,
    supported: bool,
}];

new_type![ZoneOpenWindowDetection {
    enabled: bool,
    supported: bool,
    timeout_in_seconds: u32,
}];

new_type![Zone {
    id: u32,
    name: String,
    r#type: ZoneType,
    device_types: Vec<String>,
    devices: Vec<Device>,
    date_created: String,
    report_available: bool,
    show_schedule_setup: bool,
    supports_dazzle: bool,
    dazzle_enabled: bool,
    dazzle_mode: ZoneDazzleMode,
    open_window_detection: ZoneOpenWindowDetection,
}];

new_type![ZoneState {
    geolocation_override: Option<bool>,
    geolocation_override_disable_time: Option<bool>,
}];

new_type![WeatherSolarIntensity {
    percentage: f32,
    timestamp: String,
}];

new_type![WeatherOutsideTemperature {
    celsius: f32,
    fahrenheit: f32,
    timestamp: String,
    //precision: Temperature
}];

new_enum![WeatherStateValue {
    Cloudy,
    CloudyPartly,
    CloudyMostly,
    NightCloudy,
    NightClear,
    Sun,
    ScatteredRain,
}];

new_type![WeatherState {
    value: WeatherStateValue,
    timestamp: String,
}];

new_type![Weather {
    solar_intensity: WeatherSolarIntensity,
    outside_temperature: WeatherOutsideTemperature,
    weather_state: WeatherState,
}];

new_type![EarlyStart { enabled: bool }];

new_type![HeatingCircuit {
    number: u32,
    driver_serial_no: String,
    driver_short_serial_no: String
}];

new_enum![TemperatureLevel { Cold, Comfy, Hot }];

new_enum![HumidityLevel { Dry, Comfy, Humid }];

new_type![AirComfortFreshness {
    value: String,
    last_open_window: String,
}];

new_type![AirComfortCoordinate {
    radial: f32,
    angular: f32,
}];

new_type![AirComfortRoom {
    room_id: u32,
    temperature_level: TemperatureLevel,
    humidity_level: HumidityLevel,
    coordinate: AirComfortCoordinate,
}];

new_type![AirComfort {
    freshness: AirComfortFreshness,
    comfort: Vec<AirComfortRoom>
}];

new_type![HeatingSystemBoiler {
    present: bool,
    id: u32,
    found: bool,
}];

new_type![HeatingSystemUnderfloorHeating { present: bool }];

new_type![HeatingSystem {
    boiler: HeatingSystemBoiler,
    underfloor_heating: HeatingSystemUnderfloorHeating,
}];

new_enum![AwayConfigurationPreheatingLevel {
    Low,
    Medium,
    Comfort
}];

new_type![AwayConfiguration {
    r#type: ZoneType,
    preheating_level: AwayConfigurationPreheatingLevel,
    minimum_away_temperature: Temperature
}];
