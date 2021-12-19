use std::{str::FromStr};
use serde::{Serialize, Deserialize};
use teloxide::{prelude::*, payloads::SendMessageSetters};
use teloxide::types::{InputFile, ParseMode};
use userdb::db::{UserID, RaffleDB, RegistrationStatus};

use crate::commands::admin::RaffleDescription;
use crate::commands::Context;
use crate::utils::*;

use super::{dialogues::*, RaffleBot};

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

    let user_id = match cx.update.from() {
        Some(user) => user.id,
        None => return next(Dialogue::Begin(NoData))
    };
    if is_admin(user_id) {
        // show admin keyboard
        cx.answer("Available commands for admins:
/startraffle to start a new raffle
/endraffle to end an ongoing raffle
/generatecode [usages=illimited, a number, once] to generate a redeemable code
")        .await?;
        next(Dialogue::Begin(NoData))
    } else {
        let ongoing_raffle = {
            let raffle_db = crate::DB_INSTANCE.lock().await;
            raffle_db.get_ongoing_raffle()
        };
        let ongoing_raffle = match ongoing_raffle {
            Ok(thing) => thing,
            Err(e) => {
                on_error(e, &cx.update, &cx.requester, "on start: get ongoing raffle").await;
                return next(Dialogue::Begin(NoData));
            }
        };
        match ongoing_raffle {
            None => {
                cx.answer("Hello! At the moment there are no raffles running, so please wait for an announcment!").await?;
                next(Dialogue::Begin(NoData))
            }
            Some(raffle) => {
                match referrer {
                    Some(_) => {
                        join_cmd(referrer, cx).await
                    }
                    None => {
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
    }
}
/* This code is kept because atm serde_json can't deserialize Messages, should it be resolved send_raffle_desc_into_chat is going to be replaced with this
async fn clone_message_into_chat(bot: &AutoSend<Bot>, message: &Message, target_chat: i64) {
    let _ = match (message.text(), message.photo()) {
        (Some(content), _) => {
            bot.send_message(target_chat, content).await
        }
        (None, Some(images)) => {
            let req = bot.send_photo(target_chat,
                                                                        InputFile::FileId(
                                                                            images.get(0)
                                                                            .expect("Got intot the Some(photos) arm but no photos were found")
                                                                            .file_id.clone()
                                                                        )
                                                                );
            match message.caption() {
                Some(caption) => req.caption(caption),
                None => req
            }.await
        }
        _ => {unreachable!()}
    };
}
*/
async fn send_raffle_desc_into_chat(bot: &RaffleBot, message: RaffleDescription, target_chat: i64) {
    let _ = match message {
        RaffleDescription::Text(text_string) => { bot.send_message(target_chat, text_string).await },
        RaffleDescription::Photo {file_id, caption} => {
            let req = bot.send_photo(target_chat, InputFile::FileId(file_id));
            match caption {
                Some(caption) => req.caption(caption),
                None => req
            }.await
        }
    };
}

pub async fn join_cmd(
    referrer: Option<UserID>,
    cx: Context) -> TransitionOut<Dialogue> {
    let user_id = match cx.update.from() {
        Some(u) => u.id,
        None => { 
            return next(Dialogue::Begin(NoData));
        }
    };
    if is_admin(user_id) {
        // show admin keyboard
        cx.answer("You can't join the raffle as an admin, silly.
Type /start to see what you can do as an admin")
        .await?;
        return next(Dialogue::Begin(NoData));
    }
    if !is_member_of_target_group(user_id, &cx.requester).await? {
        let join_link = generate_invite_for_target_chat(&cx.requester).await?;
        cx.answer(format!("Please join {} before trying to join the raffle.", join_link))
            .parse_mode(ParseMode::Html)
            .await?;
        return next(Dialogue::AwaitingJoinChannel(AwaitingJoinChannelState{
            referrer
        }));
    }
    let is_partecipant = {
        let raffle_db = crate::DB_INSTANCE.lock().await;
        raffle_db.is_partecipant(user_id)
    };
    let is_partecipant = match is_partecipant {
            Ok(result) => result, 
            Err(e) => {
                on_error(e, &cx.update, &cx.requester, "on registration").await;
                return next(Dialogue::Begin(NoData));
            }
        };
    if is_partecipant {
        cx.answer("You already belong in the raffle.
As a partecipant, you can issue the following commands:
/points to see how many points you have
/redeem CODE to redeem a code 
/leave to leave the raffle        
").await?;
    } else {
        let result = {
            let mut raffle_db = crate::DB_INSTANCE.lock().await;
            raffle_db.register_partecipant(user_id, referrer)
        };
        match result {
            Ok(RegistrationStatus::NotRegistered) => {
                panic!("This should not be reached");
            }
            Ok(RegistrationStatus::NoRaffleOngoing) => {
                cx.reply_to("Sorry, there are no ongoing raffles at the moment. Please try again later!").await?;
            }
            Err(e) => {
                on_error(e, &cx.update, &cx.requester, "on registration").await;
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
    let user_id = match cx.update.from() {
        Some(u) => u.id,
        None => { 
            return next(Dialogue::Begin(NoData));
        }
    };
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
    _state: LeaveState,
    cx: TransitionIn<RaffleBot>,
    _ans: String) -> TransitionOut<Dialogue> {
    let user_id = match cx.update.from() {
        Some(u) => u.id,
        None => { 
            return next(Dialogue::Begin(NoData));
        }
    };
    match cx.update.text() {
        Some(YES) => {
            let remove_status = {
                let mut raffle_db = crate::DB_INSTANCE.lock().await;
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
                    on_error(e, &cx.update, &cx.requester, "on leave").await;
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