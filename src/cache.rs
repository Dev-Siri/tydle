use std::hash::Hash;

use anyhow::{Result, anyhow};
use dashmap::DashMap;
use fancy_regex::Regex;
use url::Url;

pub struct MemoryCacheStore<T = String> {
    cache: DashMap<T, String>,
}

impl MemoryCacheStore {
    pub fn new<T: Eq + Hash>() -> MemoryCacheStore<T> {
        MemoryCacheStore {
            cache: DashMap::default(),
        }
    }
}

pub trait CacheAccess<T = String> {
    fn add(&self, key: T, value: String) -> Result<()>;
    fn contains(&self, key: &T) -> Result<bool>;
    fn get(&self, key: &T) -> Result<Option<String>>;
}

pub trait PlayerCacheHandle {
    fn get_player_id_and_path(&self, player_url: &String) -> Result<(String, String)>;
    fn extract_player_info(&self, player_url: &String) -> Result<String>;
    // fn store_player_data_from_cache(
    //     &mut self,
    //     name: &str,
    //     player_url: String,
    //     data: String,
    // ) -> Result<()>;
    fn player_js_cache_key(&self, player_url: &String) -> Result<String>;
    fn load_player_data_from_cache(&self, name: &str, player_url: String)
    -> Result<Option<String>>;
}

impl<T> CacheAccess<T> for MemoryCacheStore<T>
where
    T: Eq + Hash,
{
    fn get(&self, key: &T) -> Result<Option<String>> {
        let value = self.cache.get(key);
        Ok(value.map(|v| v.value().clone()))
    }

    fn add(&self, key: T, value: String) -> Result<()> {
        self.cache.insert(key, value);
        Ok(())
    }

    fn contains(&self, key: &T) -> Result<bool> {
        Ok(self.cache.contains_key(key))
    }
}

impl PlayerCacheHandle for MemoryCacheStore<(String, String)> {
    fn extract_player_info(&self, player_url: &String) -> Result<String> {
        const PLAYER_INFO_RE: [&str; 3] = [
            r"/s/player/(?P<id>[a-zA-Z0-9_-]{8,})/(?:tv-)?player",
            r"/(?P<id>[a-zA-Z0-9_-]{8,})/player(?:_ias\.vflset(?:/[a-zA-Z]{2,3}_[a-zA-Z]{2,3})?|-plasma-ias-(?:phone|tablet)-[a-z]{2}_[A-Z]{2}\.vflset)/base\.js$",
            r"\b(?P<id>vfl[a-zA-Z0-9_-]+)\b.*?\.js$",
        ];

        for player_info_re in PLAYER_INFO_RE {
            let re = Regex::new(player_info_re)?;
            if let Ok(Some(caps)) = re.captures(player_url) {
                if let Some(matched) = caps.name("id") {
                    return Ok(matched.as_str().to_string());
                }
            }
        }

        Err(anyhow!("Cannot identify player: {}", player_url))
    }

    fn get_player_id_and_path(&self, player_url: &String) -> Result<(String, String)> {
        let player_id = self.extract_player_info(player_url)?;
        let player_path = Url::parse(player_url)?.path().to_string();

        Ok((player_id, player_path))
    }

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
        &self,
        name: &str,
        player_url: String,
    ) -> Result<Option<String>> {
        let cache_id = (
            format!("youtube-{}", name),
            self.player_js_cache_key(&player_url)?,
        );

        if let Some(data) = self.cache.get(&cache_id) {
            return Ok(Some(data.clone()));
        }

        Ok(None)
    }

    // fn store_player_data_from_cache(
    //     &mut self,
    //     name: &str,
    //     player_url: String,
    //     data: String,
    // ) -> Result<()> {
    //     let cache_id = (
    //         format!("youtube-{}", name),
    //         self.player_js_cache_key(&player_url)?,
    //     );

    //     if !self.cache.contains_key(&cache_id) {
    //         self.cache.insert(cache_id, data);
    //         return Ok(());
    //     }

    //     Ok(())
    // }
}
