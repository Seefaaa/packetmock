#![allow(dead_code)]

use std::collections::HashMap;

use color_eyre::eyre::bail;

pub struct HttpHeaders {
    raw: HashMap<String, String>,
}

impl HttpHeaders {
    pub fn get(&self, name: &str) -> Option<&String> {
        self.raw.get(name)
    }
}

impl TryFrom<&[u8]> for HttpHeaders {
    type Error = color_eyre::Report;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut headers = HashMap::new();

        for line in str::from_utf8(value)?.lines().skip(1) {
            let Some((name, value)) = line.split_once(':') else {
                break;
            };

            headers.insert(
                name.to_lowercase().trim().to_string(),
                value.trim().to_string(),
            );
        }

        if !headers.is_empty() {
            return Ok(HttpHeaders { raw: headers });
        }

        bail!("No HTTP headers found");
    }
}
