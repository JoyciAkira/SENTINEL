//! World-class embeddings for semantic memory
//!
//! Provides SOTA (State-of-the-Art) text embeddings using:
//! - Candle: Pure Rust ML framework (primary)
//! - Fallback to pseudo-embeddings if model unavailable
//!
//! All models are:
//! - 100% local (no API calls)
//! - 100% free (open-source)
//! - Privacy-preserving (no data leaves machine)

#![allow(unsafe_code)]

use anyhow::{Error as E, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::sync::{Arc, Mutex, OnceLock};
use tokenizers::{PaddingParams, Tokenizer};

static EMBEDDER_INSTANCE: OnceLock<Option<Arc<CandleEmbedder>>> = OnceLock::new();

/// Embedding backend for semantic similarity
#[derive(Debug)]
pub enum EmbeddingBackend {
    /// Candle with SOTA models (primary)
    Candle(Arc<CandleEmbedder>),

    /// Pseudo-embeddings (fallback if model fails to load)
    Pseudo,
}

pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

impl std::fmt::Debug for CandleEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleEmbedder")
            .field("device", &self.device)
            .finish()
    }
}

/// World-class embedder
#[derive(Debug)]
pub struct Embedder {
    backend: EmbeddingBackend,
    dimension: usize,
}

impl Embedder {
    /// Create a new embedder with Candle
    pub fn new() -> Self {
        match Self::try_candle() {
            Ok(backend) => {
                eprintln!("✅ Loaded SOTA embedding model (Candle - 384 dimensions)");
                Self {
                    backend,
                    dimension: 384,
                }
            }
            Err(e) => {
                eprintln!("⚠️  Candle unavailable ({}), using pseudo-embeddings", e);
                Self {
                    backend: EmbeddingBackend::Pseudo,
                    dimension: 768,
                }
            }
        }
    }

    fn try_candle() -> Result<EmbeddingBackend> {
        let instance = EMBEDDER_INSTANCE.get_or_init(|| match CandleEmbedder::new() {
            Ok(embedder) => Some(Arc::new(embedder)),
            Err(e) => {
                eprintln!("Failed to initialize Candle: {:?}", e);
                None
            }
        });

        if let Some(instance) = instance {
            Ok(EmbeddingBackend::Candle(instance.clone()))
        } else {
            Err(E::msg("Failed to initialize Candle model"))
        }
    }

    /// Compute embedding for text
    pub fn embed(&self, text: &str) -> Vec<f32> {
        match &self.backend {
            EmbeddingBackend::Candle(embedder) => embedder.embed_single(text).unwrap_or_else(|e| {
                eprintln!("Embedding error: {}", e);
                Self::pseudo_embed(text, self.dimension)
            }),
            EmbeddingBackend::Pseudo => Self::pseudo_embed(text, self.dimension),
        }
    }

    /// Compute embeddings for multiple texts (batched for efficiency)
    pub fn embed_batch(&self, texts: &[String]) -> Vec<Vec<f32>> {
        match &self.backend {
            EmbeddingBackend::Candle(embedder) => embedder.embed_batch(texts).unwrap_or_else(|e| {
                eprintln!("Batch embedding error: {}", e);
                texts
                    .iter()
                    .map(|t| Self::pseudo_embed(t, self.dimension))
                    .collect()
            }),
            EmbeddingBackend::Pseudo => texts
                .iter()
                .map(|t| Self::pseudo_embed(t, self.dimension))
                .collect(),
        }
    }

    /// Pseudo-embeddings (fallback)
    fn pseudo_embed(text: &str, dimension: usize) -> Vec<f32> {
        let mut embedding = vec![0.0; dimension];

        // Character frequency-based features
        for (i, ch) in text.chars().enumerate() {
            let idx = (ch as usize + i) % dimension;
            embedding[idx] += 1.0;
        }

        // Normalize
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        embedding
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Check if using SOTA model
    pub fn is_sota(&self) -> bool {
        matches!(self.backend, EmbeddingBackend::Candle(_))
    }
}

impl Default for Embedder {
    fn default() -> Self {
        Self::new()
    }
}

impl CandleEmbedder {
    fn new() -> Result<Self> {
        let device = Device::Cpu;

        // Use all-MiniLM-L6-v2
        let api = Api::new()?;
        let repo = api.repo(Repo::new(
            "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            RepoType::Model,
        ));

        let config_filename = repo.get("config.json")?;
        let tokenizer_filename = repo.get("tokenizer.json")?;
        let weights_filename = repo.get("model.safetensors")?;

        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_filename)?)?;
        let mut tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };
        let model = BertModel::load(vb, &config)?;

        // Setup tokenizer for batching
        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }

        Ok(Self {
            model,
            tokenizer: Mutex::new(tokenizer),
            device,
        })
    }

    fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let res = self.embed_batch(&[text.to_string()])?;
        Ok(res[0].clone())
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut tokenizer = self.tokenizer.lock().unwrap();
        let tokens = tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(E::msg)?;

        let token_ids = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Ok(Tensor::new(tokens.as_slice(), &self.device)?)
            })
            .collect::<Result<Vec<_>>>()?;

        let attention_mask = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_attention_mask().to_vec();
                Ok(Tensor::new(tokens.as_slice(), &self.device)?)
            })
            .collect::<Result<Vec<_>>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let attention_mask = Tensor::stack(&attention_mask, 0)?;
        let token_type_ids = token_ids.zeros_like()?; // BERT uses 0 for sentence A

        // Run model
        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))?;

        // Mean pooling
        // Dimensions: [batch, seq_len, hidden_size]
        let (_batch_len, _seq_len, _hidden_size) = embeddings.dims3()?;

        // Calculate mean over sequence length (dim 1)
        // Note: For simplicity with short texts, we use simple mean.
        let mean_pool = embeddings.mean(1)?;

        // Normalize embeddings (L2 norm)
        let sum_sq = mean_pool.sqr()?.sum_keepdim(1)?;
        let norm = sum_sq.sqrt()?;
        let normalized = mean_pool.broadcast_div(&norm)?;

        let rows = normalized.to_vec2::<f32>()?;
        Ok(rows)
    }
}

/// Compute cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedder_creation() {
        let embedder = Embedder::new();
        assert!(embedder.dimension() > 0);
    }

    #[test]
    fn test_embed_single() {
        let embedder = Embedder::new();
        let embedding = embedder.embed("Hello world");

        assert_eq!(embedding.len(), embedder.dimension());

        // Check normalization
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_embed_batch() {
        let embedder = Embedder::new();
        let texts = vec!["Hello world".to_string(), "Goodbye world".to_string()];

        let embeddings = embedder.embed_batch(&texts);
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), embedder.dimension());
    }

    #[test]
    fn test_cosine_similarity() {
        let embedder = Embedder::new();

        let emb1 = embedder.embed("machine learning");
        let emb2 = embedder.embed("machine learning");
        let emb3 = embedder.embed("cooking recipes");

        // Identical texts should have high similarity
        let sim_same = cosine_similarity(&emb1, &emb2);
        assert!(sim_same > 0.95);

        // Different texts should have lower similarity
        let sim_diff = cosine_similarity(&emb1, &emb3);
        assert!(sim_diff < sim_same);
    }

    #[test]
    fn test_pseudo_embed_fallback() {
        let embedding = Embedder::pseudo_embed("test", 384);
        assert_eq!(embedding.len(), 384);

        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }
}
