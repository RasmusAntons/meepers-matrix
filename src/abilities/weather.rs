use std::collections::HashMap;
use std::ops::Deref;
use std::os::unix::raw::time_t;
use crate::abilities::Ability;
use anyhow::{Result, Error};
use clap::{arg, ArgMatches, Command};
use futures::FutureExt;
use matrix_sdk::Room;
use matrix_sdk::ruma::events::room::message::{AddMentions, ForwardThread, OriginalSyncRoomMessageEvent, RoomMessageEventContent};
use reqwest::{StatusCode, Url};
use nest_struct::nest_struct;
use rusqlite::fallible_iterator::FallibleIterator;
use serde::{Deserialize, Serialize};
use uom::si::f64::ThermodynamicTemperature;
use uom::si::thermodynamic_temperature;
use crate::config;
use crate::config::OWM_API_KEY;

static OWM_GEOCODING_ZIP_API: &str = "https://api.openweathermap.org/geo/1.0/zip";
static OWM_GEOCODING_DIRECT_API: &str = "https://api.openweathermap.org/geo/1.0/direct";
static OWM_CURRENT_WEATHER_API: &str = "https://api.openweathermap.org/data/2.5/weather";


#[derive(Debug, Serialize, Deserialize)]
struct Location {
    name: String,
    zip: Option<String>,
    local_names: Option<HashMap<String, String>>,
    lat: f64,
    lon: f64,
    country: String,
    state: Option<String>,
}

async fn resolve_location(name: String, zip: bool, api_key: String) -> Result<Vec<Location>> {
    let url = match zip {
        true => Url::parse_with_params(
            OWM_GEOCODING_ZIP_API,
            &[
                ("zip", name.as_str()),
                ("appid", api_key.as_str()),
            ]
        )?,
        false => Url::parse_with_params(
            OWM_GEOCODING_DIRECT_API,
            &[
                ("q", name.as_str()),
                ("appid", api_key.as_str()),
            ]
        )?
    };
    let resp = reqwest::get(url).await?;
    match resp.status() {
        StatusCode::OK => {
            let text = resp.text().await?;
            if zip {
                Ok(vec![serde_json::from_str::<Location>(text.as_str())?])
            } else {
                Ok(serde_json::from_str::<Vec<Location>>(text.as_str())?)
            }
        },
        _ => Err(Error::msg("failed to resolve location")),
    }
}


#[nest_struct]
#[derive(Debug, Serialize, Deserialize)]
struct CurrentWeather {
    coord: nest! {
        lon: f64,
        lat: f64,
    },
    weather: Vec<nest! {
        id: i64,
        main: String,
        description: String,
        icon: String,
    }>,
    base: String,
    main: nest! {
        temp: f64,
        feels_like: f64,
        temp_min: f64,
        temp_max: f64,
        pressure: f64,
        humidity: f64,
        sea_level: f64,
        grnd_level: f64,
    },
    visibility: f64,
    wind: nest! {
        speed: f64,
        deg: f64,
        gust: f64,
    },
    rain: Option<nest! {
        #[serde(rename="1h")]
        hour: f64
    }>,
    snow: Option<nest! {
        #[serde(rename="1h")]
        hour: f64
    }>,
    dt: time_t,
    sys: nest! {
        #[serde(rename="type")]
        sys_type: Option<i64>,
        id: Option<i64>,
        country: String,
        sunrise: time_t,
        sunset: time_t,
    },
    timezone: i64,
    id: i64,
    name: String,
    cod: i64,
}


async fn get_current_weather(location: &Location, api_key: String) -> Result<CurrentWeather> {
    let url = Url::parse_with_params(
        OWM_CURRENT_WEATHER_API,
        &[
            ("lat", format!("{}", location.lat).as_str()),
            ("lon", format!("{}", location.lon).as_str()),
            ("appid", api_key.as_str()),
        ]
    )?;
    let resp = reqwest::get(url).await?;
    match resp.status() {
        StatusCode::OK => {
            let text = resp.text().await?;
            Ok(serde_json::from_str::<CurrentWeather>(text.as_str())?)
        },
        _ => Err(Error::msg("failed to resolve location")),
    }
}


pub static WEATHER_ABILITY: Ability = Ability {
    name: "weather",
    aliases: &[],
    description: "Get the current weather at a location",
    command: || {
        Some(
            Command::new("weather").arg(
                arg!(--zip)
                    .num_args(0)
                    .required(false)
            ).arg(
                arg!(<location>)
                    .num_args(1..)
                    .trailing_var_arg(true),
            )
        )
    },
    execute: |args: &ArgMatches, ev: &OriginalSyncRoomMessageEvent, room: &Room| {
        async move {
            let zip_arg = args.get_flag("zip");
            let location_arg = args
                .get_many::<String>("location")
                .unwrap()
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            let owm_api_key = config::get(OWM_API_KEY.deref()).expect("OWM_API_KEY is not configured");
            let resolved_location = resolve_location(location_arg, zip_arg, owm_api_key.clone()).await?;
            if resolved_location.len() < 1 {
                return Err(Error::msg("cannot find location"));
            }
            let current_weather = get_current_weather(&resolved_location[0], owm_api_key.clone()).await?;
            let current_temperature =
                ThermodynamicTemperature::new::<thermodynamic_temperature::kelvin>(current_weather.main.temp);
            let message = RoomMessageEventContent::text_markdown(
                format!("# Weather: {}, {}\n{:.2} °C ({:.2} °F)",
                        resolved_location[0].name,
                        resolved_location[0].country,
                        current_temperature.get::<thermodynamic_temperature::degree_celsius>(),
                        current_temperature.get::<thermodynamic_temperature::degree_fahrenheit>())
            ).make_reply_to(ev, ForwardThread::Yes, AddMentions::No);
            room.send(message).await?;
            Ok(())
        }
        .boxed()
    }
};
