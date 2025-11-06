use std::collections::HashMap;

use anyhow::{Result, anyhow};
use fancy_regex::Regex;
use serde_json::Value;

use crate::extractor::extract::YtExtractor;

pub trait ExtractorJsonHandle {
    fn search_json(
        &self,
        start_pattern: &str,
        html: &str,
        end_pattern: Option<&str>,
        default: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Value>>;
    fn get_text(
        &self,
        data: &Value,
        path_list: Option<Vec<Vec<&str>>>,
        max_runs: Option<usize>,
    ) -> Option<String>;
}

impl ExtractorJsonHandle for YtExtractor {
    fn get_text(
        &self,
        data: &Value,
        path_list: Option<Vec<Vec<&str>>>,
        max_runs: Option<usize>,
    ) -> Option<String> {
        let paths = path_list.unwrap_or_else(|| vec![vec![]]);
        for path in paths {
            let mut current = data;
            for key in &path {
                if !current.is_object() {
                    current = &Value::Null;
                    break;
                }
                current = current.get(*key).unwrap_or(&Value::Null);
            }

            let objs: Vec<&Value> = if path.is_empty() {
                vec![data]
            } else if !current.is_null() {
                vec![current]
            } else {
                continue;
            };

            for item in objs {
                if let Some(text) = item.get("simpleText").and_then(|v| v.as_str()) {
                    return Some(text.to_string());
                }

                let mut runs: Vec<Value> = item
                    .get("runs")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(|| {
                        if let Some(arr) = item.as_array() {
                            arr.clone()
                        } else {
                            vec![]
                        }
                    });

                if runs.is_empty() {
                    continue;
                }

                if let Some(limit) = max_runs {
                    runs.truncate(limit.min(runs.len()));
                }

                let text = runs
                    .iter()
                    .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                    .collect::<String>();

                if !text.is_empty() {
                    return Some(text);
                }
            }
        }

        None
    }

    fn search_json(
        &self,
        start_pattern: &str,
        html: &str,
        end_pattern: Option<&str>,
        default: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Value>> {
        let default_value = default.unwrap_or_default();
        let end_pattern = end_pattern.unwrap_or("");

        let re_start =
            Regex::new(start_pattern).map_err(|e| anyhow!("Invalid start regex: {e}"))?;
        let re_end = if !end_pattern.is_empty() {
            Some(Regex::new(end_pattern).map_err(|e| anyhow!("Invalid end regex: {e}"))?)
        } else {
            None
        };

        let start_pos = if let Some(m) = re_start.find(html)? {
            m.end()
        } else {
            return Ok(default_value);
        };

        let mut json_start = None;
        let mut depth = 0usize;
        let mut in_str = false;
        let mut escape = false;

        let chars: Vec<char> = html[start_pos..].chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if json_start.is_none() {
                if c == '{' {
                    json_start = Some(i);
                    depth = 1;
                }
                continue;
            }

            if in_str {
                if escape {
                    escape = false;
                    continue;
                }
                if c == '\\' {
                    escape = true;
                    continue;
                }
                if c == '"' {
                    in_str = false;
                }
            } else {
                match c {
                    '"' => in_str = true,
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            let json_str: String = chars[json_start.unwrap()..=i].iter().collect();
                            if let Some(re_end) = &re_end {
                                if let Some(m_end) = re_end.find(&html[start_pos + i..])? {
                                    let _ = m_end;
                                }
                            }

                            return serde_json::from_str(&json_str)
                                .map_err(|e| anyhow!("Failed to parse JSON: {e}\n{json_str}"))
                                .or_else(|_| Ok(default_value.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(default_value)
    }
}
