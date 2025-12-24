use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, net::SocketAddr};

use anyhow::{Result, bail};
use config::{Config as C, ConfigError, File, FileFormat};
use serde::{Deserialize, Serialize};

use crate::producer::{ProducerManager, TopicContainer};

const DEFAULT_NB_WORKERS: u16 = 2;
const DEFAULT_PORT: u16 = 8080;

pub struct ProducerConfig<T> {
    pub id: usize,
    pub receivers: Vec<ReceiverConfig<T>>,
    pub nb_workers: u16,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct ProducerConfigOption<T> {
    pub id: usize,
    pub nb_workers: Option<u16>,
    pub port: Option<u16>,
    pub receivers: Vec<ReceiverConfig<T>>,
}

impl<T> From<ProducerConfigOption<T>> for ProducerConfig<T> {
    fn from(config_option: ProducerConfigOption<T>) -> Self {
        Self {
            id: config_option.id,
            receivers: config_option.receivers,
            nb_workers: config_option.nb_workers.unwrap_or(DEFAULT_NB_WORKERS),
            port: config_option.port.unwrap_or(DEFAULT_PORT),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReceiverConfig<T> {
    pub id: usize,
    pub addr: SocketAddr,
    pub topics: Vec<T>,
}

#[derive(Clone, Copy)]
pub struct Receiver {
    pub id: usize,
    pub address: SocketAddr,
}

pub fn load_config<'de, T>(
    filename: &str,
    format: FileFormat,
) -> Result<ProducerConfig<T>, ConfigError>
where
    T: serde::Deserialize<'de>,
{
    let settings = C::builder()
        .add_source(File::with_name(filename).format(format))
        .build()?;
    settings.try_deserialize().map(|config_option: ProducerConfigOption<T>| config_option.into())
}

pub fn get_file_format(filename: &str) -> Result<FileFormat> {
    let extension_str = filename.split(".").last();
    if extension_str.is_none() {
        bail!("The provided configuration file has no extension");
    }

    let extension = match extension_str.unwrap() {
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        "yaml" | "yml" => FileFormat::Yaml,
        format => bail!("Configuration format {} not supported", format),
    };

    Ok(extension)
}

pub struct ReceiversConfig<T> {
    pub topics: HashMap<T, Vec<usize>>,
    pub receivers: Vec<Receiver>,
    _phantom: PhantomData<T>,
}

pub trait FromConfig<'de, T>
where
    T: TopicContainer + Debug + Copy + Eq + Hash + Serialize + Deserialize<'de> + Send + Sync + 'static,
{
    fn from_config(filename: &str) -> Result<ProducerManager<T>>
    {
        let format = get_file_format(filename)?;
        let config = load_config(filename, format)?;
        let manager = ProducerManager::new(config.id);
        for receiver in config.receivers {
            manager.add_receiver(receiver.id, receiver.addr, receiver.topics);
        }
        Ok(manager)
    }
}

impl<T: Debug + Serialize + Copy + Hash + TopicContainer + Eq + Send + Sync + 'static> From<ProducerConfig<T>> for ProducerManager<T> {
    fn from(config: ProducerConfig<T>) -> Self {
        let res = Self::new(config.id);
        for receiver in config.receivers {
            res.add_receiver(receiver.id, receiver.addr, receiver.topics);
        }
        res
    }
}