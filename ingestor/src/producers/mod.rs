#[cfg(feature = "deltalake")]
mod delta;
mod stdout;

#[cfg(feature = "pubsub")]
mod pubsub;

#[cfg(feature = "deltalake")]
use delta::DeltaLakeProducer;
#[cfg(feature = "pubsub")]
use pubsub::PubSubProducer;
#[cfg(feature = "deltalake")]
use stdout::StdOutProducer;

use crate::config::CommandConfig;
use crate::core::ProducerTrait;
use crate::errors::ProducerError;
use crate::name_svc::NameService;
use crate::proto::BlockTrait;
use strum::Display;

use common_libs::async_trait::async_trait;
use common_libs::log::info;

#[derive(Display)]
pub enum Producer<B: BlockTrait> {
    #[strum(serialize = "Stdout-Producer")]
    StdOut(StdOutProducer<B>),
    #[cfg(feature = "deltalake")]
    #[strum(serialize = "Delta-Producer")]
    DeltaLake(DeltaLakeProducer),
    #[cfg(feature = "pubsub")]
    #[strum(serialize = "Pubsub-Producer")]
    PubSub(PubSubProducer),
}

#[async_trait]
impl<B: BlockTrait> ProducerTrait<B> for Producer<B> {
    async fn publish_blocks(&self, blocks: Vec<B>) -> Result<(), ProducerError> {
        match self {
            Producer::StdOut(producer) => producer.publish_blocks(blocks).await,
            #[cfg(feature = "deltalake")]
            Producer::DeltaLake(producer) => producer.publish_blocks(blocks).await,
            #[cfg(feature = "pubsub")]
            Producer::PubSub(producer) => producer.publish_blocks(blocks).await,
        }
    }
}

pub async fn create_producer<B: BlockTrait>(
    cfg: &CommandConfig,
    _name_service: &NameService,
) -> Result<Producer<B>, ProducerError> {
    let producer_type = cfg.producer.clone().unwrap_or("unspecified".to_string());
    match producer_type.as_str() {
        #[cfg(feature = "deltalake")]
        "delta" => {
            info!("Setting up DeltaLake Producer");
            let producer = DeltaLakeProducer::new(cfg).await?;
            Ok(Producer::DeltaLake(producer))
        }
        #[cfg(feature = "pubsub")]
        "pubsub" => {
            info!("Setting up PubSub Producer");
            let producer = PubSubProducer::new().await?;
            Ok(Producer::PubSub(producer))
        }
        _ => {
            info!("Unknown Producer type, using stdout as Producer");
            let producer = StdOutProducer::new();
            Ok(Producer::StdOut(producer))
        }
    }
}
