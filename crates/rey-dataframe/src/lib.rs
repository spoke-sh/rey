#![forbid(unsafe_code)]

use std::{collections::BTreeMap, io::Cursor, sync::Arc};

use polars::prelude::{
    DataFrame, IpcStreamReader, IpcStreamWriter, PlSmallStr, SerReader, SerWriter,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ARROW_STREAM_MEDIA_TYPE: &str = "application/vnd.apache.arrow.stream";
pub const METADATA_RELATION: &str = "rey.relation";
pub const METADATA_SCHEMA_VERSION: &str = "rey.schema-version";
pub const METADATA_SEMANTIC_DIGEST: &str = "rey.semantic-digest";
pub const METADATA_ROW_COUNT: &str = "rey.row-count";
pub const METADATA_COMPLETE: &str = "rey.complete";
pub const METADATA_KEY_COLUMNS: &str = "rey.key-columns";
pub const METADATA_ATTRIBUTES: &str = "rey.attributes";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrameMetadata {
    pub relation: String,
    pub schema_version: String,
    pub semantic_digest: String,
    pub row_count: u64,
    pub complete: bool,
    pub key_columns: Vec<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Frame {
    dataframe: DataFrame,
    metadata: FrameMetadata,
}

impl Frame {
    pub fn new(dataframe: DataFrame, metadata: FrameMetadata) -> Result<Self, FrameError> {
        let actual = dataframe.height() as u64;
        if actual != metadata.row_count {
            return Err(FrameError::RowCount {
                declared: metadata.row_count,
                actual,
            });
        }
        Ok(Self {
            dataframe,
            metadata,
        })
    }

    pub fn from_arrow_stream(bytes: &[u8]) -> Result<Self, FrameError> {
        let mut reader = IpcStreamReader::new(Cursor::new(bytes));
        let custom = reader.custom_metadata()?.unwrap_or_default();
        let metadata = FrameMetadata {
            relation: required(&custom, METADATA_RELATION)?.to_owned(),
            schema_version: required(&custom, METADATA_SCHEMA_VERSION)?.to_owned(),
            semantic_digest: required(&custom, METADATA_SEMANTIC_DIGEST)?.to_owned(),
            row_count: required(&custom, METADATA_ROW_COUNT)?
                .parse()
                .map_err(|_| FrameError::InvalidMetadata(METADATA_ROW_COUNT))?,
            complete: required(&custom, METADATA_COMPLETE)?
                .parse()
                .map_err(|_| FrameError::InvalidMetadata(METADATA_COMPLETE))?,
            key_columns: serde_json::from_str(required(&custom, METADATA_KEY_COLUMNS)?)
                .map_err(|_| FrameError::InvalidMetadata(METADATA_KEY_COLUMNS))?,
            attributes: custom
                .get(METADATA_ATTRIBUTES)
                .map(|value| serde_json::from_str(value))
                .transpose()
                .map_err(|_| FrameError::InvalidMetadata(METADATA_ATTRIBUTES))?
                .unwrap_or_default(),
        };
        Self::new(reader.finish()?, metadata)
    }

    pub fn to_arrow_stream(&self) -> Result<Vec<u8>, FrameError> {
        let mut encoded = Vec::new();
        let mut dataframe = self.dataframe.clone();
        let key_columns = serde_json::to_string(&self.metadata.key_columns)?;
        let attributes = serde_json::to_string(&self.metadata.attributes)?;
        let custom = BTreeMap::from([
            (METADATA_RELATION, self.metadata.relation.as_str()),
            (
                METADATA_SCHEMA_VERSION,
                self.metadata.schema_version.as_str(),
            ),
            (
                METADATA_SEMANTIC_DIGEST,
                self.metadata.semantic_digest.as_str(),
            ),
            (METADATA_ROW_COUNT, &self.metadata.row_count.to_string()),
            (METADATA_COMPLETE, &self.metadata.complete.to_string()),
            (METADATA_KEY_COLUMNS, key_columns.as_str()),
            (METADATA_ATTRIBUTES, attributes.as_str()),
        ])
        .into_iter()
        .map(|(key, value)| {
            (
                PlSmallStr::from_static(key),
                PlSmallStr::from_string(value.to_owned()),
            )
        })
        .collect();
        let mut writer = IpcStreamWriter::new(&mut encoded);
        writer.set_custom_schema_metadata(Arc::new(custom));
        writer.finish(&mut dataframe)?;
        Ok(encoded)
    }

    #[must_use]
    pub const fn dataframe(&self) -> &DataFrame {
        &self.dataframe
    }

    #[must_use]
    pub const fn metadata(&self) -> &FrameMetadata {
        &self.metadata
    }
}

fn required<'a>(
    metadata: &'a BTreeMap<PlSmallStr, PlSmallStr>,
    key: &'static str,
) -> Result<&'a str, FrameError> {
    metadata
        .get(key)
        .map(AsRef::as_ref)
        .ok_or(FrameError::MissingMetadata(key))
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("Polars dataframe operation failed: {0}")]
    Polars(#[from] polars::error::PolarsError),
    #[error("frame is missing Arrow metadata key {0}")]
    MissingMetadata(&'static str),
    #[error("frame has invalid Arrow metadata key {0}")]
    InvalidMetadata(&'static str),
    #[error("frame declares {declared} rows but contains {actual}")]
    RowCount { declared: u64, actual: u64 },
    #[error("frame metadata JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use polars::df;

    use super::{Frame, FrameMetadata};

    #[test]
    fn arrow_round_trip_preserves_frame_metadata() {
        let frame = Frame::new(
            df!("id" => ["a", "b"], "available" => [true, false]).unwrap(),
            FrameMetadata {
                relation: "rey.test".to_owned(),
                schema_version: "1".to_owned(),
                semantic_digest: "blake3:test".to_owned(),
                row_count: 2,
                complete: true,
                key_columns: vec!["id".to_owned()],
                attributes: BTreeMap::from([("source".to_owned(), "fixture".to_owned())]),
            },
        )
        .unwrap();

        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert!(decoded.dataframe().equals_missing(frame.dataframe()));
    }
}
