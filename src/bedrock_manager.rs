// src/bedrock_manager.rs
// Manages interactions with the AWS Bedrock API

#![cfg(feature = "bedrock_integration")]

use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_bedrock::{Client, Region};
use aws_sdk_bedrock::types::FoundationModelSummary;
use tracing::{info, error, debug};

pub struct BedrockManager {
    client: Client,
    pub bedrock_connected: bool,
}

impl BedrockManager {
    pub async fn new(aws_region: Option<String>) -> Result<Self> {
        let region_provider = RegionProviderChain::first_try(aws_region.map(Region::new))
            .or_default_provider()
            .or_else(Region::new("us-east-1"));

        info!("Attempting to connect to AWS Bedrock in region: {:?}", region_provider.region().await);

        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&config);

        let mut bedrock_connected = false;
        match client.list_foundation_models().send().await {
            Ok(_) => {
                info!("Successfully connected to AWS Bedrock.");
                bedrock_connected = true;
            }
            Err(e) => {
                error!("Failed to connect to AWS Bedrock: {}", e);
            }
        }

        info!("BedrockManager initialized. Bedrock integration enabled.");
        Ok(Self { client, bedrock_connected })
    }

    pub fn is_bedrock_connected(&self) -> bool {
        self.bedrock_connected
    }

    pub async fn list_foundation_models(&self) -> Result<Vec<FoundationModelSummary>> {
        if self.bedrock_connected {
            debug!("Listing foundation models from AWS Bedrock.");
            match self.client.list_foundation_models().send().await {
                Ok(response) => {
                    let models = response.model_summaries().unwrap_or_default().to_vec();
                    debug!("Successfully listed {} Bedrock models.", models.len());
                    Ok(models)
                }
                Err(e) => {
                    error!("Failed to list Bedrock models: {}", e);
                    Err(anyhow::anyhow!("Failed to list Bedrock models: {}", e))
                }
            }
        } else {
            debug!("Bedrock client not available for listing models. Returning empty list.");
            Ok(Vec::new())
        }
    }
} 