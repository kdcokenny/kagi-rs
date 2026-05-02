use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

const SEARCH_DEFAULT_LIMIT: u8 = 5;
const SEARCH_MIN_LIMIT: u8 = 1;
const SEARCH_MAX_LIMIT: u8 = 10;
const SUMMARIZE_MAX_TEXT_BYTES: usize = 50_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchToolInput {
    #[serde(deserialize_with = "deserialize_trimmed_query")]
    pub query: String,

    #[serde(
        default = "default_search_limit",
        deserialize_with = "deserialize_search_limit"
    )]
    pub limit: u8,
}

impl SearchToolInput {
    pub fn limit_as_usize(&self) -> usize {
        self.limit as usize
    }
}

impl JsonSchema for SearchToolInput {
    fn schema_name() -> Cow<'static, str> {
        "SearchToolInput".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::SearchToolInput").into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "additionalProperties": false,
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "minLength": 1,
                    "pattern": ".*\\S.*"
                },
                "limit": {
                    "type": "integer",
                    "minimum": SEARCH_MIN_LIMIT,
                    "maximum": SEARCH_MAX_LIMIT,
                    "default": SEARCH_DEFAULT_LIMIT
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct SummarizeToolInput {
    pub url: Option<String>,
    pub text: Option<String>,
}

impl JsonSchema for SummarizeToolInput {
    fn schema_name() -> Cow<'static, str> {
        "SummarizeToolInput".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::SummarizeToolInput").into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "url": {
                    "type": "string"
                },
                "text": {
                    "type": "string"
                }
            }
        })
    }
}

impl<'de> Deserialize<'de> for SummarizeToolInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSummarizeToolInput {
            url: Option<String>,
            text: Option<String>,
        }

        let raw = RawSummarizeToolInput::deserialize(deserializer)?;

        let normalized_url = raw
            .url
            .and_then(|url| if url.is_empty() { None } else { Some(url) });
        let normalized_text = raw
            .text
            .and_then(|text| if text.is_empty() { None } else { Some(text) });

        let has_url = normalized_url.is_some();
        let has_text = normalized_text.is_some();
        if has_url == has_text {
            return Err(serde::de::Error::custom(
                "exactly one of `url` or `text` must be provided",
            ));
        }

        if let Some(raw_url) = normalized_url {
            if raw_url != raw_url.trim() {
                return Err(serde::de::Error::custom(
                    "`url` cannot have leading or trailing whitespace",
                ));
            }

            let parsed = Url::parse(&raw_url).map_err(|source| {
                serde::de::Error::custom(format!(
                    "`url` must be an absolute HTTP(S) URL ({source})"
                ))
            })?;

            if !matches!(parsed.scheme(), "http" | "https") {
                return Err(serde::de::Error::custom("`url` must use `http` or `https`"));
            }

            return Ok(Self {
                url: Some(parsed.to_string()),
                text: None,
            });
        }

        let text = normalized_text.expect("xor check ensures text exists");
        if text.trim().is_empty() {
            return Err(serde::de::Error::custom("`text` cannot be blank"));
        }

        let byte_len = text.len();
        if byte_len > SUMMARIZE_MAX_TEXT_BYTES {
            return Err(serde::de::Error::custom(format!(
                "`text` exceeds {SUMMARIZE_MAX_TEXT_BYTES} UTF-8 bytes"
            )));
        }

        Ok(Self {
            url: None,
            text: Some(text),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchResultCard {
    pub title: String,
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchToolOutput {
    pub results: Vec<SearchResultCard>,
    pub total_returned: usize,
}

impl JsonSchema for SearchToolOutput {
    fn schema_name() -> Cow<'static, str> {
        "SearchToolOutput".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::SearchToolOutput").into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let results_schema = generator
            .subschema_for::<Vec<SearchResultCard>>()
            .to_value();

        json_schema!({
            "type": "object",
            "additionalProperties": false,
            "required": ["results", "total_returned"],
            "properties": {
                "results": results_schema,
                "total_returned": {
                    "type": "integer",
                    "minimum": 0
                }
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SummarizeToolOutput {
    pub markdown: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

fn default_search_limit() -> u8 {
    SEARCH_DEFAULT_LIMIT
}

fn deserialize_trimmed_query<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_query = String::deserialize(deserializer)?;
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return Err(serde::de::Error::custom("`query` cannot be blank"));
    }

    Ok(trimmed.to_string())
}

fn deserialize_search_limit<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let limit = u8::deserialize(deserializer)?;
    if !(SEARCH_MIN_LIMIT..=SEARCH_MAX_LIMIT).contains(&limit) {
        return Err(serde::de::Error::custom(format!(
            "`limit` must be between {SEARCH_MIN_LIMIT} and {SEARCH_MAX_LIMIT}"
        )));
    }

    Ok(limit)
}
