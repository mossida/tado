use std::time::Instant;

use cnf::AUTH_SCOPE;
use data::{
    AirComfort, AwayConfiguration, Device, DeviceUsage, EarlyStart, HeatingCircuit, HeatingSystem,
    Home, HomeState, MobileDevice, MobileDeviceSettings, StatePresence, Temperature, User, Weather,
    Zone, ZoneId, ZoneState,
};
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    AccessToken, AuthUrl, ClientId, ClientSecret, EmptyExtraTokenFields, ResourceOwnerPassword,
    ResourceOwnerUsername, Scope, StandardTokenResponse, TokenResponse, TokenUrl,
};
use reqwest::{Client as HttpClient, Method, RequestBuilder};
use serde_json::{json, Value};
use thiserror::Error;

use crate::cnf::{CLIENT_ID, CLIENT_SECRET};

mod cnf;
pub mod data;

macro_rules! response {
    ($self:expr, $method:expr, $url:tt) => {{
        let response = $self.build($method, &$url).await?.send().await?;
        Ok(response.json().await?)
    }};

    ($self:expr, $method:expr, $url:tt, $payload:tt) => {{
        let payload = json!($payload);
        let response = $self
            .build($method, &$url)
            .await?
            .json(&payload)
            .send()
            .await?;

        Ok(response.json().await?)
    }};
}

macro_rules! api {
    ($name:ident, $data:ty, $path:literal) => {
        pub async fn $name(&mut self) -> Result<$data, Error> {
            let url = concat!("https://my.tado.com/api/v2/", $path);
            let url = url.replace("{home}", &self.get_me()?.homes[0].id.to_string());

            response!(self, Method::GET, url)
        }
    };

    ($name:ident, $data:ty, $path:literal $(, $dyn_param:ident: $dyn_type:ty)*) => {
        pub async fn $name(&mut self $(, $dyn_param: $dyn_type)*) -> Result<$data, Error> {
            let template = concat!("https://my.tado.com/api/v2/", $path);
            let url = template.replace("{home}", &self.get_me()?.homes[0].id.to_string());

            $(let url = url.replace(concat!("{", stringify!($dyn_param), "}"), $dyn_param.to_string().as_str());)*

            response!(self, Method::GET, url)
        }
    };

    ($name:ident, $data:ty, $method:expr, $payload:tt, $path:literal $(, $dyn_param:ident: $dyn_type:ty)*) => {
        pub async fn $name(&mut self $(, $dyn_param: $dyn_type)*) -> Result<$data, Error> {
            let payload = json!($payload);
            let template = concat!("https://my.tado.com/api/v2/", $path);
            let url = template.replace("{home}", &self.get_me()?.homes[0].id.to_string());

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
    session: Option<AuthSession>,
    configuration: Configuration,
}

impl Client {
    async fn build(&mut self, method: Method, url: &str) -> Result<RequestBuilder, Error> {
        let token = self.token().await?;
        let builder = self.inner.request(method, url).bearer_auth(token.secret());

        Ok(builder)
    }

    async fn token(&mut self) -> Result<AccessToken, Error> {
        // TODO: Correctly handle errors
        match &mut self.session {
            Some(session) => {
                let AuthSession {
                    user: _,
                    token,
                    instant,
                } = session;

                if let Some(expiration) = token.expires_in() {
                    if instant.elapsed().as_secs() < expiration.as_secs() {
                        return Ok(token.access_token().clone());
                    } else {
                        *token = self
                            .oauth
                            .exchange_refresh_token(token.refresh_token().unwrap())
                            .request_async(async_http_client)
                            .await
                            .unwrap();

                        *instant = Instant::now();
                        return Ok(token.access_token().clone());
                    }
                } else {
                    return Err(Error::InvalidAuth);
                }
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
            AuthUrl::new("https://auth.tado.com/oauth/token".to_string()).unwrap(),
            Some(TokenUrl::new("https://auth.tado.com/oauth/token".to_string()).unwrap()),
        );

        Self {
            oauth,
            inner: HttpClient::new(),
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
            .unwrap();

        // Cannot use internal build as we didn't create the session yet
        let response = self
            .inner
            .request(Method::GET, "https://my.tado.com/api/v2/me")
            .bearer_auth(token.access_token().secret())
            .send()
            .await?;

        dbg!(token.access_token().secret());

        let user = response.json::<User>().await?;

        self.session = Some(AuthSession {
            user,
            token,
            instant: Instant::now(),
        });

        Ok(())
    }
}

impl Client {
    pub fn get_me(&self) -> Result<&User, Error> {
        self.session
            .as_ref()
            .ok_or(Error::InvalidAuth)
            .map(|session| &session.user)
    }

    api!(get_home, Home, "homes/{home}");

    api!(get_home_state, HomeState, "homes/{home}/state");

    api!(get_weather, Weather, "homes/{home}/weather");

    api!(get_devices, Vec<Device>, "homes/{home}/devices");

    api!(get_device_usage, DeviceUsage, "homes/{home}/deviceList");

    // TODO: Add return type
    api!(get_invitations, Vec<Value>, "homes/{home}/invitations");

    // TODO: Add return type
    api!(set_invitation, (), Method::POST, {
        "email": email
    }, "homes/{home}/invitations", email: String);

    // TODO: Add return type
    api!(delete_invitation, (), Method::DELETE, null, "homes/{home}/invitations/{invitation}", token: String);

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

    api!(end_manual_control, (), Method::DELETE, null, "homes/{home}/zones/{zone}/overlay", zone: &ZoneId);

    api!(get_zone_states, Vec<ZoneState>, "homes/{home}/zoneStates");

    api!(
        get_heating_circuits,
        Vec<HeatingCircuit>,
        "homes/{home}/heatingCircuits"
    );

    // TODO: Type unknown
    // TODO: Use minder API
    api!(get_incidents, Value, "homes/{home}/incidents");

    // TODO: Add return type
    api!(set_incident_detection, (), Method::PUT, {
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

    api!(set_open_window_detection, (), Method::PUT, {
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

    // TODO: Add return type
    api!(set_presence, Value, Method::PUT, {
        "homePresence": presence
    }, "homes/{home}/presenceLock", presence: StatePresence);

    // TODO: Add return type
    api!(set_identify, Value, Method::POST, null, "devices/{device}/identify", device: &String);

    // TODO: Add return type
    api!(set_child_lock, (), Method::PUT, {
        "childLockEnabled": enabled
    }, "devices/{device}/childLock", device: String, enabled: bool);
}
