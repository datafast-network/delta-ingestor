use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::{Client, ClientConfig};
use crate::errors::ProducerError;
use google_cloud_pubsub::publisher::{Publisher};
use serde::{Deserialize, Serialize};
use common_libs::async_trait::async_trait;
use common_libs::envy;
use common_libs::log::info;
use crate::core::ProducerTrait;
use crate::proto::BlockTrait;

#[derive(Serialize, Deserialize)]
struct PubSubConfig {
    pubsub_topic: String,
    pubsub_ordering_key: Option<String>,
    pubsub_compression: bool,
}

#[derive(Clone)]
pub struct PubSubProducer {
    ordering_key: Option<String>,
    publisher: Publisher,
    compression: bool,
}

impl PubSubProducer {
    pub async fn new() -> Result<Self, ProducerError> {
        let env_conf = envy::from_env::<PubSubConfig>().unwrap();
        let config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| ProducerError::Initialization(e.to_string()))?;
        let client = Client::new(config).await.map_err(|e| ProducerError::Initialization(e.to_string()))?;
        let topic = client.topic(&env_conf.pubsub_topic);
        match topic.exists(None).await {
            Ok(status) => {
                if !status {
                    topic.create(None, None).await
                        .map_err(|_| ProducerError::Initialization(format!("create topic {} error", env_conf.pubsub_topic)))?;
                }
                let publisher = topic.new_publisher(None);
                Ok(Self {
                    ordering_key: env_conf.pubsub_ordering_key,
                    publisher,
                    compression: env_conf.pubsub_compression,
                })
            }
            _ => {
                return Err(ProducerError::Initialization(format!("check topic {} error", env_conf.pubsub_topic)));
            }
        }
    }

    async fn publish<B: BlockTrait>(&self, block: B) -> Result<(), ProducerError> {
        let mut message = PubsubMessage {
            data: block.encode_to_vec(),
            ..Default::default()
        };

        if self.compression {
            let block_compressed = lz4::block::compress(block.encode_to_vec().as_slice(), None, true)
                .map_err(|e| ProducerError::Publish(e.to_string()))?;
            message.data = block_compressed
        }

        if self.ordering_key.is_some() {
            message.ordering_key = self.ordering_key.clone().unwrap()
        }
        let awaiter = self.publisher.publish(message).await;
        awaiter.get().await.map_err(|_| ProducerError::Publish("publish error".to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl<B: BlockTrait> ProducerTrait<B> for PubSubProducer {
    async fn publish_blocks(&self, blocks: Vec<B>) -> Result<(), ProducerError> {
        for block in blocks {
            self.publish(block).await?
        }
        Ok(())
    }
}