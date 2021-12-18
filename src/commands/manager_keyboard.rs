use super::Context;
use crate::utils::*;
use super::dialogues::*;

use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, InlineKeyboardButtonKind};
    
fn build_admin_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(
        vec![
            vec![InlineKeyboardButton::new("Test1", InlineKeyboardButtonKind::CallbackData("1".to_owned())), InlineKeyboardButton::new("2", InlineKeyboardButtonKind::CallbackData("None".to_owned()))],
            vec![InlineKeyboardButton::new("Test3", InlineKeyboardButtonKind::CallbackData("3".to_owned())), InlineKeyboardButton::new("4", InlineKeyboardButtonKind::CallbackData("None".to_owned()))]
        ]
    )
}

pub async fn send_manager_keyboard_command(ctx: Context) 
    -> TransitionOut<Dialogue> {
    let user_id = ctx.update.from();
    if user_id.is_none() {
        return exit();
    }
    let user_id = user_id.unwrap().id;
    let chat = &ctx.update.chat;

    if is_chat_with_manager(user_id, chat) {
        let markup = build_admin_keyboard();
        ctx.requester.send_message(chat.id, "Sure my lord.")
            .reply_markup(markup)
            .await?;
    } else {
        ctx.answer("heh you tried").await?;
    }
    
    exit()
}