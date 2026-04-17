use std::fmt;

use reqwest::Method;

use crate::auth::CredentialKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolSurface {
    OfficialApi,
    SessionWeb,
}

impl fmt::Display for ProtocolSurface {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OfficialApi => formatter.write_str("OfficialApi"),
            Self::SessionWeb => formatter.write_str("SessionWeb"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiVersion {
    V0,
    V1,
    NotApplicable,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V0 => formatter.write_str("v0"),
            Self::V1 => formatter.write_str("v1"),
            Self::NotApplicable => formatter.write_str("n/a"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserShape {
    JsonEnvelope,
    Html,
    Stream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
}

impl HttpMethod {
    pub(crate) fn as_reqwest(self) -> Method {
        match self {
            Self::Get => Method::GET,
            Self::Post => Method::POST,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointId {
    OfficialSearch,
    OfficialEnrichWeb,
    OfficialEnrichNews,
    OfficialSummarizeGet,
    OfficialSummarizePost,
    OfficialFastGpt,
    OfficialSmallwebFeed,
    SessionHtmlSearch,
    SessionSummaryLabsGet,
    SessionSummaryLabsPost,
}

impl fmt::Display for EndpointId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.spec().name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndpointSpec {
    pub name: &'static str,
    pub surface: ProtocolSurface,
    pub method: HttpMethod,
    pub route: &'static str,
    pub version: ApiVersion,
    pub parser: ParserShape,
    pub allowed_credential: CredentialKind,
}

impl EndpointId {
    pub fn spec(self) -> EndpointSpec {
        match self {
            Self::OfficialSearch => EndpointSpec {
                name: "official.search",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Get,
                route: "/api/v0/search",
                version: ApiVersion::V0,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::OfficialEnrichWeb => EndpointSpec {
                name: "official.enrich_web",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Get,
                route: "/api/v0/enrich/web",
                version: ApiVersion::V0,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::OfficialEnrichNews => EndpointSpec {
                name: "official.enrich_news",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Get,
                route: "/api/v0/enrich/news",
                version: ApiVersion::V0,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::OfficialSummarizeGet => EndpointSpec {
                name: "official.summarize_get",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Get,
                route: "/api/v0/summarize",
                version: ApiVersion::V0,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::OfficialSummarizePost => EndpointSpec {
                name: "official.summarize_post",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Post,
                route: "/api/v0/summarize",
                version: ApiVersion::V0,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::OfficialFastGpt => EndpointSpec {
                name: "official.fastgpt",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Post,
                route: "/api/v0/fastgpt",
                version: ApiVersion::V0,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::OfficialSmallwebFeed => EndpointSpec {
                name: "official.smallweb_feed",
                surface: ProtocolSurface::OfficialApi,
                method: HttpMethod::Get,
                route: "/api/v1/smallweb/feed",
                version: ApiVersion::V1,
                parser: ParserShape::JsonEnvelope,
                allowed_credential: CredentialKind::BotToken,
            },
            Self::SessionHtmlSearch => EndpointSpec {
                name: "session.html_search",
                surface: ProtocolSurface::SessionWeb,
                method: HttpMethod::Get,
                route: "/html/search",
                version: ApiVersion::NotApplicable,
                parser: ParserShape::Html,
                allowed_credential: CredentialKind::SessionToken,
            },
            Self::SessionSummaryLabsGet => EndpointSpec {
                name: "session.summary_labs_get",
                surface: ProtocolSurface::SessionWeb,
                method: HttpMethod::Get,
                route: "/mother/summary_labs",
                version: ApiVersion::NotApplicable,
                parser: ParserShape::Stream,
                allowed_credential: CredentialKind::SessionToken,
            },
            Self::SessionSummaryLabsPost => EndpointSpec {
                name: "session.summary_labs_post",
                surface: ProtocolSurface::SessionWeb,
                method: HttpMethod::Post,
                route: "/mother/summary_labs/",
                version: ApiVersion::NotApplicable,
                parser: ParserShape::Stream,
                allowed_credential: CredentialKind::SessionToken,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiVersion, EndpointId};

    #[test]
    fn official_endpoints_cover_both_v0_and_v1_routes() {
        let v0 = EndpointId::OfficialSearch.spec();
        let v1 = EndpointId::OfficialSmallwebFeed.spec();

        assert_eq!(v0.version, ApiVersion::V0);
        assert_eq!(v0.route, "/api/v0/search");
        assert_eq!(v1.version, ApiVersion::V1);
        assert_eq!(v1.route, "/api/v1/smallweb/feed");
    }
}
