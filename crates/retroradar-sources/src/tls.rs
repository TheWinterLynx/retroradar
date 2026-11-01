//! Rust-only crypto for HTTP and WebSocket clients. SQLite is the native-code exception.
//! The provider is experimental upstream; see docs/RUST-DEPENDENCIES-v1.0.md.
use std::sync::Arc;

pub(crate) fn provider() -> rustls::crypto::CryptoProvider {
    rustls_rustcrypto::provider()
}

pub(crate) fn http_config() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::Rustls)
        .unversioned_rustls_crypto_provider(Arc::new(provider()))
        .build()
}

pub(crate) fn client_config() -> Result<rustls::ClientConfig, rustls::Error> {
    client_with_roots(rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    })
}

fn client_with_roots(roots: rustls::RootCertStore) -> Result<rustls::ClientConfig, rustls::Error> {
    Ok(
        rustls::ClientConfig::builder_with_provider(Arc::new(provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
    use rustls::{ClientConnection, ServerConnection};

    fn root_store() -> rustls::RootCertStore {
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(CertificateDer::from(
                include_bytes!("../fixtures/tls/ca.der").to_vec(),
            ))
            .unwrap();
        roots
    }

    fn server(version: &'static rustls::SupportedProtocolVersion) -> ServerConnection {
        let cfg = rustls::ServerConfig::builder_with_provider(Arc::new(provider()))
            .with_protocol_versions(&[version])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(
                    include_bytes!("../fixtures/tls/server.der").to_vec(),
                )],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
                    include_bytes!("../fixtures/tls/server-key.der").to_vec(),
                )),
            )
            .unwrap();
        ServerConnection::new(Arc::new(cfg)).unwrap()
    }

    fn handshake(
        name: &'static str,
        trust: bool,
        version: &'static rustls::SupportedProtocolVersion,
    ) -> Result<ClientConnection, rustls::Error> {
        let roots = if trust {
            root_store()
        } else {
            rustls::RootCertStore::empty()
        };
        let mut client = ClientConnection::new(
            Arc::new(client_with_roots(roots)?),
            ServerName::try_from(name).unwrap(),
        )?;
        let mut server = server(version);
        for _ in 0..16 {
            let mut bytes = Vec::new();
            client.write_tls(&mut bytes).unwrap();
            server.read_tls(&mut bytes.as_slice()).unwrap();
            server.process_new_packets()?;
            bytes.clear();
            server.write_tls(&mut bytes).unwrap();
            client.read_tls(&mut bytes.as_slice()).unwrap();
            client.process_new_packets()?;
            if !client.is_handshaking() && !server.is_handshaking() {
                return Ok(client);
            }
        }
        panic!("handshake did not finish");
    }

    #[test]
    fn rustcrypto_negotiates_tls12_and_tls13() {
        for version in [&rustls::version::TLS12, &rustls::version::TLS13] {
            let client = handshake("localhost", true, version).unwrap();
            assert_eq!(client.protocol_version(), Some(version.version));
        }
    }

    #[test]
    fn rustcrypto_rejects_untrusted_certificate() {
        assert!(matches!(
            handshake("localhost", false, &rustls::version::TLS13),
            Err(rustls::Error::InvalidCertificate(_))
        ));
    }

    #[test]
    fn rustcrypto_rejects_wrong_server_name() {
        assert!(matches!(
            handshake("wrong.example", true, &rustls::version::TLS13),
            Err(rustls::Error::InvalidCertificate(_))
        ));
    }

    #[test]
    fn http_uses_explicit_provider_and_certificate_verification() {
        let config = http_config();
        assert_eq!(config.provider(), ureq::tls::TlsProvider::Rustls);
        assert!(!config.disable_verification());
        assert!(config.unversioned_rustls_crypto_provider().is_some());
        assert!(
            !client_config()
                .unwrap()
                .crypto_provider()
                .cipher_suites
                .is_empty()
        );
    }
}
