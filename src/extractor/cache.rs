use anyhow::Result;

use crate::extractor::{extract::YtExtractor, player::ExtractorPlayerHandle};

pub trait ExtractorCacheHandle {
    fn store_player_data_from_cache(
        &mut self,
        name: &str,
        player_url: String,
        data: String,
    ) -> Result<()>;
    fn player_js_cache_key(&self, player_url: &String) -> Result<String>;
    fn load_player_data_from_cache(
        &mut self,
        name: &str,
        player_url: String,
    ) -> Result<Option<String>>;
}

impl ExtractorCacheHandle for YtExtractor {
    fn player_js_cache_key(&self, player_url: &String) -> Result<String> {
        let (player_id, player_path) = self.get_player_id_and_path(player_url)?;

        /*
        ! SKIPPED PYTHON SNIPPET:
        if not variant:
           variant = re.sub(r'[^a-zA-Z0-9]', '_', remove_end(player_path, '.js'))
        */
        Ok(format!("{}-{}", player_id, player_path))
    }

    fn load_player_data_from_cache(
        &mut self,
        name: &str,
        player_url: String,
    ) -> Result<Option<String>> {
        let cache_id = (
            format!("youtube-{}", name),
            self.player_js_cache_key(&player_url)?,
        );

        if let Some(data) = self.player_cache.get(&cache_id) {
            return Ok(Some(data.clone()));
        }

        Ok(None)
    }

    fn store_player_data_from_cache(
        &mut self,
        name: &str,
        player_url: String,
        data: String,
    ) -> Result<()> {
        let cache_id = (
            format!("youtube-{}", name),
            self.player_js_cache_key(&player_url)?,
        );

        if !self.player_cache.contains_key(&cache_id) {
            self.player_cache.insert(cache_id, data);
            return Ok(());
        }

        Ok(())
    }
}
