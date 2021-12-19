use std::{io::BufReader, collections::HashSet, error::Error};
use async_mutex::Mutex;

use serde::Deserialize;
use teloxide::{types::{Chat, Message, ChatKind, ChatPublic}, prelude::Requester, ApiError, RequestError};
use userdb::db::UserID;
use lazy_static::lazy_static;

use crate::commands::RaffleBot;

#[derive(Deserialize)]
struct Config {
    manager: UserID,
    target_chat: i64,
    admin_users: HashSet<i64>
}

fn load_config(file: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file)?;
    let reader = BufReader::new(file);
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}
lazy_static! {
    static ref CONFIG : Config = load_config("config.json").expect("Failed to load config file");
}

pub fn target_chat() -> i64 {
    CONFIG.target_chat
}


pub fn is_admin(user_id: UserID) -> bool {
    CONFIG.admin_users.contains(&user_id) || is_manager(user_id)
}

pub fn is_manager(user_id: UserID) -> bool {
    user_id == CONFIG.manager
}

pub async fn get_target_chat(bot: &RaffleBot) -> Result<Chat,RequestError> {
    lazy_static! {
        static ref CHAT: Mutex<Vec<Chat>> = Mutex::new(vec![]);
    };
    let mut chat_mutex = CHAT.lock().await;
    let cached_chat = chat_mutex.get(0);
    Ok(match cached_chat {
        Some(chat) => chat.clone(),
        None => {
            let chat_from_bot = bot.get_chat(target_chat()).await?;
            chat_mutex.push(chat_from_bot.clone());
            chat_from_bot
        }
    })
}

pub async fn generate_invite_for_target_chat(bot: &RaffleBot) -> Result<String, RequestError> {
    let chat = get_target_chat(&bot).await?;
    let invite_link = match chat.invite_link() {
        Some(link) => link.to_owned(),
        None => format!("https://t.me/{}", chat.id)
    };
    let chat_fullname = match chat.kind {
        ChatKind::Public(ChatPublic{title, .. }) => title.unwrap_or("A chat without a title?".to_owned()),
        ChatKind::Private(_) => format!("https://t.me/user?id={}", chat.id)
    };
    Ok(format!("<a href=\"{0}\">{1}</a>", invite_link, chat_fullname))
}

pub fn is_chat_with_manager(user_id: UserID, chat: &Chat) -> bool {
    is_manager(user_id) && chat.is_private()
}

pub async fn get_user_tag(user_id: UserID, chat_id: i64, bot: &RaffleBot) -> Result<String, RequestError> {
    let user = bot.get_chat_member(chat_id, user_id).await?.user;
    Ok(match user.username {
        Some(username) => format!("@{}", username),
        None => format!("<a href=\"tg://user?id={}\">{}</a>", user.id, user.full_name()),
    })
}

pub async fn is_member_of_target_group(user_id: UserID, bot: &RaffleBot) -> Result<bool, RequestError> {
    match bot.get_chat_member(CONFIG.target_chat, user_id)
        .await
    {
        Ok(mb) => {
            Ok(
                match mb.kind {
                    teloxide::types::ChatMemberKind::Owner(_) | 
                    teloxide::types::ChatMemberKind::Administrator(_) |
                    teloxide::types::ChatMemberKind::Restricted(_) |
                    teloxide::types::ChatMemberKind::Member => true,
                    _ => false
                }
            )
        },
        Err(e) => match &e {
            RequestError::ApiError { kind, .. } => {
                match &kind {
                    ApiError::UserNotFound => Ok(false),
                    _ => Err(e)
                }
            },
            _ => Err(e)
        }
    }
}

pub async fn on_error<'a>(err: Box<dyn Error + Send + Sync + 'a>, msg: &Message, bot: &RaffleBot, user_err: &str) {
    // Inform the user that an error occurred, ONLY IN PRIVATE CHAT (to avoid possible spamming)
    let chat = &msg.chat;
    if chat.is_private() {
        let _ = bot.send_message(chat.id, "Sorry! While processing your message an error has occurred, i'm signaling it to the bot manager.")
        .await;
    }
    // Try to signal the error at the manager
    let error_message = format!("
<b>An error occurred: {}</b>

Error details:
{:?}

Message that caused the error:
{:?}

", user_err, msg, err);
    let _ = bot.send_message(CONFIG.manager, &error_message).await;
    log::error!("
---- ERROR -------
{}
---- ERROR END ---
", error_message);
}