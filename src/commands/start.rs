use std::{str::FromStr};
use serde::{Serialize, Deserialize};

use teloxide::payloads::LeaveChat;
use teloxide::prelude::*;
use teloxide::types::{InputFile, InlineKeyboardMarkup, InlineKeyboardButton, InlineKeyboardButtonKind};
use userdb::db::{UserID, RaffleDB, RegistrationStatus};

use crate::commands::admin::RaffleDescription;
use crate::commands::Context;
use crate::utils::*;

use super::dialogues::*;

#[derive(Serialize, Deserialize)]
pub struct StartData {
    pub referrer: Option<UserID>
}

impl FromStr for StartData {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StartData {
            referrer: match s.parse::<i64>() {
                Ok(num) => Some(num),
                Err(_) => None
            }
        })
    }
}

fn make_referral_link(bot_name: String, user_id: UserID) -> String {
    format!("https://t.me/{}?start={}", bot_name, user_id)
}

pub async fn start_cmd(
    referrer: Option<UserID>,
    cx: Context) -> TransitionOut<Dialogue> {

    let from_user = cx.update.from();
    if from_user.is_none() {
        return exit();
    }
    let user_id = from_user.unwrap().id;
    if is_admin(user_id) {
        // show admin keyboard
        cx.answer("todo show admin kb.")
        .await?;
        next(Dialogue::Begin(NoData))
    } else {
        let ongoing_raffle = {
            let raffle_db = crate::db_instance.lock().await;
            raffle_db.get_ongoing_raffle()
        };
        let ongoing_raffle = match ongoing_raffle {
            Ok(thing) => thing,
            Err(e) => {
                
                cx.answer(format!("A fatal error occurred.
{}
Please report this error to my manger.", e.to_string())).await?;
                return next(Dialogue::Begin(NoData));
            }
        };
        if ongoing_raffle.is_none() {
            cx.answer("Hello! At the moment there are no raffles running, so please wait for an announcment!").await?;
                            return next(Dialogue::Begin(NoData));
        }
        match referrer {
            Some(_) => {
                join_cmd(referrer, cx).await
            }
            None => {
                let raffle = ongoing_raffle.unwrap();
                let message_copy = serde_json::from_str::<RaffleDescription>(&raffle.raffle_description)
                    .expect("Failed to parse message from database")
                    .clone();
                    send_raffle_desc_into_chat(&cx.requester, message_copy, cx.chat_id()).await;
                cx.answer("In order to join the raffle please type /join.")
                .await?;
                next(Dialogue::AwaitingJoinChannel(AwaitingJoinChannelState{
                    referrer
                }))
            }
        }
    }
}

async fn clone_message_into_chat(bot: &AutoSend<Bot>, message: &Message, target_chat: i64) {
    let _ = match (message.text(), message.photo()) {
        (Some(content), _) => {
            bot.send_message(target_chat, content).await
        }
        (None, Some(images)) => {
            let req = bot.send_photo(target_chat, InputFile::FileId(images.get(0).unwrap().file_id.clone()));
            if message.caption().is_some() {
                req.caption(message.caption().unwrap()).await
            } else {
                req.await
            }
        }
        _ => {unreachable!()}
    };
}

async fn send_raffle_desc_into_chat(bot: &AutoSend<Bot>, message: RaffleDescription, target_chat: i64) {
    let _ = match message {
        RaffleDescription::Text(text_string) => { bot.send_message(target_chat, text_string).await },
        RaffleDescription::Photo {file_id, caption} => {
            let req = bot.send_photo(target_chat, InputFile::FileId(file_id));
            if caption.is_some() {
                req.caption(caption.unwrap()).await
            } else {
                req.send().await
            }
        }
    };
}

pub async fn join_cmd(
    referrer: Option<UserID>,
    cx: Context) -> TransitionOut<Dialogue> {
    let from_user = cx.update.from();
    if from_user.is_none() {
        return exit();
    }
    let user_id = from_user.unwrap().id;
    if is_admin(user_id) {
        // show admin keyboard
        cx.answer("You can't join the raffle as an admin, silly..")
        .await?;
        return next(Dialogue::Begin(NoData));
    }
    if !is_member_of_target_group(user_id, &cx.requester).await? {     
        cx.answer("Please join the @@@TARGET_CHANNEL_NAME@@@ chat before trying to join the raffle.").await?;
        return next(Dialogue::AwaitingJoinChannel(AwaitingJoinChannelState{
            referrer
        }));
    }

    let mut raffle_db = crate::db_instance.lock().await;
    if raffle_db.is_partecipant(user_id).unwrap() {
        cx.answer("You already belong in the raffle.").await?;
    } else {
        let result = raffle_db.register_partecipant(user_id, referrer);
        match result {
            Ok(RegistrationStatus::NotRegistered) => {
                panic!("This should not be reached");
            }
            Ok(RegistrationStatus::NoRaffleOngoing) => {
                cx.reply_to("Sorry, there are no ongoing raffles at the moment. Please try again later!").await?;
            }
            Err(e) => {
                log::error!("While attempting registration: {:?}", e);
            },
            _ => {
                let me = cx.requester.get_me().await?.user.username.expect("Could not fetch the username of this bot!");
                let referral = make_referral_link(me, user_id);
                cx.reply_to("Welcome to this raffle! You gained one point for joining, use /redeem to redeem additional codes and /points to see your points!
Below you will find a referral link you can share with other people, if they join you will get an additional point").await?;
                cx.answer(referral).await?;
            }
        }
        // TODO Notify referrer in case
    }
    next(Dialogue::Begin(NoData))
}

const YES: &str = "yes";

pub async fn leave_cmd(
    cx: Context) -> TransitionOut<Dialogue> {
    let from_user = cx.update.from();
    if from_user.is_none() {
        return exit();
    };
    let user_id = from_user.unwrap().id;
    if is_admin(user_id) {
        cx.answer("You can't leave the chat as an admin, silly!").await?;
        return next(Dialogue::Begin(NoData));
    }
    cx.answer("Do you really want to leave the raffle? Please type yes (lowercase!) or anything else to abort").await?;
    next(Dialogue::AwaitingLeaveAnswer(LeaveState))
}

#[derive(Serialize, Deserialize)]
pub struct LeaveState;

#[teloxide(subtransition)]
async fn leave_got_answer(
    state: LeaveState,
    cx: TransitionIn<AutoSend<Bot>>,
    _ans: String) -> TransitionOut<Dialogue> {
    let from_user = cx.update.from();
    if from_user.is_none() {
        cx.answer("Invalid answer.")
        .await?;
        return next(state);
    };
    let user_id = from_user.unwrap().id;
    match cx.update.text() {
        Some(YES) => {
            let remove_status = {
                let mut raffle_db = crate::db_instance.lock().await;
                raffle_db.remove_partecipant(user_id)
            };
            match remove_status {
                Ok(true) => {
                    cx.answer("Well, bye! We will miss you!
If you do decide to come back, remember that we will keep all your points.")
                    .await?;
                }
                Ok(false) => {
                    cx.answer("Sorry, to leave the raffle you must first join it with /join.")
                    .await?;
                }
                Err(e) => {
                    cx.answer(format!("Sorry, an error occurred: {}, pass this to my manager.", e.to_string()))
                    .await?;
                }
            }
            next(Dialogue::Begin(NoData))
        }
        _ => {
            cx.answer("Okay! Well you got me scared there.")
            .await?;
            next(Dialogue::Begin(NoData))
        }
    }
}