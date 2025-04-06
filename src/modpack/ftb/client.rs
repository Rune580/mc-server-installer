use reqwest::Client;
use reqwest::header::HeaderMap;
use crate::modpack::ftb::model::{PackDetails, SearchResults};

#[derive(Clone, Debug)]
pub struct FtbClient {
    client: Client,
}

impl FtbClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        FtbClient {
            client,
        }
    }

    pub async fn search(
        &mut self,
        terms: &[String],
    ) -> color_eyre::Result<SearchResults> {
        let mut query = String::new();

        for (i, term) in terms.iter().enumerate() {
            let mut term = term.clone();
            if i > 0 {
                term = format!("+{}", urlencoding::encode(&term));
            }

            query += &term;
        }

        let url = format!("https://api.modpacks.ch/public/modpack/search/8?term={query}");
        let resp = self.client.get(url)
            .send()
            .await?
            .text()
            .await?;

        let results = serde_json::from_str(&resp)?;

        Ok(results)
    }

    pub async fn get_pack_details(
        &mut self,
        pack_id: usize,
    ) -> color_eyre::Result<PackDetails> {
        let url = format!("https://api.modpacks.ch/public/modpack/{pack_id}");
        let resp = self.client.get(url)
            .send()
            .await?
            .text()
            .await?;

        let details = serde_json::from_str(&resp)?;

        Ok(details)
    }
}