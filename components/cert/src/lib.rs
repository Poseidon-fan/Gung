use anyhow::{Context, Result, bail};
use directories_next::ProjectDirs;
use std::path::PathBuf;
use std::{fs, io};
use tracing::info;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, pem::PemObject};

pub fn get_cert_key(
    key: &Option<PathBuf>,
    cert: &Option<PathBuf>,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    if let (Some(key_path), Some(cert_path)) = (key, cert) {
        load_cert_key(key_path, cert_path)
    } else {
        generate_self_signed_cert_key()
    }
}

pub fn load_certs(cert_path: &PathBuf) -> Result<Vec<CertificateDer<'static>>> {
    if cert_path.extension().is_some_and(|x| x == "der") {
        Ok(vec![CertificateDer::from(
            fs::read(cert_path).context("failed to read certificate chain file")?,
        )])
    } else {
        Ok(CertificateDer::pem_file_iter(cert_path)
            .context("failed to read PEM from certificate chain file")?
            .collect::<Result<_, _>>()
            .context("invalid PEM-encoded certificate")?)
    }
}

fn load_cert_key(
    key_path: &PathBuf,
    cert_path: &PathBuf,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let key = if key_path.extension().is_some_and(|x| x == "der") {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            fs::read(key_path).context("failed to read private key file")?,
        ))
    } else {
        PrivateKeyDer::from_pem_file(key_path)
            .context("failed to read PEM from private key file")?
    };

    let cert_chain = if cert_path.extension().is_some_and(|x| x == "der") {
        vec![CertificateDer::from(
            fs::read(cert_path).context("failed to read certificate chain file")?,
        )]
    } else {
        CertificateDer::pem_file_iter(cert_path)
            .context("failed to read PEM from certificate chain file")?
            .collect::<Result<_, _>>()
            .context("invalid PEM-encoded certificate")?
    };

    Ok((cert_chain, key))
}

fn generate_self_signed_cert_key() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>
{
    let dirs = ProjectDirs::from("org", "Gung", "gungs").unwrap();
    let path = dirs.data_local_dir().join(".cert");
    let cert_path = path.join("cert.der");
    let key_path = path.join("key.der");

    let (cert, key) = match fs::read(&cert_path).and_then(|x| Ok((x, fs::read(&key_path)?))) {
        Ok((cert, key)) => (
            CertificateDer::from(cert),
            PrivateKeyDer::try_from(key).map_err(anyhow::Error::msg)?,
        ),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            info!("generating self-signed certificate");
            let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
            let key = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
            let cert = cert.cert.into();
            fs::create_dir_all(path).context("failed to create certificate directory")?;
            fs::write(&cert_path, &cert).context("failed to write certificate")?;
            fs::write(&key_path, key.secret_pkcs8_der()).context("failed to write private key")?;
            (cert, key.into())
        }
        Err(e) => {
            bail!("failed to read certificate: {e}");
        }
    };

    Ok((vec![cert], key))
}
