//! 多 Bot 管理器——持有多个 [`Bot`] 实例，按名称索引。
//!
//! ```ignore
//! let mut pool = BotPool::new();
//! pool.add("main", Bot::new(creds1));
//! pool.add("alt", Bot::new(creds2));
//! pool.default().post_group_message(...);
//! ```

use std::fmt;

use crate::auth::Credentials;
use crate::error::AuthError;
use crate::http::Bot;

/// 多 Bot 容器。
pub struct BotPool {
    bots: Vec<(String, Bot)>,
}

impl Default for BotPool {
    fn default() -> Self {
        Self::new()
    }
}

impl BotPool {
    /// 创建空池。
    pub fn new() -> Self {
        Self { bots: Vec::new() }
    }

    /// 添加一个 Bot 实例。
    pub fn add(&mut self, name: impl Into<String>, bot: Bot) {
        self.bots.push((name.into(), bot));
    }

    /// 从 (名称, 凭证) 列表批量构建。
    pub fn from_credentials(accounts: Vec<(String, Credentials)>) -> Result<Self, AuthError> {
        let mut pool = Self::new();
        for (name, creds) in accounts {
            pool.add(name, Bot::new(creds));
        }
        Ok(pool)
    }

    /// 按名称取 Bot。
    pub fn get(&self, name: &str) -> Option<&Bot> {
        self.bots.iter().find(|(n, _)| n == name).map(|(_, b)| b)
    }

    /// 取第一个 Bot（默认）。
    pub fn default(&self) -> Option<&Bot> {
        self.bots.first().map(|(_, b)| b)
    }

    /// 遍历所有 (名称, Bot)。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Bot)> {
        self.bots.iter().map(|(n, b)| (n.as_str(), b))
    }

    /// Bot 数量。
    pub fn len(&self) -> usize {
        self.bots.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.bots.is_empty()
    }

    /// 取出所有 Bot 的 Vec<Bot>（丢弃名称）。
    pub fn into_bots(self) -> Vec<Bot> {
        self.bots.into_iter().map(|(_, b)| b).collect()
    }
}

impl fmt::Debug for BotPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.bots.iter().map(|(n, _)| n.as_str()).collect();
        f.debug_struct("BotPool")
            .field("count", &self.bots.len())
            .field("names", &names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pool_has_no_default() {
        let pool = BotPool::new();
        assert!(pool.default().is_none());
        assert!(pool.is_empty());
    }

    #[test]
    fn add_and_retrieve() {
        let mut pool = BotPool::new();
        let c1 = Credentials::new("111", "secret1");
        let c2 = Credentials::new("222", "secret2");
        pool.add("bot-a", Bot::new(c1));
        pool.add("bot-b", Bot::new(c2));
        assert_eq!(pool.len(), 2);
        assert!(pool.get("bot-a").is_some());
        assert!(pool.get("bot-c").is_none());
        assert_eq!(pool.default().unwrap().app_id(), "111");
    }

    #[test]
    fn into_bots_drops_names() {
        let mut pool = BotPool::new();
        pool.add("x", Bot::new(Credentials::new("a", "b")));
        let bots = pool.into_bots();
        assert_eq!(bots.len(), 1);
        assert_eq!(bots[0].app_id(), "a");
    }
}
