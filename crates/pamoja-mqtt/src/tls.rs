//! TLS between the client and the broker.
//!
//! The connection runs over rustls with its ring provider, chosen explicitly so the client
//! works whatever other TLS providers a program links. MQTT over TLS conventionally uses
//! port 8883, and the broker's certificate is checked against the host name the
//! configuration connects to.

use std::sync::Arc;

use pamoja_core::{Error, Result};
use rustls::crypto::ring::default_provider;
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};

/// How a connection to the broker is secured.
///
/// Start from [`Tls::system_roots`] for a broker whose certificate a public authority
/// issued, or [`Tls::with_ca_pem`] for one whose certificate comes from an authority of its
/// own, and add [`client_certificate`](Tls::client_certificate) when the broker asks each
/// client to prove who it is.
///
/// # Examples
///
/// ```
/// use pamoja_mqtt::{MqttConfig, Tls};
///
/// // A hosted broker with a publicly issued certificate, on the MQTT over TLS port.
/// let config = MqttConfig::new("sensor-1", "broker.example.com", 8883).tls(Tls::system_roots());
/// assert!(config.uses_tls());
/// ```
#[derive(Clone)]
pub struct Tls {
    roots: Roots,
    identity: Option<(Vec<u8>, Vec<u8>)>,
}

#[derive(Clone)]
enum Roots {
    System,
    Pem(Vec<u8>),
}

impl Tls {
    /// Trusts the certificate authorities the operating system trusts.
    ///
    /// # Returns
    ///
    /// The TLS settings, presenting no client certificate.
    pub fn system_roots() -> Tls {
        Tls {
            roots: Roots::System,
            identity: None,
        }
    }

    /// Trusts only the certificate authorities in a PEM file.
    ///
    /// A broker on a private network usually holds a certificate its operator issued, so
    /// the client trusts that operator's authority and nothing else.
    ///
    /// # Arguments
    ///
    /// * `pem` - one or more `CERTIFICATE` blocks.
    ///
    /// # Returns
    ///
    /// The TLS settings, presenting no client certificate.
    pub fn with_ca_pem(pem: impl Into<Vec<u8>>) -> Tls {
        Tls {
            roots: Roots::Pem(pem.into()),
            identity: None,
        }
    }

    /// Presents a client certificate, for a broker that authenticates each client by one.
    ///
    /// # Arguments
    ///
    /// * `certificate_pem` - the client's certificate, followed by any intermediates.
    /// * `key_pem` - the certificate's private key, in PKCS #8, PKCS #1, or SEC1 form.
    ///
    /// # Returns
    ///
    /// The updated settings, for chaining.
    pub fn client_certificate(
        mut self,
        certificate_pem: impl Into<Vec<u8>>,
        key_pem: impl Into<Vec<u8>>,
    ) -> Tls {
        self.identity = Some((certificate_pem.into(), key_pem.into()));
        self
    }

    /// Builds the rustls configuration these settings describe.
    ///
    /// # Returns
    ///
    /// The configuration, shared.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when a PEM file holds no usable certificate or key, the
    /// system trusts no authority rustls can read, or the key does not match the
    /// certificate.
    pub(crate) fn client_config(&self) -> Result<Arc<ClientConfig>> {
        let mut roots = RootCertStore::empty();
        match &self.roots {
            Roots::System => {
                let found = rustls_native_certs::load_native_certs();
                for certificate in found.certs {
                    let _ = roots.add(certificate);
                }
                if roots.is_empty() {
                    return Err(Error::Transport(
                        "the system trusts no certificate authority this client can read".into(),
                    ));
                }
            }
            Roots::Pem(pem) => {
                for certificate in certificates(pem, "the CA file")? {
                    roots.add(certificate).map_err(|error| {
                        Error::Transport(format!(
                            "the CA file holds a certificate rustls refuses: {error}"
                        ))
                    })?;
                }
            }
        }
        let builder = ClientConfig::builder_with_provider(Arc::new(default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|error| Error::Transport(error.to_string()))?
            .with_root_certificates(roots);
        let config = match &self.identity {
            None => builder.with_no_client_auth(),
            Some((certificate, key)) => {
                let chain = certificates(certificate, "the client certificate")?;
                let key = PrivateKeyDer::from_pem_slice(key).map_err(|error| {
                    Error::Transport(format!("the client key is not a PEM private key: {error}"))
                })?;
                builder.with_client_auth_cert(chain, key).map_err(|error| {
                    Error::Transport(format!("the client certificate cannot be used: {error}"))
                })?
            }
        };
        Ok(Arc::new(config))
    }
}

// Private keys never reach a log or a panic message through this type.
impl core::fmt::Debug for Tls {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let roots = match self.roots {
            Roots::System => "system",
            Roots::Pem(_) => "pem",
        };
        f.debug_struct("Tls")
            .field("roots", &roots)
            .field("client_certificate", &self.identity.is_some())
            .finish()
    }
}

fn certificates(pem: &[u8], what: &str) -> Result<Vec<CertificateDer<'static>>> {
    let found = CertificateDer::pem_slice_iter(pem)
        .collect::<core::result::Result<Vec<_>, _>>()
        .map_err(|error| Error::Transport(format!("{what} is not PEM: {error}")))?;
    if found.is_empty() {
        return Err(Error::Transport(format!(
            "{what} holds no CERTIFICATE block"
        )));
    }
    Ok(found)
}
