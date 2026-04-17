use crate::{
    auth::{BotToken, CredentialKind, Credentials, SessionToken},
    config::ClientConfig,
    error::KagiError,
    official_api::OfficialApi,
    routing::ProtocolSurface,
    session_web::SessionWeb,
    transport::Transport,
};

#[derive(Debug, Clone)]
pub struct KagiClient {
    transport: Transport,
    credentials: Credentials,
}

impl KagiClient {
    pub fn builder() -> KagiClientBuilder {
        KagiClientBuilder::default()
    }

    pub fn new(credentials: Credentials, config: ClientConfig) -> Result<Self, KagiError> {
        let transport = Transport::new(config)?;
        Ok(Self {
            transport,
            credentials,
        })
    }

    pub fn with_bot_token(token: BotToken) -> Result<Self, KagiError> {
        Self::new(Credentials::from(token), ClientConfig::default())
    }

    pub fn with_session_token(token: SessionToken) -> Result<Self, KagiError> {
        Self::new(Credentials::from(token), ClientConfig::default())
    }

    pub fn official_api(&self) -> Result<OfficialApi<'_>, KagiError> {
        self.ensure_surface_access(ProtocolSurface::OfficialApi)?;
        Ok(OfficialApi::new(self))
    }

    pub fn session_web(&self) -> Result<SessionWeb<'_>, KagiError> {
        self.ensure_surface_access(ProtocolSurface::SessionWeb)?;
        Ok(SessionWeb::new(self))
    }

    pub(crate) fn transport(&self) -> &Transport {
        &self.transport
    }

    pub(crate) fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    fn ensure_surface_access(&self, surface: ProtocolSurface) -> Result<(), KagiError> {
        let provided = self.credentials.kind();
        let expected = match surface {
            ProtocolSurface::OfficialApi => CredentialKind::BotToken,
            ProtocolSurface::SessionWeb => CredentialKind::SessionToken,
        };

        if provided != expected {
            return Err(KagiError::UnsupportedAuthSurface {
                surface,
                credential: provided,
                expected,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct KagiClientBuilder {
    config: ClientConfig,
    credentials: Option<Credentials>,
    credential_conflict: Option<(CredentialKind, CredentialKind)>,
}

impl KagiClientBuilder {
    pub fn config(mut self, config: ClientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn bot_token(mut self, token: BotToken) -> Self {
        self.set_credentials(Credentials::from(token));
        self
    }

    pub fn session_token(mut self, token: SessionToken) -> Self {
        self.set_credentials(Credentials::from(token));
        self
    }

    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.set_credentials(credentials);
        self
    }

    pub fn build(self) -> Result<KagiClient, KagiError> {
        if let Some((already_set, attempted)) = self.credential_conflict {
            return Err(KagiError::ConflictingCredentialConfiguration {
                already_set,
                attempted,
            });
        }

        let credentials =
            self.credentials
                .ok_or_else(|| KagiError::MissingCredentialConfiguration {
                    reason: "set bot_token(...) or session_token(...) before build()".to_string(),
                })?;

        KagiClient::new(credentials, self.config)
    }

    fn set_credentials(&mut self, next_credentials: Credentials) {
        let attempted = next_credentials.kind();

        if let Some(current_credentials) = &self.credentials {
            let already_set = current_credentials.kind();
            if already_set != attempted {
                self.credential_conflict = Some((already_set, attempted));
                return;
            }
        }

        self.credentials = Some(next_credentials);
    }
}
