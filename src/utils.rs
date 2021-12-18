use std::{io::BufReader, collections::HashSet};

use serde::Deserialize;
use teloxide::{types::Chat, Bot, prelude::Requester, adaptors::AutoSend, ApiError, RequestError};
use userdb::db::UserID;
use lazy_static::lazy_static;

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

pub fn is_chat_with_manager(user_id: UserID, chat: &Chat) -> bool {
    is_manager(user_id) && chat.is_private()
}

pub async fn get_user_tag(user_id: UserID, chat_id: i64, bot: &AutoSend<Bot>) -> Result<String, RequestError> {
    let user = bot.get_chat_member(chat_id, user_id).await?.user;
    Ok(match user.username {
        Some(username) => format!("@{}", username),
        None => format!("<a href=\"tg://user?id={}\">{}</a>", user.id, user.full_name()),
    })
}

pub async fn is_member_of_target_group(user_id: UserID, bot: &AutoSend<Bot>) -> Result<bool, RequestError> {
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