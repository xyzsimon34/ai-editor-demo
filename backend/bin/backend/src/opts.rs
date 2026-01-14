use std::{fs, io::Read, path::PathBuf};

use atb_cli_utils::clap::{self, Parser, ValueHint};
use atb_types::{
    Duration,
    jwt::HEADER_RS256,
    prelude::{
        Builder, Claims,
        jsonwebtoken::{DecodingKey, EncodingKey},
    },
};
use axum_client_ip::ClientIpSource;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Parser)]
pub struct HttpOpts {
    /// Address/port for the HTTP listener
    #[arg(long, env = "BACKEND_HOST", default_value = "0.0.0.0:3030")]
    pub host: String,

    #[arg(
        long,
        value_delimiter = ';',
        default_value = "http://localhost:8080;http://127.0.0.1:8080;http://localhost:3000;http://127.0.0.1:3001;http://localhost:3001;http://127.0.0.1:3001",
        env = "BACKEND_CORS_ORIGINS"
    )]
    pub origins: Vec<String>,

    // Client IP extraction source (default: raw socket via ConnectInfo).
    #[arg(long, default_value = "ConnectInfo", env = "BACKEND_CLIENT_IP_SOURCE")]
    pub client_ip_source: ClientIpSource,

    /// JWT Private Key PEM
    #[arg(
        long,
        env = "BACKEND_JWT_PRIV",
        value_hint = ValueHint::FilePath,
    )]
    pub jwt_priv_key: Option<PathBuf>,

    /// JWT Public Key PEM
    #[arg(
        long,
        env = "BACKEND_JWT_PUB",
        value_hint = ValueHint::FilePath,
    )]
    pub jwt_pub_key: Option<PathBuf>,
}

impl HttpOpts {
    pub fn load_jwt(&self) -> anyhow::Result<(Encoder, Decoder)> {
        Ok(match (&self.jwt_priv_key, &self.jwt_pub_key) {
            (Some(priv_file), Some(pub_file)) => {
                tracing::info!("loading jwt from files");
                (
                    Encoder(EncodingKey::from_rsa_pem(&load_file(priv_file)?)?),
                    Decoder(DecodingKey::from_rsa_pem(&load_file(pub_file)?)?),
                )
            }
            (None, None) => {
                tracing::info!("loading jwt from fixtures");
                (
                    Encoder(atb::fixtures::jwt::JWT_ENCODING_KEY.clone()),
                    Decoder(atb::fixtures::jwt::JWT_DECODING_KEY.clone()),
                )
            }
            _ => return Err(anyhow::anyhow!("jwt cannot be disjoint")),
        })
    }
}

#[derive(Clone)]
pub struct Encoder(EncodingKey);

impl Encoder {
    pub fn claims_encoded<S, C>(
        &self,
        subject: S,
        audience: Vec<String>,
        expiry_duration: Duration,
        custom: C,
    ) -> anyhow::Result<(String, String, i64)>
    where
        S: std::fmt::Display,
        C: Serialize + DeserializeOwned + Default,
    {
        let (claim, fingerprint) = Self::claims(subject, audience, expiry_duration, custom);
        let encoded = claim.encode(&HEADER_RS256, &self.0)?;
        Ok((encoded, fingerprint, claim.expiry()))
    }

    fn claims<S, C>(
        subject: S,
        audience: Vec<String>,
        expiry_duration: Duration,
        custom: C,
    ) -> (Claims<C>, String)
    where
        S: std::fmt::Display,
        C: Serialize + Default,
    {
        Builder::with_custom("tt", expiry_duration, custom)
            .subject(subject)
            .audience(audience)
            .build_fingerprinted()
    }
}

pub fn load_file(file: &PathBuf) -> anyhow::Result<Vec<u8>> {
    let mut f = fs::File::open(file)?;
    let metadata = fs::metadata(file)?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read_exact(&mut buffer)?;
    Ok(buffer)
}

#[derive(Clone)]
pub struct Decoder(pub DecodingKey);

#[derive(Debug, Clone, Parser)]
pub struct DatabaseOpts {
    /// Database connection
    #[arg(
        long,
        default_value = "postgres://postgres:123456@localhost:5433/postgres?sslmode=disable",
        env = "BACKEND_POSTGRES"
    )]
    pub postgres: String,

    /// Run Migrations
    #[arg(long, default_value = "false", env = "BACKEND_MIGRATE")]
    pub migrate: bool,
}

#[derive(Clone, Debug, Parser)]
pub struct TemporalOpts {
    /// Temporal Server URL
    #[clap(
        long,
        default_value = "http://localhost:7233",
        env = "BACKEND_TEMPORAL"
    )]
    pub temporal: String,

    /// Temporal namespace
    #[clap(long, default_value = "default", env = "BACKEND_TEMPORAL_NAMESPACE")]
    pub namespace: String,

    /// Temporal task queue
    #[clap(long, default_value = "default", env = "BACKEND_TEMPORAL_TASK_QUEUE")]
    pub task_queue: String,
}

#[derive(Clone, Debug, Parser)]
pub struct WorkerOpts {
    #[clap(flatten)]
    pub temporal: TemporalOpts,

    /// Temporal max cached workflows
    #[clap(
        long,
        default_value = "1000",
        env = "BACKEND_TEMPORAL_MAX_CACHED_WORKFLOWS"
    )]
    pub max_cached_workflows: usize,
}

#[derive(Clone, Debug, Parser)]
pub struct Opts {
    #[arg(long, env = "OPENAI_API_KEY")]
    pub openai_api_key: String,
}
