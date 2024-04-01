use std::time::Instant;

use cnf::{API_URL, AUTH_SCOPE, AUTH_URL};
use data::{
    AirComfort, AwayConfiguration, Device, DeviceUsage, EarlyStart, HeatingCircuit, HeatingSystem,
    Home, HomeState, Invitation, MobileDevice, MobileDeviceSettings, StatePresence, Temperature,
    User, Weather, Zone, ZoneId, ZoneState,
};
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    AccessToken, AuthUrl, ClientId, ClientSecret, EmptyExtraTokenFields, ResourceOwnerPassword,
    ResourceOwnerUsername, Scope, StandardTokenResponse, TokenResponse, TokenUrl,
};
use reqwest::{Client as HttpClient, ClientBuilder, Method, RequestBuilder};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::cnf::{CLIENT_ID, CLIENT_SECRET};

mod cnf;
pub mod data;

macro_rules! response {
    ($self:expr, $method:expr, $url:tt) => {{
        let token = $self.token().await?;
        let response = $self.build($method, &$url, &token).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::UnsuccesfulOperation(response.json().await?))
        }
    }};

    ($self:expr, $method:expr, $url:tt, $payload:tt) => {{
        let payload = json!($payload);
        let token = $self.token().await?;
        let response = $self
            .build($method, &$url, &token)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::UnsuccesfulOperation(response.json().await?))
        }
    }};
}

macro_rules! api {
    ($name:ident, $data:ty, $path:literal) => {
        pub async fn $name(&self) -> Result<$data, Error> {
            let url = format!("{}{}", API_URL, $path);
            let url = url.replace("{home}", &self.get_me().await?.homes[0].id.to_string());

            response!(self, Method::GET, url)
        }
    };

    ($name:ident, $data:ty, $path:literal $(, $dyn_param:ident: $dyn_type:ty)*) => {
        pub async fn $name(&self $(, $dyn_param: $dyn_type)*) -> Result<$data, Error> {
            let template = format!("{}{}", API_URL, $path);
            let url = template.replace("{home}", &self.get_me().await?.homes[0].id.to_string());

            $(let url = url.replace(concat!("{", stringify!($dyn_param), "}"), $dyn_param.to_string().as_str());)*

            response!(self, Method::GET, url)
        }
    };

    // Special case where we don't need to return anything
    // So json deserialize is not needed otherwise it would fail
    ($name:ident, $method:expr, $payload:tt, $path:literal $(, $dyn_param:ident: $dyn_type:ty)*) => {
        pub async fn $name(&self $(, $dyn_param: $dyn_type)*) -> Result<(), Error> {
            let payload = json!($payload);
            let template = format!("{}{}", API_URL, $path);
            let url = template.replace("{home}", &self.get_me().await?.homes[0].id.to_string());

            $(let url = url.replace(concat!("{", stringify!($dyn_param), "}"), $dyn_param.to_string().as_str());)*

            let token = self.token().await?;
            let response = self
                .build($method, &url, &token)
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                // Still consume the response
                let _ = response.text().await?;

                Ok(())
            } else {
                Err(Error::UnsuccesfulOperation(response.json().await?))
            }
        }
    };

    ($name:ident, $data:ty, $method:expr, $payload:tt, $path:literal $(, $dyn_param:ident: $dyn_type:ty)*) => {
        pub async fn $name(&self $(, $dyn_param: $dyn_type)*) -> Result<$data, Error> {
            let payload = json!($payload);
            let template = format!("{}{}", API_URL, $path);
            let url = template.replace("{home}", &self.get_me().await?.homes[0].id.to_string());

            $(let url = url.replace(concat!("{", stringify!($dyn_param), "}"), $dyn_param.to_string().as_str());)*

            response!(self, $method, url, payload)
        }
    };
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Something went wrong when requesting data: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Invalid authentication, are you sure you made login?")]
    InvalidAuth,
    #[error("Failed to execute operation: {0}")]
    UnsuccesfulOperation(Value),
}

pub struct Auth {
    pub username: String,
    pub password: String,
}

pub struct Configuration {
    pub auth: Auth,
}

struct AuthSession {
    user: User,
    token: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    instant: Instant,
}

pub struct Client {
    inner: HttpClient,
    oauth: BasicClient,
    session: Option<Mutex<AuthSession>>,
    configuration: Configuration,
}

impl Client {
    fn build(&self, method: Method, url: &str, token: &AccessToken) -> RequestBuilder {
        self.inner.request(method, url).bearer_auth(token.secret())
    }

    async fn token(&self) -> Result<AccessToken, Error> {
        match &self.session {
            Some(session_lock) => {
                let mut session = session_lock.lock().await;

                match session.token.expires_in() {
                    Some(expiration)
                        if session.instant.elapsed().as_secs() < expiration.as_secs() =>
                    {
                        session.token = self
                            .oauth
                            .exchange_refresh_token(session.token.refresh_token().unwrap())
                            .request_async(async_http_client)
                            .await
                            .map_err(|_| Error::InvalidAuth)?;

                        session.instant = Instant::now();
                    }
                    _ => {}
                };

                Ok(session.token.access_token().clone())
            }
            _ => Err(Error::InvalidAuth),
        }
    }
}

impl Client {
    pub fn new(configuration: Configuration) -> Self {
        let oauth = BasicClient::new(
            ClientId::new(CLIENT_ID.to_string()),
            Some(ClientSecret::new(CLIENT_SECRET.to_string())),
            // Safe to unwrap as we know the URL is correct
            AuthUrl::new(AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(AUTH_URL.to_string()).unwrap()),
        );

        let builder = ClientBuilder::new();

        Self {
            oauth,
            inner: builder.build().unwrap(),
            session: None,
            configuration,
        }
    }

    pub async fn login(&mut self) -> Result<(), Error> {
        let token = self
            .oauth
            .exchange_password(
                &ResourceOwnerUsername::new(self.configuration.auth.username.clone()),
                &ResourceOwnerPassword::new(self.configuration.auth.password.clone()),
            )
            .add_scope(Scope::new(AUTH_SCOPE.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|_| Error::InvalidAuth)?;

        let response = self
            .build(
                Method::GET,
                format!("{}me", API_URL).as_str(),
                token.access_token(),
            )
            .send()
            .await?;

        let user = response.json::<User>().await?;

        self.session = Some(Mutex::new(AuthSession {
            user,
            token,
            instant: Instant::now(),
        }));

        Ok(())
    }
}

impl Client {
    pub async fn get_me(&self) -> Result<User, Error> {
        let session_lock = self.session.as_ref().ok_or(Error::InvalidAuth)?;
        let session = session_lock.lock().await;

        Ok(session.user.clone())
    }

    api!(get_home, Home, "homes/{home}");

    api!(get_home_state, HomeState, "homes/{home}/state");

    api!(get_weather, Weather, "homes/{home}/weather");

    api!(get_devices, Vec<Device>, "homes/{home}/devices");

    api!(get_device_usage, DeviceUsage, "homes/{home}/deviceList");

    api!(get_invitations, Vec<Invitation>, "homes/{home}/invitations");

    api!(set_invitation, Invitation, Method::POST, {
        "email": email
    }, "homes/{home}/invitations", email: String);

    api!(delete_invitation, Method::DELETE, null, "homes/{home}/invitations/{invitation}", token: String);

    api!(
        get_mobile_devices,
        Vec<MobileDevice>,
        "homes/{home}/mobileDevices"
    );

    api!(
        get_mobile_device_settings,
        MobileDeviceSettings,
        "homes/{home}/mobileDevices/{device}/settings",
        device: String
    );

    api!(get_users, Vec<User>, "homes/{home}/users");

    api!(get_zones, Vec<Zone>, "homes/{home}/zones");

    api!(get_early_start, EarlyStart, "homes/{home}/zones/{zone}/earlyStart", zone: &ZoneId);

    api!(set_early_start, EarlyStart, Method::PUT, {
        "enabled": enabled
    }, "homes/{home}/zones/{zone}/earlyStart", zone: &ZoneId, enabled: bool);

    api!(end_manual_control, Method::DELETE, null, "homes/{home}/zones/{zone}/overlay", zone: &ZoneId);

    api!(get_zone_states, Vec<ZoneState>, "homes/{home}/zoneStates");

    api!(
        get_heating_circuits,
        Vec<HeatingCircuit>,
        "homes/{home}/heatingCircuits"
    );

    // TODO: Type unknown
    // TODO: Use minder API
    api!(get_incidents, Value, "homes/{home}/incidents");

    api!(set_incident_detection, Method::PUT, {
        "enabled": enabled
    }, "homes/{home}/incidentDetection", enabled: bool);

    // TODO: Type unknown
    api!(get_installtions, Value, "homes/{home}/installations");

    api!(get_air_comfort, AirComfort, "homes/{home}/airComfort");

    api!(
        get_heating_system,
        HeatingSystem,
        "homes/{home}/heatingSystem"
    );

    api!(
        get_temperature_offset,
        Temperature,
        "devices/{device}/temperatureOffset",
        device: String
    );

    api!(
        set_temperature_offset,
        Temperature,
        Method::PUT,
        {
            "celsius": offset
        },
        "devices/{device}/temperatureOffset",
        device: &String,
        offset: f32
    );

    api!(
        get_away_configuration,
        AwayConfiguration,
        "homes/{home}/zones/{zone}/awayConfiguration",
        zone: &ZoneId
    );

    api!(set_open_window_detection, Method::PUT, {
        "enabled": enabled,
        "timeoutInSeconds": timeout,
    }, "homes/{home}/zones/{zone}/openWindowDetection", zone: &ZoneId, enabled: bool, timeout: u32);

    // TODO: Type unknown
    api!(
        get_default_overlay,
        Value,
        "homes/{home}/zones/{zone}/defaultOverlay",
        zone: &ZoneId
    );

    api!(get_measuring_device, String, "homes/{home}/zones/{zone}/measuringDevice", zone: &ZoneId);

    api!(get_state, ZoneState, "homes/{home}/zones/{zone}/state", zone: &ZoneId);

    // TODO: Add return type
    api!(set_zone_name, (), Method::PUT, {
        "name": name
    }, "homes/{home}/zones/{zone}/name", zone: &ZoneId, name: String);

    api!(get_schedule, String, "homes/{home}/zones/{zone}/schedule/activeTimetable", zone: &ZoneId);

    // TODO: Add return type
    api!(set_schedule, (), Method::PUT, {
        "id": schedule
    }, "homes/{home}/zones/{zone}/schedule/activeTimetable", zone: &ZoneId, schedule: u32);

    api!(get_schedule_timetables, String, "homes/{home}/zones/{zone}/schedule/timetables", zone: &u32);

    api!(set_presence, Method::PUT, {
        "homePresence": presence
    }, "homes/{home}/presenceLock", presence: StatePresence);

    api!(set_identify, Method::POST, null, "devices/{device}/identify", device: &String);

    api!(set_child_lock, Method::PUT, {
        "childLockEnabled": enabled
    }, "devices/{device}/childLock", device: &String, enabled: bool);
}
