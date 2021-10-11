use codec::{Decode, Encode};
use separator::FixedPlaceSeparatable;
use sp_core::crypto::{AccountId32, Ss58AddressFormat};
use std::collections::BTreeMap;
use substate::{
    storage_key,
    utils::{accountid_to_address, address_to_accountid},
    StorageHasher,
};
use yew::{
    prelude::*,
    services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask},
};
use yew_router::prelude::RouteService;

enum Msg {
    WS(Result<String, anyhow::Error>),
    Wss(WebSocketStatus),
}

enum WsMsg {
    None,
    ErasStakers,
    Validators,
    CurrentSession,
    AuthoredBlocks,
    ActiveEraInfo,
    ErasRewardPoints,
    ErasRewardPointsCurrent,
    ErasValidatorReward,
}

struct Model {
    ws: WebSocketTask,
    total: u128,
    own: u128,
    nominators: Vec<IndividualExposure>,
    current_msg: WsMsg,
    current_era: u32,
    current_session: u32,
    account: AccountId32,
    format: Ss58AddressFormat,
    in_set: bool,
    blocks: u32,
    era_epoch: (u64, f64),
    session_epoch: (u64, f64),
    points: (u32, u32),
    last_reward: (u128, u128),
    current_reward: u128,
    _up: bool,
    home: bool,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let route_service: RouteService<()> = RouteService::new();
        let route = route_service.get_route();
        
        yew::services::ConsoleService::info(route.as_str());

        yew::services::ConsoleService::info(route.as_str());

        yew::services::ConsoleService::info(
            format!("{:?}", route.rsplit('/').collect::<Vec<&str>>()).as_str(),
        );

        yew::services::ConsoleService::info(route.rsplit('/').collect::<Vec<&str>>()[0]);

        let final_path = route.rsplit('/').collect::<Vec<&str>>()[0];

        let (account, format) = address_to_accountid(if final_path.is_empty() {
            "esqyGXvN7eezFoGtXAiLvXNnai2KFWkt7VfWwywHNBdwb8dUh"
        } else {
            final_path
        })
        .unwrap();
        Self {
            total: 0u128,
            own: 0u128,
            nominators: vec![],
            ws: WebSocketService::connect_text(
                "wss://mainnet.polkadex.trade",
                link.callback(Msg::WS),
                link.callback(Msg::Wss),
            )
            .unwrap(),
            current_msg: WsMsg::None,
            current_era: 0u32,
            current_session: 0u32,
            account,
            format,
            home: final_path.is_empty(),
            in_set: false,
            blocks: 0u32,
            era_epoch: (0u64, 0f64),
            session_epoch: (0u64, 0f64),
            points: (0u32, 0u32),
            last_reward: (0u128, 0u128),
            current_reward: 0u128,
            _up: false,
        }
    }
    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        if let WsMsg::None = self.current_msg {
            let storage_key = storage_key!("Session", "CurrentIndex");

            self.current_msg = WsMsg::CurrentSession;
            self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
        }

        match msg {
            Msg::WS(msg) => {
                let msg = msg.unwrap();

                let hexstr = msg
                    .trim_matches('\"')
                    .to_string()
                    .trim_start_matches("0x")
                    .to_string()
                    .replace("{\"jsonrpc\":\"2.0\",\"result\":\"0x", "")
                    .replace("\",\"id\":1}", "");

                let slice = hex::decode(&hexstr).unwrap();
                let mut slice = slice.as_slice();

                match self.current_msg {
                    WsMsg::ErasStakers => {
                        let era = Exposure::decode(&mut slice).unwrap();
                        self.total = era.total;
                        self.own = era.own;
                        self.nominators = era.others;

                        let storage_key = storage_key!("Session", "Validators");

                        self.current_msg = WsMsg::Validators;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                    WsMsg::None => {}
                    WsMsg::ActiveEraInfo => {
                        let era_info: ActiveEraInfo = ActiveEraInfo::decode(&mut slice).unwrap();

                        self.current_era = era_info.index;

                        let epoch = era_info.start.unwrap();

                        let end = epoch + 86400000;
                        let now = instant::now() as u64;
                        let mut i = 1;
                        while (epoch + (i * 14400000)) < now {
                            i += 1;
                        }

                        let end_session = epoch + (i * 14400000);
                        let epoch_session = end_session - 14400000;

                        self.era_epoch.0 = end - now;
                        self.session_epoch.0 = end_session - now;

                        self.session_epoch.1 = ((now as f64 - epoch_session as f64)
                            / (end_session as f64 - epoch_session as f64)
                            * 100f64) as f64;

                        self.era_epoch.1 = ((now as f64 - epoch as f64)
                            / (end as f64 - epoch as f64)
                            * 100f64) as f64;

                        let storage_key = storage_key!(
                            "ImOnline",
                            "AuthoredBlocks",
                            (StorageHasher::Twox64Concat, self.current_session),
                            (StorageHasher::Twox64Concat, self.account)
                        );

                        self.current_msg = WsMsg::AuthoredBlocks;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                    WsMsg::ErasValidatorReward => {
                        let total = u128::decode(&mut slice).unwrap();

                        self.last_reward.1 = total;

                        self.last_reward.0 =
                            (self.points.0 as u128 * total) / self.points.1 as u128;

                        let storage_key = storage_key!(
                            "Staking",
                            "ErasRewardPoints",
                            (StorageHasher::Twox64Concat, self.current_era)
                        );

                        self.current_msg = WsMsg::ErasRewardPointsCurrent;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                    WsMsg::ErasRewardPoints => {
                        let context = EraRewardPoints::decode(&mut slice).unwrap();

                        self.points.1 = context.total;
                        self.points.0 = *context.individual.get(&self.account).unwrap();

                        let storage_key = storage_key!(
                            "Staking",
                            "ErasValidatorReward",
                            (StorageHasher::Twox64Concat, self.current_era - 1)
                        );

                        self.current_msg = WsMsg::ErasValidatorReward;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                    WsMsg::ErasRewardPointsCurrent => {
                        let context = EraRewardPoints::decode(&mut slice).unwrap();

                        self.current_reward = (*context.individual.get(&self.account).unwrap()
                            as u128
                            * self.last_reward.1)
                            / context.total as u128;
                    }
                    WsMsg::Validators => {
                        self.in_set = Vec::decode(&mut slice)
                            .unwrap()
                            .into_iter()
                            .any(|validator: AccountId32| validator == self.account);

                        let storage_key = storage_key!(
                            "Staking",
                            "ErasRewardPoints",
                            (StorageHasher::Twox64Concat, self.current_era - 1)
                        );

                        self.current_msg = WsMsg::ErasRewardPoints;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                    WsMsg::AuthoredBlocks => {
                        self.blocks = u32::decode(&mut slice).unwrap();

                        let storage_key = storage_key!(
                            "Staking",
                            "ErasStakers",
                            (StorageHasher::Twox64Concat, self.current_era),
                            (StorageHasher::Twox64Concat, self.account)
                        );

                        self.current_msg = WsMsg::ErasStakers;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                    WsMsg::CurrentSession => {
                        self.current_session = u32::decode(&mut slice).unwrap();

                        let storage_key = storage_key!("Staking", "ActiveEra");

                        self.current_msg = WsMsg::ActiveEraInfo;
                        self.ws.send(Ok(format!("{{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"state_getStorage\", \"params\":[\"{}\"]}}", storage_key)));
                    }
                }
                true
            }
            Msg::Wss(_) => false,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <section class="section" style="height: 100vh; padding-top: 1rem;">
                <div style="display: flex;flex-direction: column;flex-wrap: wrap-reverse;width: 100%;height: 5%;margin-bottom: 1.5rem;">

                // TODO: Indicator of node being up
                //<div style="margin-right: 0;width: 2%;text-align: center; height: 100%; top: 50%;" class={format!("tile notification is-{} is-ancestor", if self.up { "success" } else { "danger" })}>
                //<p style="align-self: center;">{ if self.up { "👍" } else { "👎" } }</p>
            //</div>
                <div></div>

                <h1 style="" class="title is-1 has-text-centered">{ if self.home { "Gabe's Validator" } else { "Validator Dashboard" } }</h1>
                <div></div>
                </div>
                <div class="tile is-ancestor" style="height: 95%;">
                <div class="tile is-vertical is-4">
                <div class="tile">
                <div class="tile is-parent">
                <article class="tile is-child notification is-info">
                <p class="subtitle">{ "Current Era" }</p>
                <p class="title is-1 has-text-centered" style="font-size: 4rem;"> { &self.current_era } </p>
                <br/>
                <br/>
                <p class="subtitle">{ "Current Session" }</p>
                <p class="title is-1 has-text-centered" style="font-size: 4rem;"> { &self.current_session } </p>
                </article>
                </div>
                <div class="tile is-parent is-vertical">
                <article class="tile is-child notification is-warning">
                <p class="subtitle"> { "In current set?" } </p>
                <p class="title is-1 has-text-centered"> { if self.in_set {"Yeah!"} else {"Nah..."} } </p>
                </article>
                <article class="tile is-child notification is-primary">
                <p class="subtitle">{ "Produced blocks this session" }</p>
                <div class="content">
                <p class="title is-1 has-text-centered"> { &self.blocks } </p>
                </div>
                </article>
                </div>
                </div>
                <div class="tile is-parent">
                <article class="tile is-child notification is-danger">
                <div class="content">
                <p class="title is-3 has-text-centered" style="margin-bottom: 0.5em;">{ "Era" }</p>
                <p class="subtitle" style="position: absolute; right: 77%; top: 45%;">{ format!("{}:{}", ((self.era_epoch.0 as f64 % 86400000f64) / 3600000f64).floor(), ((self.era_epoch.0 as f64 % 3600000f64) / 60000f64).floor()) }</p>
                <p class="subtitle" style="position: absolute; left: 75%; top: 45%">{ "24hrs" }</p>
                <div class="container0">
                          <div class="gauge-a0"></div>
                <div class="gauge-b0"></div>
                <div class="gauge-c0 has-background-primary" style={format!("transform: rotate({}deg);", ((180f64 * self.era_epoch.1) / 100f64))}></div>
                          <div class="gauge-data0">
                <h1 class="title has-text-info-light">{ format!("{:.0}%", self.era_epoch.1) }</h1>
                </div>
                </div>

                <div class="container1">
                          <div class="gauge-a1"></div>
                <div class="gauge-b1"></div>
                <div class="gauge-c1 has-background-primary" style={format!("transform: rotate({}deg);", ((180f64 * self.session_epoch.1) / 100f64))}></div>
                          <div class="gauge-data1">
                <h1 class="title has-text-info-light" style="transform: scaleY(-1)">{ format!("{:.0}%", self.session_epoch.1) }</h1>
                </div>
                </div>
                <p class="subtitle" style="position: absolute; right: 77%; bottom: 30%;">{ format!("{}:{}", ((self.session_epoch.0 as f64 % 86400000f64) / 3600000f64).floor(), ((self.session_epoch.0 as f64 % 3600000f64) / 60000f64).floor()) }</p>
                <p class="subtitle" style="position: absolute; left: 75%; bottom: 30%;">{ "4hrs" }</p>
                <p class="title is-4 has-text-centered" style="margin-top: 0.5em;">{ "Session" }</p>
                </div>
                </article>
                </div>
                </div>



                <div class="tile is-vertical is-4">
                <div class="tile">
                <div class="tile is-parent">
                <article class="tile is-child notification is-warning">
                <p class="title is-3"> { "Own stash" } </p>
                <p class="title is-1 has-text-centered">{ format!("{} PDEX", ((self.own as f64) / 1000000000000f64).separated_string_with_fixed_place(2))  }</p>
                <p class="title is-3"> { "Total stake" } </p>
                <p class="title is-1 has-text-centered">{ format!("{} PDEX", ((self.total as f64) / 1000000000000f64).separated_string_with_fixed_place(2))  }</p>
                </article>
                </div>
                </div>
                <div class="tile is-parent">
                <article class="tile is-child notification is-info">
                <p class="title is-3"> { "Last Reward" } </p>
                <p class="title is-1 has-text-centered">{ format!("{} PDEX", ((self.last_reward.0 as f64) / 1000000000000f64).separated_string_with_fixed_place(2))  }</p>
                <p class="title is-3"> { "Current Reward Estimate" } </p>
                <p class="title is-1 has-text-centered">{ format!("{} PDEX", ((self.current_reward as f64) / 1000000000000f64).separated_string_with_fixed_place(2))  }</p>
                </article>
                </div>
                </div>
                <div class="tile is-parent">
                <article class="tile is-child notification is-primary" style="overflow: auto;">
                <div class="content">
                <p class="title">{ "Nominators" }</p>
                <p class="subtitle"></p>
                <div class="content" style="font-size: 14.7; overflow-y: auto;">
            {if !self.nominators.is_empty() {self.nominators.iter().map(|nominator| html!{
                <div style="padding: 0.5em;">
                    <p>{format!("Address: {}", accountid_to_address(nominator.who.clone(), self.format))}
                <br/>
                {format!("Amount: {:.2} PDEX", (nominator.value as f64) / 1000000000000f64)}</p>
                    </div>
            }).collect()} else { html!{<p>{"None"}</p>}}}
            </div>
                </div>
                </article>
                </div>
                </div>
                </section>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode)]
pub struct IndividualExposure {
    /// The stash account of the nominator in question.
    pub who: AccountId32,
    /// Amount of funds exposed.
    #[codec(compact)]
    pub value: u128,
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default)]
pub struct Exposure {
    /// The total balance backing this validator.
    #[codec(compact)]
    pub total: u128,
    /// The validator's own stash that is exposed.
    #[codec(compact)]
    pub own: u128,
    /// The portions of nominators stashes that are exposed.
    pub others: Vec<IndividualExposure>,
}

#[derive(Encode, Decode)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: u32,
    /// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    start: Option<u64>,
}

#[derive(PartialEq, Encode, Decode, Default)]
pub struct EraRewardPoints {
    /// Total number of points. Equals the sum of reward points for each validator.
    total: u32,
    /// The reward points earned by a given validator.
    individual: BTreeMap<AccountId32, u32>,
}
