//! 端到端联调用 demo bot——除了 echo，逐项演示 SDK 已实现的消息能力。
//!
//! ## 命令
//!
//! 群 / c2c（v2）：
//! - `测试撤回`、`测试图片`、`测试markdown`、`测试键盘`、`测试分享链接`
//!
//! 频道（v1）：
//! - `测试撤回`、`测试markdown`、`测试embed`、`测试引用`、`测试表态`、`测试私信`
//!
//! ## 跨场景的支持矩阵（按 QQ 官方文档）
//!
//! | 类型 | c2c | 群 | 文字子频道 | 频道私信 |
//! |---|---|---|---|---|
//! | text | ✓ | ✓ | ✓ | ✓ |
//! | markdown | ✓ 全开放 | ✓ 全开放 | ✓ 需 QQ 邀请激活 | ✓ 需邀请 |
//! | media（图 / 视频 / 语音）| ✓ | ✓ | ✓ | ✓ |
//! | ark | ✓ | ✓ | ✓ | ✓ |
//! | embed | ❌ | ❌ | ✓ | ✓ |
//! | message_reference | 字段在协议但客户端不渲染 | 同左 | ✓ 引用卡片 | ✓ |
//!
//! 群 / c2c 不演示 embed / 引用——前者强发会被客户端 fallback 成"表情"占位，
//! 后者字段虽收但客户端无可见渲染。ark 模板要平台备案 `template_id`，没现成模板
//! 难做开箱演示，本 demo 未涉及。
//!
//! 用户 / 群管理事件（FRIEND_ADD / DEL、C2C_MSG_REJECT / RECEIVE、
//! GROUP_ADD_ROBOT / DEL_ROBOT、GROUP_MSG_REJECT / RECEIVE）只能由真实用户操作
//! 触发——demo 仅 `info!` 打日志。
//!
//! 撤回 demo 都**只删 bot 自己的回复**——不依赖 bot 是管理员，任意账号都能复现。
//!
//! ## 运行
//!
//! ```sh
//! cargo run --example run_bot
//! ```
//! 准备：`.env` 填好 `QQ_BOT_APP_ID` / `QQ_BOT_APP_SECRET`，平台开公域消息事件 + 主动消息权限。

use std::time::Duration;

use async_trait::async_trait;
use qq_bot_rs::prelude::*;
use qq_bot_rs::types::interaction::{ChatType, Interaction};
use qq_bot_rs::types::keyboard::{Action, Button, Keyboard, KeyboardRow, Permission, RenderData};
use qq_bot_rs::types::manage::{GroupManageEvent, UserManageEvent};
use qq_bot_rs::types::payloads::{EmbedField, EmbedThumbnail};
use qq_bot_rs::{EmojiType, InteractionCallbackCode};
use tokio::time::sleep;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

// 给用户肉眼确认有这条消息再删——v2 群 / c2c 协议规定 2 分钟内必须撤回。
const RECALL_DELAY: Duration = Duration::from_secs(3);

// QQ 服务端去拉这个 URL，本地不传二进制——要求 https + 外网可访问。
const DEMO_IMAGE_URL: &str = "https://picsum.photos/seed/qqbot/400/300";

const CMD_RECALL: &str = "测试撤回";
const CMD_MEDIA: &str = "测试图片";
const CMD_MARKDOWN: &str = "测试markdown";
const CMD_KEYBOARD: &str = "测试键盘";
const CMD_EMBED: &str = "测试embed";
const CMD_QUOTE: &str = "测试引用";
const CMD_REACTION: &str = "测试表态";
const CMD_DM: &str = "测试私信";
const CMD_SHARE: &str = "测试分享链接";

// QQ 系统表情索引："4" = 龇牙。
// <https://bot.q.qq.com/wiki/develop/api-v2/openapi/emoji/model.html>
const DEMO_EMOJI_ID: &str = "4";

// 用户走链接添加 bot 时回带到 `FRIEND_ADD.scene_param`，≤ 32 字符。
const SHARE_CALLBACK_DATA: &str = "demo-share";

struct DemoBot;

#[async_trait]
impl EventHandler for DemoBot {
    async fn on_ready(&self, _bot: &Bot, ready: qq_bot_rs::types::gateway::ReadyData) {
        info!(bot = %ready.user.username, "ready");
    }

    async fn on_at_message_create(&self, bot: &Bot, msg: ChannelMessage) {
        let c = msg.content.as_str();
        if c.contains(CMD_RECALL) {
            demo_recall_channel(bot, &msg).await;
        } else if c.contains(CMD_MARKDOWN) {
            demo_markdown_channel(bot, &msg).await;
        } else if c.contains(CMD_EMBED) {
            demo_embed_channel(bot, &msg).await;
        } else if c.contains(CMD_QUOTE) {
            demo_quote_channel(bot, &msg).await;
        } else if c.contains(CMD_REACTION) {
            demo_reaction_channel(bot, &msg).await;
        } else if c.contains(CMD_DM) {
            demo_dm_channel(bot, &msg).await;
        } else {
            echo_channel(bot, &msg).await;
        }
    }

    async fn on_group_at_message_create(&self, bot: &Bot, msg: GroupMessage) {
        let c = msg.content.as_str();
        if c.contains(CMD_RECALL) {
            demo_recall_group(bot, &msg).await;
        } else if c.contains(CMD_MEDIA) {
            demo_media_group(bot, &msg).await;
        } else if c.contains(CMD_KEYBOARD) {
            demo_keyboard_group(bot, &msg).await;
        } else if c.contains(CMD_MARKDOWN) {
            demo_markdown_group(bot, &msg).await;
        } else if c.contains(CMD_SHARE) {
            demo_share_group(bot, &msg).await;
        } else {
            echo_group(bot, &msg).await;
        }
    }

    async fn on_c2c_message_create(&self, bot: &Bot, msg: C2cMessage) {
        let c = msg.content.as_str();
        if c.contains(CMD_RECALL) {
            demo_recall_c2c(bot, &msg).await;
        } else if c.contains(CMD_MEDIA) {
            demo_media_c2c(bot, &msg).await;
        } else if c.contains(CMD_KEYBOARD) {
            demo_keyboard_c2c(bot, &msg).await;
        } else if c.contains(CMD_MARKDOWN) {
            demo_markdown_c2c(bot, &msg).await;
        } else if c.contains(CMD_SHARE) {
            demo_share_c2c(bot, &msg).await;
        } else {
            echo_c2c(bot, &msg).await;
        }
    }

    async fn on_interaction_create(&self, bot: &Bot, i: Interaction) {
        // ACK 失败也继续——至少把反馈消息发出去，按钮 loading 圈不消失而已。
        if let Err(e) = bot
            .put_interaction_callback(&i.id, InteractionCallbackCode::Success)
            .await
        {
            error!(error = %e, interaction_id = %i.id, "interaction: ack failed");
        } else {
            info!(interaction_id = %i.id, button_id = %i.data.resolved.button_id, "interaction: acked");
        }

        let label = format!(
            "收到按钮回调 button_id={} data={}",
            i.data.resolved.button_id, i.data.resolved.button_data,
        );
        match i.chat_type {
            ChatType::Group => {
                if let (Some(group), Some(mid)) = (&i.group_openid, &i.data.resolved.message_id) {
                    let reply = OutgoingMessage::text(label).reply_to(mid);
                    if let Err(e) = bot.post_group_message(group, &reply).await {
                        error!(error = %e, "interaction: group reply failed");
                    }
                }
            }
            ChatType::C2c => {
                if let (Some(user), Some(mid)) = (&i.user_openid, &i.data.resolved.message_id) {
                    let reply = OutgoingMessage::text(label).reply_to(mid);
                    if let Err(e) = bot.post_c2c_message(user, &reply).await {
                        error!(error = %e, "interaction: c2c reply failed");
                    }
                }
            }
            ChatType::Guild => {
                if let (Some(channel), Some(mid)) = (&i.channel_id, &i.data.resolved.message_id) {
                    let reply = OutgoingChannelMessage::text(label).reply_to(mid);
                    if let Err(e) = bot.post_channel_message(channel, &reply).await {
                        error!(error = %e, "interaction: channel reply failed");
                    }
                }
            }
        }
    }

    async fn on_friend_add(&self, _bot: &Bot, e: UserManageEvent) {
        info!(openid = %e.openid, scene = ?e.scene, scene_param = ?e.scene_param, "FRIEND_ADD");
    }

    async fn on_friend_del(&self, _bot: &Bot, e: UserManageEvent) {
        info!(openid = %e.openid, "FRIEND_DEL");
    }

    async fn on_c2c_msg_reject(&self, _bot: &Bot, e: UserManageEvent) {
        info!(openid = %e.openid, "C2C_MSG_REJECT");
    }

    async fn on_c2c_msg_receive(&self, _bot: &Bot, e: UserManageEvent) {
        info!(openid = %e.openid, "C2C_MSG_RECEIVE");
    }

    async fn on_group_add_robot(&self, _bot: &Bot, e: GroupManageEvent) {
        info!(group = %e.group_openid, op = %e.op_member_openid, "GROUP_ADD_ROBOT");
    }

    async fn on_group_del_robot(&self, _bot: &Bot, e: GroupManageEvent) {
        info!(group = %e.group_openid, op = %e.op_member_openid, "GROUP_DEL_ROBOT");
    }

    async fn on_group_msg_reject(&self, _bot: &Bot, e: GroupManageEvent) {
        info!(group = %e.group_openid, op = %e.op_member_openid, "GROUP_MSG_REJECT");
    }

    async fn on_group_msg_receive(&self, _bot: &Bot, e: GroupManageEvent) {
        info!(group = %e.group_openid, op = %e.op_member_openid, "GROUP_MSG_RECEIVE");
    }
}

async fn echo_channel(bot: &Bot, msg: &ChannelMessage) {
    let reply = OutgoingChannelMessage::text(format!("echo: {}", msg.content)).reply_to(&msg.id);
    if let Err(e) = bot.post_channel_message(&msg.channel_id, &reply).await {
        error!(error = %e, "channel reply failed");
    }
}

async fn echo_group(bot: &Bot, msg: &GroupMessage) {
    let reply = OutgoingMessage::text(format!("echo: {}", msg.content)).reply_to(&msg.id);
    if let Err(e) = bot.post_group_message(&msg.group_openid, &reply).await {
        error!(error = %e, "group reply failed");
    }
}

async fn echo_c2c(bot: &Bot, msg: &C2cMessage) {
    let reply = OutgoingMessage::text(format!("echo: {}", msg.content)).reply_to(&msg.id);
    if let Err(e) = bot.post_c2c_message(&msg.author.user_openid, &reply).await {
        error!(error = %e, "c2c reply failed");
    }
}

async fn demo_recall_channel(bot: &Bot, msg: &ChannelMessage) {
    let reply = OutgoingChannelMessage::text("3 秒后撤回此消息").reply_to(&msg.id);
    let sent = match bot.post_channel_message(&msg.channel_id, &reply).await {
        Ok(s) => s,
        Err(e) => return log_err("recall: send", e),
    };
    info!(reply_id = %sent.id, "recall: posted, waiting");
    sleep(RECALL_DELAY).await;
    if let Err(e) = bot
        .delete_channel_message(&msg.channel_id, &sent.id, true)
        .await
    {
        error!(error = %e, "recall: delete failed");
    } else {
        info!("recall: channel deleted");
    }
}

async fn demo_recall_group(bot: &Bot, msg: &GroupMessage) {
    let reply = OutgoingMessage::text("3 秒后撤回此消息").reply_to(&msg.id);
    let sent = match bot.post_group_message(&msg.group_openid, &reply).await {
        Ok(s) => s,
        Err(e) => return log_err("recall: send", e),
    };
    info!(reply_id = %sent.id, "recall: posted, waiting");
    sleep(RECALL_DELAY).await;
    if let Err(e) = bot.delete_group_message(&msg.group_openid, &sent.id).await {
        error!(error = %e, "recall: delete failed");
    } else {
        info!("recall: group deleted");
    }
}

async fn demo_recall_c2c(bot: &Bot, msg: &C2cMessage) {
    let reply = OutgoingMessage::text("3 秒后撤回此消息").reply_to(&msg.id);
    let sent = match bot.post_c2c_message(&msg.author.user_openid, &reply).await {
        Ok(s) => s,
        Err(e) => return log_err("recall: send", e),
    };
    info!(reply_id = %sent.id, "recall: posted, waiting");
    sleep(RECALL_DELAY).await;
    if let Err(e) = bot
        .delete_c2c_message(&msg.author.user_openid, &sent.id)
        .await
    {
        error!(error = %e, "recall: delete failed");
    } else {
        info!("recall: c2c deleted");
    }
}

async fn demo_media_group(bot: &Bot, msg: &GroupMessage) {
    let media = match bot
        .post_group_file(&msg.group_openid, FileType::Image, DEMO_IMAGE_URL, false)
        .await
    {
        Ok(m) => m,
        Err(e) => return log_err("media: upload", e),
    };
    info!(file_uuid = ?media.file_uuid, ttl = media.ttl, "media: uploaded");
    let reply = OutgoingMessage::media(media).reply_to(&msg.id);
    if let Err(e) = bot.post_group_message(&msg.group_openid, &reply).await {
        error!(error = %e, "media: send failed");
    }
}

async fn demo_media_c2c(bot: &Bot, msg: &C2cMessage) {
    let media = match bot
        .post_c2c_file(
            &msg.author.user_openid,
            FileType::Image,
            DEMO_IMAGE_URL,
            false,
        )
        .await
    {
        Ok(m) => m,
        Err(e) => return log_err("media: upload", e),
    };
    info!(file_uuid = ?media.file_uuid, ttl = media.ttl, "media: uploaded");
    let reply = OutgoingMessage::media(media).reply_to(&msg.id);
    if let Err(e) = bot.post_c2c_message(&msg.author.user_openid, &reply).await {
        error!(error = %e, "media: send failed");
    }
}

async fn demo_markdown_channel(bot: &Bot, msg: &ChannelMessage) {
    let md = MarkdownPayload::raw(SAMPLE_MARKDOWN);
    let reply = OutgoingChannelMessage::markdown(md).reply_to(&msg.id);
    if let Err(e) = bot.post_channel_message(&msg.channel_id, &reply).await {
        error!(error = %e, "markdown: channel send failed (频道 markdown 需 QQ 内部邀请激活)");
    }
}

async fn demo_markdown_group(bot: &Bot, msg: &GroupMessage) {
    let md = MarkdownPayload::raw(SAMPLE_MARKDOWN);
    let reply = OutgoingMessage::markdown(md).reply_to(&msg.id);
    if let Err(e) = bot.post_group_message(&msg.group_openid, &reply).await {
        error!(error = %e, "markdown: group send failed");
    }
}

async fn demo_markdown_c2c(bot: &Bot, msg: &C2cMessage) {
    let md = MarkdownPayload::raw(SAMPLE_MARKDOWN);
    let reply = OutgoingMessage::markdown(md).reply_to(&msg.id);
    if let Err(e) = bot.post_c2c_message(&msg.author.user_openid, &reply).await {
        error!(error = %e, "markdown: c2c send failed");
    }
}

async fn demo_keyboard_group(bot: &Bot, msg: &GroupMessage) {
    let md = MarkdownPayload::raw("点下方按钮试试：");
    let reply = OutgoingMessage::markdown(md)
        .with_keyboard(sample_keyboard())
        .reply_to(&msg.id);
    if let Err(e) = bot.post_group_message(&msg.group_openid, &reply).await {
        error!(error = %e, "keyboard: group send failed");
    }
}

async fn demo_keyboard_c2c(bot: &Bot, msg: &C2cMessage) {
    let md = MarkdownPayload::raw("点下方按钮试试：");
    let reply = OutgoingMessage::markdown(md)
        .with_keyboard(sample_keyboard())
        .reply_to(&msg.id);
    if let Err(e) = bot.post_c2c_message(&msg.author.user_openid, &reply).await {
        error!(error = %e, "keyboard: c2c send failed");
    }
}

async fn demo_embed_channel(bot: &Bot, msg: &ChannelMessage) {
    let reply = OutgoingChannelMessage::embed(sample_embed()).reply_to(&msg.id);
    if let Err(e) = bot.post_channel_message(&msg.channel_id, &reply).await {
        error!(error = %e, "embed: channel send failed");
    }
}

async fn demo_quote_channel(bot: &Bot, msg: &ChannelMessage) {
    let reply = OutgoingChannelMessage::text("引用刚才那条：")
        .quote(&msg.id, true)
        .reply_to(&msg.id);
    if let Err(e) = bot.post_channel_message(&msg.channel_id, &reply).await {
        error!(error = %e, "quote: channel send failed");
    }
}

async fn demo_reaction_channel(bot: &Bot, msg: &ChannelMessage) {
    if let Err(e) = bot
        .put_channel_reaction(&msg.channel_id, &msg.id, EmojiType::System, DEMO_EMOJI_ID)
        .await
    {
        return log_err("reaction: put", e);
    }
    info!(emoji_id = %DEMO_EMOJI_ID, "reaction: added");

    match bot
        .list_channel_reaction_users(
            &msg.channel_id,
            &msg.id,
            EmojiType::System,
            DEMO_EMOJI_ID,
            None,
        )
        .await
    {
        Ok(page) => info!(
            count = page.users.len(),
            is_end = page.is_end,
            "reaction: listed users"
        ),
        Err(e) => log_err("reaction: list", e),
    }

    sleep(RECALL_DELAY).await;
    if let Err(e) = bot
        .delete_channel_reaction(&msg.channel_id, &msg.id, EmojiType::System, DEMO_EMOJI_ID)
        .await
    {
        error!(error = %e, "reaction: delete failed");
    } else {
        info!("reaction: removed");
    }
}

async fn demo_dm_channel(bot: &Bot, msg: &ChannelMessage) {
    let session = match bot.create_dm(&msg.author.id, &msg.guild_id).await {
        Ok(s) => s,
        Err(e) => return log_err("dm: create", e),
    };
    info!(dm_guild = %session.guild_id, "dm: session created");

    let reply = OutgoingChannelMessage::text("这是一条来自 bot 的私信").reply_to(&msg.id);
    if let Err(e) = bot.post_dm_message(&session.guild_id, &reply).await {
        error!(error = %e, "dm: send failed");
    } else {
        info!("dm: sent");
    }
}

async fn demo_share_group(bot: &Bot, msg: &GroupMessage) {
    let url = match bot.generate_url_link(Some(SHARE_CALLBACK_DATA)).await {
        Ok(u) => u,
        Err(e) => return log_err("share: generate", e),
    };
    let reply = OutgoingMessage::text(format!("机器人添加链接：{url}")).reply_to(&msg.id);
    if let Err(e) = bot.post_group_message(&msg.group_openid, &reply).await {
        error!(error = %e, "share: group send failed");
    }
}

async fn demo_share_c2c(bot: &Bot, msg: &C2cMessage) {
    let url = match bot.generate_url_link(Some(SHARE_CALLBACK_DATA)).await {
        Ok(u) => u,
        Err(e) => return log_err("share: generate", e),
    };
    let reply = OutgoingMessage::text(format!("机器人添加链接：{url}")).reply_to(&msg.id);
    if let Err(e) = bot.post_c2c_message(&msg.author.user_openid, &reply).await {
        error!(error = %e, "share: c2c send failed");
    }
}

const SAMPLE_MARKDOWN: &str =
    "## qq-bot-rs markdown demo\n\n- **粗体**\n- *斜体*\n- [链接](https://github.com)\n";

fn sample_embed() -> EmbedPayload {
    EmbedPayload {
        title: Some("qq-bot-rs embed demo".into()),
        prompt: Some("通知栏会显示这一行".into()),
        thumbnail: Some(EmbedThumbnail {
            url: DEMO_IMAGE_URL.into(),
        }),
        fields: vec![
            EmbedField {
                name: "字段一".into(),
            },
            EmbedField {
                name: "字段二".into(),
            },
        ],
    }
}

fn sample_keyboard() -> KeyboardPayload {
    KeyboardPayload::inline(Keyboard {
        rows: vec![KeyboardRow {
            buttons: vec![
                Button {
                    id: "btn-link".into(),
                    render_data: RenderData {
                        label: "打开 GitHub".into(),
                        visited_label: "已访问".into(),
                        style: 1,
                    },
                    action: Action {
                        action_type: 0,
                        permission: Permission {
                            permission_type: 2,
                            ..Default::default()
                        },
                        data: "https://github.com".into(),
                        unsupport_tips: "客户端版本太低，请升级".into(),
                        reply: None,
                        enter: None,
                        anchor: None,
                    },
                },
                Button {
                    id: "btn-callback".into(),
                    render_data: RenderData {
                        label: "回调".into(),
                        visited_label: "点过".into(),
                        style: 0,
                    },
                    action: Action {
                        action_type: 1,
                        permission: Permission {
                            permission_type: 2,
                            ..Default::default()
                        },
                        data: "callback-payload-1".into(),
                        unsupport_tips: "客户端版本太低，请升级".into(),
                        reply: None,
                        enter: None,
                        anchor: None,
                    },
                },
            ],
        }],
    })
}

fn log_err(stage: &str, err: impl std::fmt::Display) {
    error!(stage, %err, "demo step failed");
}

#[tokio::main]
async fn main() -> Result<(), BotError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,qq_bot_rs=debug")),
        )
        .init();

    dotenvy::dotenv().ok();

    Client::from_env()?
        .intents(Intents::default_public())
        .handler(DemoBot)
        .run()
        .await
}
