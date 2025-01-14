use anyhow::{anyhow, bail, Result};
use reqwest::Url;
use std::path::{Path, PathBuf};
use std::{io, thread};
use tempfile::NamedTempFile;

pub fn get_local_path(filename: &str) -> Result<Box<dyn AsRef<Path>>> {
    if filename.starts_with("https://") || filename.starts_with("http://") {
        match download_asciicast(filename) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => bail!(anyhow!("download failed: {e}")),
        }
    } else {
        Ok(Box::new(PathBuf::from(filename)))
    }
}

const LINK_REL_SELECTOR: &str = r#"link[rel="alternate"][type="application/x-asciicast"], link[rel="alternate"][type="application/asciicast+json"]"#;

fn download_asciicast(url: &str) -> Result<NamedTempFile> {
    use reqwest::blocking::get;
    use scraper::{Html, Selector};

    let mut response = get(Url::parse(url)?)?;
    response.error_for_status_ref()?;
    let mut file = NamedTempFile::new()?;

    let content_type = response
        .headers()
        .get("content-type")
        .ok_or(anyhow!("no content-type header in the response"))?
        .to_str()?;

    if content_type.starts_with("text/html") {
        let document = Html::parse_document(&response.text()?);
        let selector = Selector::parse(LINK_REL_SELECTOR).unwrap();
        let mut elements = document.select(&selector);

        if let Some(url) = elements.find_map(|e| e.value().attr("href")) {
            let mut response = get(Url::parse(url)?)?;
            response.error_for_status_ref()?;
            io::copy(&mut response, &mut file)?;

            Ok(file)
        } else {
            bail!(
                r#"<link rel="alternate" type="application/x-asciicast" href="..."> not found in the HTML page"#
            );
        }
    } else {
        io::copy(&mut response, &mut file)?;

        Ok(file)
    }
}

pub struct JoinHandle(Option<thread::JoinHandle<()>>);

impl JoinHandle {
    pub fn new(handle: thread::JoinHandle<()>) -> Self {
        Self(Some(handle))
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        self.0
            .take()
            .unwrap()
            .join()
            .expect("worker thread should finish cleanly");
    }
}
