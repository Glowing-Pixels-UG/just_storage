use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Object kind/category for domain-specific metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Model,
    KbDoc,
    Upload,
    Log,
    Custom(String),
}

/// Model-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_name: String,
    pub version: String,
    pub family: String, // llama, qwen, mistral, etc.
    pub format: ModelFormat,
    pub quantization: Option<String>,   // Q4_0, Q5_K_M, bf16, etc.
    pub framework: Option<String>,      // pytorch, onnxruntime, tvm, mlc
    pub device_profile: Option<String>, // cpu_only, gpu_16gb+, edge_npu
    pub runtime_compat: Vec<String>,    // llamacpp, exllamav2, etc.
}

/// Model file format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    Gguf,
    Safetensors,
    Onnx,
    Hf,
    Coreml,
    Tflite,
    Custom(String),
}

/// Knowledge base document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbDocMetadata {
    pub title: String,
    pub source: String, // confluence, pdf, email, zendesk
    pub source_url: Option<String>,
    pub source_id: Option<String>,
    pub language: Option<String>,
    pub doc_type: Option<String>, // policy, faq, ticket, contract
    pub schema: Option<String>,   // if structured data

    // Embedding reference (NOT the vector itself)
    pub embedding_index_id: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_vector_id: Option<String>,
}

/// Extended metadata container with domain-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// Object kind/category
    pub kind: ObjectKind,

    /// Content MIME type
    pub content_type: Option<String>,

    /// Short text summary (512-1024 chars)
    pub summary_short: Option<String>,

    /// Optional description
    pub description: Option<String>,

    /// Last access timestamp (for tiering/GC decisions)
    pub last_access_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Origin/provenance info
    pub origin: Option<OriginInfo>,

    /// Model-specific metadata
    pub model: Option<ModelMetadata>,

    /// KB document-specific metadata
    pub kb_doc: Option<KbDocMetadata>,

    /// Custom extensible tags (JSONB-compatible)
    pub tags: HashMap<String, serde_json::Value>,
}

/// Origin/provenance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginInfo {
    pub source_system: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
    pub upload_ip: Option<String>,
    pub upload_user: Option<String>,
}

impl Default for ObjectMetadata {
    fn default() -> Self {
        Self {
            kind: ObjectKind::Upload,
            content_type: None,
            summary_short: None,
            description: None,
            last_access_at: None,
            origin: None,
            model: None,
            kb_doc: None,
            tags: HashMap::new(),
        }
    }
}

impl ObjectMetadata {
    /// Create new metadata for a model
    pub fn new_model(
        model_name: String,
        version: String,
        family: String,
        format: ModelFormat,
    ) -> Self {
        Self {
            kind: ObjectKind::Model,
            model: Some(ModelMetadata {
                model_name,
                version,
                family,
                format,
                quantization: None,
                framework: None,
                device_profile: None,
                runtime_compat: Vec::new(),
            }),
            ..Default::default()
        }
    }

    /// Create new metadata for a KB document
    pub fn new_kb_doc(title: String, source: String) -> Self {
        Self {
            kind: ObjectKind::KbDoc,
            kb_doc: Some(KbDocMetadata {
                title,
                source,
                source_url: None,
                source_id: None,
                language: None,
                doc_type: None,
                schema: None,
                embedding_index_id: None,
                embedding_model: None,
                embedding_vector_id: None,
            }),
            ..Default::default()
        }
    }

    /// Serialize to JSON for storage in Postgres JSONB column
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Deserialize from JSON stored in Postgres JSONB column
    pub fn from_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_metadata_serialization() {
        let meta = ObjectMetadata::new_model(
            "llama-3.1-8b".to_string(),
            "2024-07-01".to_string(),
            "llama".to_string(),
            ModelFormat::Gguf,
        );

        let json = meta.to_json().unwrap();
        let deserialized = ObjectMetadata::from_json(&json).unwrap();

        assert_eq!(deserialized.kind, ObjectKind::Model);
        assert!(deserialized.model.is_some());
    }

    #[test]
    fn test_kb_doc_metadata() {
        let mut meta =
            ObjectMetadata::new_kb_doc("Refund Policy 2025".to_string(), "confluence".to_string());

        if let Some(ref mut kb) = meta.kb_doc {
            kb.language = Some("en".to_string());
            kb.doc_type = Some("policy".to_string());
            kb.embedding_index_id = Some("kb-prod".to_string());
        }

        let json = meta.to_json().unwrap();
        assert!(json["kb_doc"]["embedding_index_id"].is_string());
    }
}
