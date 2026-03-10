use crate::db::Db;
use anyhow::{Error, Result};
use lazy_static::lazy_static;
use rusqlite::Error::QueryReturnedNoRows;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::RwLock;

macro_rules! config_keys {
    ( $( $name:ident, $ty:ty, $desc:expr $( , default = $default:expr )? );* $(;)? ) => {
        lazy_static! {
            $(
                pub static ref $name: ConfigKey<$ty> = {
                    let name = stringify!($name).to_lowercase();
                    let k = ConfigKey::new(Box::leak(name.into_boxed_str()), $desc);
                    $( let k = k.default($default); )?
                    k
                };
            )*
            pub static ref CONFIG_REGISTRY: RwLock<HashMap<&'static str, &'static dyn GenericConfigKey>> =
                RwLock::new({
                    let mut m = HashMap::new();
                    $( m.insert($name.name(), &*$name as &dyn GenericConfigKey); )*
                    m
                });
        }
    };
}

// for valid types, see [verify_json_type]
config_keys! {
    TEST, String, "test key";
    BOT_PREFIX, String, "The prefix for bot commands", default = "!".to_string();
    SOME_NUMBER, i64, "some number", default = 67;
    LIST_OF_NUMBERS, Vec<i64>, "list of numbers", default = vec![1, 2, 3];
    STRING_TO_STRING_MAP, HashMap<String, String>, "a map from string to string";
}

#[derive(Debug)]
pub struct ConfigKey<T> {
    name: &'static str,
    description: &'static str,
    default: Option<T>,
    on_change: Option<fn(old_value: &T, new_value: &T)>,
}

impl<T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static> ConfigKey<T> {
    fn new(key: &'static str, description: &'static str) -> Self {
        Self {
            name: key,
            description,
            default: None,
            on_change: None,
        }
    }

    fn default(mut self, default: T) -> Self {
        self.default = Some(default);
        self
    }

    fn on_change(mut self, on_change: fn(old_value: &T, new_value: &T)) -> Self {
        self.on_change = Some(on_change);
        self
    }
}

pub trait GenericConfigKey: Send + Sync {
    fn name(&self) -> &'static str;
    fn type_id(&self) -> TypeId;
    fn description(&self) -> &'static str;
    fn default(&self) -> Option<Value>;
    fn call_on_change(&self, old_value: &Value, new_value: &Value);
}

impl<T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static> GenericConfigKey for ConfigKey<T> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn default(&self) -> Option<Value> {
        self.default.as_ref().map(|v| serde_json::to_value(v).unwrap())
    }

    fn call_on_change(&self, old_value: &Value, new_value: &Value) {
        if let Some(on_change) = self.on_change {
            let old_t: T = serde_json::from_value(old_value.clone()).unwrap();
            let new_t: T = serde_json::from_value(new_value.clone()).unwrap();
            on_change(&old_t, &new_t);
        }
    }
}

pub fn get<T: Send + Sync + DeserializeOwned>(key: &ConfigKey<T>) -> Result<T, Error> {
    let db = Db::connect()?;
    let value = match db.conn.query_one(
        "SELECT value FROM config WHERE key = :key",
        &[(":key", &key.name)],
        |r| r.get::<_, String>(0),
    ) {
        Ok(value) => value,
        Err(QueryReturnedNoRows) => return Err(Error::msg("config key does not exist")),
        Err(e) => return Err(Error::from(e)),
    };
    Ok(serde_json::from_str(&value)?)
}

pub fn get_json_by_name(key_name: &str) -> Result<Value, Error> {
    let key = *CONFIG_REGISTRY
        .read()
        .unwrap()
        .get(key_name)
        .ok_or(Error::msg("config key not found"))?;
    let db = Db::connect()?;
    let value = match db.conn.query_one(
        "SELECT value FROM config WHERE key = :key",
        &[(":key", key.name())],
        |r| r.get::<_, String>(0),
    ) {
        Ok(value) => value,
        Err(QueryReturnedNoRows) => return Err(Error::msg("config key does not exist")),
        Err(e) => return Err(Error::from(e)),
    };
    Ok(serde_json::from_str(&value)?)
}

pub fn set<T: Send + Sync + Serialize>(key: &ConfigKey<T>, value: T) -> Result<(), Error> {
    let serialized_value = serde_json::to_string(&value)?;
    let db = Db::connect()?;
    db.conn.execute(
        "INSERT INTO config (key, value) VALUES (:key, :value) ON CONFLICT (key) DO UPDATE SET value = :value",
        &[(":key", &key.name), (":value", &serialized_value.as_str())],
    )?;
    Ok(())
}

fn verify_json_type(type_id: TypeId, json_value: &Value) -> Result<(), Error> {
    if type_id == TypeId::of::<String>() {
        if !json_value.is_string() {
            return Err(Error::msg("value needs to be a string"));
        }
    } else if type_id == TypeId::of::<i64>() {
        if !json_value.is_i64() {
            return Err(Error::msg("value needs to be an integer"));
        }
    } else if type_id == TypeId::of::<f64>() {
        if !json_value.is_f64() {
            return Err(Error::msg("value needs to be a float"));
        }
    } else if type_id == TypeId::of::<bool>() {
        if !json_value.is_boolean() {
            return Err(Error::msg("value needs to be a boolean"));
        }
    } else if type_id == TypeId::of::<Vec<String>>() {
        if !json_value.is_array() || json_value.as_array().unwrap().iter().any(|v| !v.is_string()) {
            return Err(Error::msg("value needs to be an array of strings"));
        }
    } else if type_id == TypeId::of::<Vec<i64>>() {
        if !json_value.is_array() || json_value.as_array().unwrap().iter().any(|v| !v.is_i64()) {
            return Err(Error::msg("value needs to be an array of integers"));
        }
    } else if type_id == TypeId::of::<Vec<f64>>() {
        if !json_value.is_array() || json_value.as_array().unwrap().iter().any(|v| !v.is_f64()) {
            return Err(Error::msg("value needs to be an array of floats"));
        }
    } else if type_id == TypeId::of::<HashMap<String, String>>() {
        if !json_value.is_object() || json_value.as_object().unwrap().iter().any(|(_, v)| !v.is_string()) {
            return Err(Error::msg("value needs to be an object with string values"));
        }
    } else if type_id == TypeId::of::<HashMap<String, i64>>() {
        if !json_value.is_object() || json_value.as_object().unwrap().iter().any(|(_, v)| !v.is_i64()) {
            return Err(Error::msg("value needs to be an object with integer values"));
        }
    } else if type_id == TypeId::of::<HashMap<String, f64>>() {
        if !json_value.is_object() || json_value.as_object().unwrap().iter().any(|(_, v)| !v.is_f64()) {
            return Err(Error::msg("value needs to be an object with float values"));
        }
    } else {
        return Err(Error::msg("config key uses an unsupported type"));
    }
    Ok(())
}

pub fn set_json_by_name(key_name: &str, value: &Value) -> Result<(), Error> {
    let key = *CONFIG_REGISTRY
        .read()
        .unwrap()
        .get(key_name)
        .ok_or(Error::msg("config key not found"))?;
    verify_json_type(key.type_id(), &value)?;
    let db = Db::connect()?;
    db.conn.execute(
        "INSERT INTO config (key, value) VALUES (:key, :value) ON CONFLICT (key) DO UPDATE SET value = :value",
        &[(":key", key.name()), (":value", value.to_string().as_str())],
    )?;
    Ok(())
}
