//! 论坛 API——帖子 / 帖子评论 CRUD。

use crate::error::HttpError;
use crate::http::Bot;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 论坛帖子。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForumThread {
    pub thread_id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub create_time: String,
}

/// 论坛帖子评论。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForumPost {
    pub post_id: String,
    pub thread_id: String,
    pub content: String,
    pub author_id: String,
    pub create_time: String,
}

#[derive(Debug, Serialize)]
struct CreateThreadRequest<'a> {
    title: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct CreatePostRequest<'a> {
    content: &'a str,
}

impl Bot {
    /// `GET /guilds/{guild_id}/threads` —— 获取论坛帖子列表。
    pub async fn get_forum_threads(&self, guild_id: &str) -> Result<Vec<ForumThread>, HttpError> {
        let path = format!("/guilds/{guild_id}/threads");
        info!(%guild_id, "[获取论坛帖子列表]");
        self.get_json(&path).await
    }

    /// `GET /channels/{channel_id}/threads` —— 获取频道维度的论坛帖子列表（公域论坛）。
    pub async fn get_channel_threads(
        &self,
        channel_id: &str,
    ) -> Result<Vec<ForumThread>, HttpError> {
        let path = format!("/channels/{channel_id}/threads");
        info!(%channel_id, "[获取频道论坛帖子]");
        self.get_json(&path).await
    }

    /// `GET /channels/{channel_id}/threads/{thread_id}` —— 获取帖子详情。
    pub async fn get_forum_thread(
        &self,
        channel_id: &str,
        thread_id: &str,
    ) -> Result<ForumThread, HttpError> {
        let path = format!("/channels/{channel_id}/threads/{thread_id}");
        info!(%channel_id, %thread_id, "[获取论坛帖子]");
        self.get_json(&path).await
    }

    /// `POST /channels/{channel_id}/threads` —— 创建论坛帖子。
    pub async fn create_forum_thread(
        &self,
        channel_id: &str,
        title: &str,
        content: &str,
    ) -> Result<ForumThread, HttpError> {
        let path = format!("/channels/{channel_id}/threads");
        let body = CreateThreadRequest { title, content };
        info!(%channel_id, title, "[创建论坛帖子]");
        self.post_json(&path, &body).await
    }

    /// `DELETE /channels/{channel_id}/threads/{thread_id}` —— 删除论坛帖子。
    pub async fn delete_forum_thread(
        &self,
        channel_id: &str,
        thread_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/threads/{thread_id}");
        info!(%channel_id, %thread_id, "[删除论坛帖子]");
        self.delete_empty(&path).await
    }

    /// `GET /channels/{channel_id}/threads/{thread_id}/posts` —— 获取帖子评论。
    pub async fn get_forum_posts(
        &self,
        channel_id: &str,
        thread_id: &str,
    ) -> Result<Vec<ForumPost>, HttpError> {
        let path = format!("/channels/{channel_id}/threads/{thread_id}/posts");
        info!(%channel_id, %thread_id, "[获取帖子评论]");
        self.get_json(&path).await
    }

    /// `POST /channels/{channel_id}/threads/{thread_id}/posts` —— 发表评论。
    pub async fn create_forum_post(
        &self,
        channel_id: &str,
        thread_id: &str,
        content: &str,
    ) -> Result<ForumPost, HttpError> {
        let path = format!("/channels/{channel_id}/threads/{thread_id}/posts");
        let body = CreatePostRequest { content };
        info!(%channel_id, %thread_id, "[发表论坛评论]");
        self.post_json(&path, &body).await
    }

    /// `DELETE /channels/{channel_id}/threads/{thread_id}/posts/{post_id}` —— 删除评论。
    pub async fn delete_forum_post(
        &self,
        channel_id: &str,
        thread_id: &str,
        post_id: &str,
    ) -> Result<(), HttpError> {
        let path = format!("/channels/{channel_id}/threads/{thread_id}/posts/{post_id}");
        info!(%channel_id, %thread_id, %post_id, "[删除论坛评论]");
        self.delete_empty(&path).await
    }
}
