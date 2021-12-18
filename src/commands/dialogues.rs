use serde::{Serialize, Deserialize};
use teloxide::{
    prelude::*,
    macros::Transition
};
use userdb::db::UserID;
use crate::{utils::*, commands::RaffleBot};
use crate::commands::start::*;

#[derive(Transition, Serialize, Deserialize, derive_more::From)]
pub enum Dialogue {
    Begin(NoData),
    AwaitingJoinChannel(AwaitingJoinChannelState),
    Registered(RegistrationState),
    AwaitRaffleTitle(AwaitingRaffleTitleState),
    AwaitingRaffleMessage(AwaitingRaffleMessageState),
    AwaitingLeaveAnswer(LeaveState)
}

#[derive(Serialize, Deserialize)]
pub struct NoData;

#[derive(Serialize, Deserialize)]
pub struct RegistrationState;

#[derive(Serialize, Deserialize)]
pub struct AwaitingRaffleTitleState;


#[derive(Serialize, Deserialize)]
pub struct AwaitingRaffleMessageState {
    pub title: String
}

#[derive(Serialize, Deserialize)]
pub struct AwaitingJoinChannelState {
    pub referrer: Option<UserID>
}

impl Default for Dialogue {
    fn default() -> Self {
        Dialogue::Begin(NoData)
    }
}

#[teloxide(subtransition)]
async fn no_data(
    state: NoData,
    cx: TransitionIn<RaffleBot>,
    _ans: String
) -> TransitionOut<Dialogue> {
    cx.answer("Is this some text?").await?;
    next(state)
}

#[teloxide(subtransition)]
async fn registered(
    state: RegistrationState,
    cx: TransitionIn<RaffleBot>,
    _ans: String
) -> TransitionOut<Dialogue> {
    cx.answer("You are in the current raffle, please use the control keyboard, if you can't access it type /keyboard").await?;
    next(state)
}

#[teloxide(subtransition)]
async fn await_join_channel_msg(
    state: AwaitingJoinChannelState,
    cx: TransitionIn<RaffleBot>,
    _ans: String
) -> TransitionOut<Dialogue> {
    let user_id = 0;
    if !is_member_of_target_group(user_id, &cx.requester).await? {
        cx.answer("Sorry, but you must be a member of @@@CHAT_ID@@@ to join the raffle")
            .await?;
        next(state)
    } else {
        cx.answer("All right! Welcome to the raffle!").await?;
        next(Dialogue::Registered(RegistrationState))
    }
}