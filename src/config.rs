use std::{collections::HashMap, hash::Hash, marker::PhantomData, net::IpAddr};

use anyhow::{Result, bail};
use config::{Config as C, ConfigError, File, FileFormat};
use serde::Deserialize;

use crate::producer::ReceiverDispatch;

#[derive(Debug, Deserialize)]
pub struct AppConfig<T> {
    pub receivers: Vec<ReceiverConfig<T>>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiverConfig<T> {
    pub id: u16,
    pub address: IpAddr,
    pub port: u16,
    pub topics: Vec<T>,
}

#[derive(Clone, Copy)]
pub struct Receiver {
    pub id: u16,
    pub address: IpAddr,
    pub port: u16,
}

pub fn load_config<'de, T>(filename: &str, format: FileFormat) -> Result<AppConfig<T>, ConfigError>
where
    T: serde::Deserialize<'de>,
{
    let settings = C::builder()
        .add_source(File::with_name(filename).format(format))
        .build()?;
    settings.try_deserialize()
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
    T: Sized,
    T: Copy + Eq + Hash + serde::Deserialize<'de>,
{
    fn from_config(filename: &str) -> Result<Self>
    where
        Self: ReceiverDispatch<T> + Sized + Default,
    {
        let mut res = Self::default();

        let format = get_file_format(filename)?;
        let config = load_config(filename, format)?;
        let receivers_config = res.sort_by_topics(config)?;

        for (topic, receivers_id) in receivers_config.topics {
            for receiver_id in receivers_id {
                let receiver = receivers_config.receivers[receiver_id];
                res.add_receiver(
                    topic,
                    receiver.id as usize,
                    &format!("{}:{}", receiver.address, receiver.port),
                );
            }
        }

        Ok(res)
    }

    fn sort_by_topics(&self, config: AppConfig<T>) -> Result<ReceiversConfig<T>> {
        let mut topics: HashMap<T, Vec<usize>> = HashMap::new();
        let mut receivers = Vec::new();

        for (index, receiver_config) in config.receivers.iter().enumerate() {
            let receiver = Receiver {
                id: receiver_config.id,
                address: receiver_config.address,
                port: receiver_config.port,
            };
            receivers.push(receiver);

            for topic in &receiver_config.topics {
                topics.entry(*topic).or_default().push(index);
            }
        }

        Ok(ReceiversConfig {
            receivers,
            topics,
            _phantom: PhantomData,
        })
    }
}
