//! Telegram routes and asynchronous screen lifecycle, with Grammy as an opaque host SDK.
use super::{
    bridge::*,
    server_runtime::{engine, log},
};
use js_sys::{Array, Map, Promise, Reflect, RegExp, Symbol};
use std::rc::Rc;
use wasm_bindgen::{UnwrapThrowExt, prelude::*};
use wasm_bindgen_futures::{JsFuture, future_to_promise};

async fn run(object: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    JsFuture::from(Promise::resolve(&try_call(object, method, args)?)).await
}
fn view(method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    engine(&format!("server.view.{method}"), args)
}
fn keyboard(rows: &JsValue) -> JsValue {
    let output = Array::new();
    for row in values(rows) {
        let buttons = Array::new();
        for button in values(&row) {
            let url = get(&button, "url");
            let key = if present(&url) && !string(&url).is_empty() {
                "url"
            } else {
                "callback_data"
            };
            buttons.push(&object(&[
                ("text", get(&button, "text")),
                (
                    key,
                    if key == "url" {
                        url
                    } else {
                        get(&button, "data")
                    },
                ),
            ]));
        }
        output.push(&buttons);
    }
    object(&[("inline_keyboard", output.into())])
}
fn html() -> JsValue {
    object(&[("parse_mode", "HTML".into())])
}
fn markup(rows: &JsValue) -> JsValue {
    object(&[("reply_markup", keyboard(rows))])
}
fn html_buttons(rows: &JsValue) -> JsValue {
    let options = html();
    set(&options, "reply_markup", &keyboard(rows));
    options
}
fn empty_buttons() -> JsValue {
    markup(&Array::new().into())
}
fn instance_of(value: &JsValue, constructor: &JsValue) -> bool {
    if !present(constructor) {
        return false;
    }
    Reflect::get(constructor, &Symbol::has_instance().into())
        .and_then(|method| apply(&method, constructor, &[value.clone()]))
        .map(|v| boolean(&v))
        .unwrap_or(false)
}
fn lead(mode: &str) -> &'static str {
    if mode == "bot" {
        "<b>Бой с ботом.</b>"
    } else {
        "<b>Дуэль с другом.</b> Я дам ссылку, которую надо ему переслать."
    }
}
fn numeric(value: &JsValue) -> JsValue {
    if value.as_f64().is_some() {
        value.clone()
    } else {
        string(value).parse::<f64>().unwrap_or(f64::NAN).into()
    }
}

// Only a WeakRef crosses from Rust back into Grammy's callback graph. The JS
// callbacks retain their context as a normal property, so the host GC can trace
// and release bot -> callback -> context -> bot cycles when the bot is discarded.
struct Telegram {
    context: JsValue,
    screens: Map,
    text: JsValue,
}
impl Telegram {
    fn context(&self) -> JsValue {
        call(&self.context, "deref", &[])
    }
    fn bot(&self) -> JsValue {
        get(&self.context(), "bot")
    }
    fn duels(&self) -> JsValue {
        get(&self.context(), "duels")
    }
    fn operators(&self) -> JsValue {
        get(&self.context(), "operators")
    }
    fn host(&self) -> JsValue {
        get(&self.context(), "host")
    }
    fn not_modified(&self, error: &JsValue) -> bool {
        instance_of(error, &get(&self.host(), "GrammyError"))
            && string(&get(error, "description")).contains("message is not modified")
    }
    fn who(&self, ctx: &JsValue) -> Result<JsValue, JsValue> {
        let from = get(ctx, "from");
        if !present(&from) || string(&get(&get(ctx, "chat"), "type")) != "private" {
            return Ok(JsValue::NULL);
        }
        let id = get(&from, "id");
        let id = id
            .as_f64()
            .map(|n| n.to_string())
            .unwrap_or_else(|| string(&id));
        let mut name = string(&get(&from, "first_name"));
        if name.is_empty() {
            name = string(&get(&from, "username"));
        }
        if name.is_empty() {
            name = "Оператор".into();
        }
        let op = try_call(&self.operators(), "byTelegram", &[id.clone().into()])?;
        Ok(object(&[
            ("id", id.into()),
            ("name", name.into()),
            ("tag", get(&op, "tag")),
        ]))
    }
    async fn needs_contract(
        &self,
        ctx: &JsValue,
        me: &JsValue,
        next: &str,
    ) -> Result<bool, JsValue> {
        if present(&get(me, "tag")) && !string(&get(me, "tag")).is_empty() {
            return Ok(false);
        }
        if present(&get(ctx, "callbackQuery")) {
            run(ctx, "answerCallbackQuery", &[]).await?;
        }
        run(
            ctx,
            "reply",
            &[
                get(&self.text, "CONTRACT"),
                html_buttons(&view("contractButtons", &[next.into()])?),
            ],
        )
        .await?;
        Ok(true)
    }
    async fn welcome(&self, ctx: &JsValue) -> Result<JsValue, JsValue> {
        run(
            ctx,
            "reply",
            &[
                get(&self.text, "WELCOME"),
                html_buttons(&view("menuButtons", &[])?),
            ],
        )
        .await
    }
    async fn ask_weapon(&self, ctx: &JsValue, mode: &str, lead: &str) -> Result<JsValue, JsValue> {
        let text = format!("{lead}\n\nВыбери оружие (в скобках дистанция, где оно сильнее всего):");
        run(
            ctx,
            "reply",
            &[
                text.into(),
                html_buttons(&view("weaponPicker", &[mode.into(), false.into()])?),
            ],
        )
        .await
    }
    async fn open_invite(
        &self,
        ctx: &JsValue,
        me: &JsValue,
        code: &str,
    ) -> Result<JsValue, JsValue> {
        let invite = try_call(&self.duels(), "inviteInfo", &[code.into()])?;
        if !present(&invite) {
            return run(
                ctx,
                "reply",
                &["Приглашение не найдено или устарело. Попроси друга прислать новое.".into()],
            )
            .await;
        }
        let host = get(&invite, "host");
        if get(&host, "id") == get(me, "id") {
            return run(
                ctx,
                "reply",
                &["Это твоё приглашение. Перешли ссылку другу.".into()],
            )
            .await;
        }
        self.ask_weapon(
            ctx,
            &format!("join:{}", string(&get(&invite, "code"))),
            &format!(
                "{} вызывает тебя на дуэль.",
                string(&view("nameWithTag", &[host])?)
            ),
        )
        .await
    }
    async fn show_turn(&self, duel: &JsValue, side: u32, prefix: &str) -> Result<(), JsValue> {
        let player = at(&get(duel, "players"), side as usize);
        let screen = view("turnScreen", &[duel.clone(), side.into()])?;
        let message = run(
            &get(&self.bot(), "api"),
            "sendMessage",
            &[
                numeric(&get(&player, "id")),
                format!("{prefix}{}", string(&get(&screen, "text"))).into(),
                html_buttons(&get(&screen, "buttons")),
            ],
        )
        .await?;
        self.screens.set(
            &format!("{}:{side}", string(&get(duel, "id"))).into(),
            &get(&message, "message_id"),
        );
        Ok(())
    }
    async fn retire(&self, duel: &JsValue, side: u32) {
        let key: JsValue = format!("{}:{side}", string(&get(duel, "id"))).into();
        let id = self.screens.get(&key);
        self.screens.delete(&key);
        if !present(&id) {
            return;
        }
        let player = at(&get(duel, "players"), side as usize);
        if let Err(error) = run(
            &get(&self.bot(), "api"),
            "editMessageReplyMarkup",
            &[numeric(&get(&player, "id")), id, empty_buttons()],
        )
        .await
        {
            if !self.not_modified(&error) {
                log("warn", "could not retire a turn screen", &error);
            }
        }
    }
    async fn player_event(&self, event: &JsValue, side: u32) -> Result<(), JsValue> {
        let duel = get(event, "duel");
        if string(&get(event, "type")) == "start" {
            let foe = at(&get(&duel, "players"), (1 - side) as usize);
            let name = string(&view("nameWithTag", &[foe.clone()])?);
            let intro = if boolean(&get(&foe, "bot")) {
                format!("Против тебя бот {name}.\n\n")
            } else {
                format!("Дуэль с {name} началась. Приказы тайные.\n\n")
            };
            return self.show_turn(&duel, side, &intro).await;
        }
        self.retire(&duel, side).await;
        let report = format!(
            "{}\n\n",
            string(&view(
                "turnReport",
                &[duel.clone(), side.into(), get(event, "report")]
            )?)
        );
        if present(&get(&get(&duel, "match"), "result")) {
            let screen = view("finalScreen", &[duel.clone(), side.into()])?;
            let player = at(&get(&duel, "players"), side as usize);
            run(
                &get(&self.bot(), "api"),
                "sendMessage",
                &[
                    numeric(&get(&player, "id")),
                    format!("{report}{}", string(&get(&screen, "text"))).into(),
                    html_buttons(&get(&screen, "buttons")),
                ],
            )
            .await?;
            Ok(())
        } else {
            self.show_turn(&duel, side, &report).await
        }
    }
    async fn on_event(&self, event: JsValue) -> Result<JsValue, JsValue> {
        for side in [0, 1] {
            let player = at(&get(&get(&event, "duel"), "players"), side as usize);
            if boolean(&get(&player, "bot")) {
                continue;
            }
            if let Err(error) = self.player_event(&event, side).await {
                log(
                    "error",
                    &format!("could not message player {}", string(&get(&player, "id"))),
                    &error,
                );
            }
        }
        Ok(JsValue::UNDEFINED)
    }
    async fn route(&self, route: &str, ctx: JsValue) -> Result<JsValue, JsValue> {
        match route {
            "rules" => return run(&ctx, "reply", &[get(&self.text, "RULES"), html()]).await,
            "help" => return self.welcome(&ctx).await,
            "stale" => {
                return run(
                    &ctx,
                    "answerCallbackQuery",
                    &[object(&[("text", "Эта кнопка устарела".into())])],
                )
                .await;
            }
            "weapon-kit" => {
                let matches = get(&ctx, "match");
                let veteran = string(&at(&matches, 2)) == "v";
                let answer = if veteran {
                    object(&[
                        ("text", get(&self.text, "VETERAN_NOTE")),
                        ("show_alert", true.into()),
                    ])
                } else {
                    JsValue::UNDEFINED
                };
                run(&ctx, "answerCallbackQuery", &[answer]).await?;
                let _ = run(
                    &ctx,
                    "editMessageReplyMarkup",
                    &[markup(&view(
                        "weaponPicker",
                        &[at(&matches, 1), veteran.into()],
                    )?)],
                )
                .await;
                return Ok(JsValue::UNDEFINED);
            }
            _ => (),
        }
        let me = self.who(&ctx)?;
        if !present(&me) {
            return if present(&get(&ctx, "callbackQuery")) {
                run(&ctx, "answerCallbackQuery", &[]).await
            } else {
                run(&ctx, "reply", &["Напиши мне в личные сообщения.".into()]).await
            };
        }
        let matches = get(&ctx, "match");
        match route {
            "start" => {
                let payload = string(&matches).trim().to_string();
                let invite = payload.strip_prefix("duel_").filter(|s| {
                    !s.is_empty()
                        && s.bytes()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                });
                if self
                    .needs_contract(&ctx, &me, if invite.is_some() { &payload } else { "" })
                    .await?
                {
                    return Ok(JsValue::UNDEFINED);
                }
                if let Some(code) = invite {
                    self.open_invite(&ctx, &me, code).await
                } else {
                    self.welcome(&ctx).await
                }
            }
            "contract" => {
                let fresh = !present(&get(&me, "tag"));
                let operator = try_call(
                    &self.operators(),
                    "sign",
                    &[get(&me, "id"), get(&me, "name")],
                )?;
                let answer = if fresh {
                    object(&[(
                        "text",
                        format!("Жетон {}", string(&get(&operator, "tag"))).into(),
                    )])
                } else {
                    JsValue::UNDEFINED
                };
                run(&ctx, "answerCallbackQuery", &[answer]).await?;
                let _ = run(&ctx, "editMessageReplyMarkup", &[empty_buttons()]).await;
                if fresh {
                    run(
                        &ctx,
                        "reply",
                        &[view("signedText", &[get(&operator, "tag")])?, html()],
                    )
                    .await?;
                }
                let next = string(&at(&matches, 1));
                if let Some(code) = next.strip_prefix("duel_") {
                    self.open_invite(&ctx, &me, code).await
                } else if next == "bot" || next == "duel" {
                    self.ask_weapon(&ctx, &next, lead(&next)).await
                } else {
                    self.welcome(&ctx).await
                }
            }
            "me" => {
                if self.needs_contract(&ctx, &me, "").await? {
                    return Ok(JsValue::UNDEFINED);
                }
                let operator = try_call(&self.operators(), "byTelegram", &[get(&me, "id")])?;
                run(
                    &ctx,
                    "reply",
                    &[
                        view("tagText", &[get(&me, "tag"), get(&operator, "name")])?,
                        html(),
                    ],
                )
                .await
            }
            "bot" | "duel" | "menu" => {
                let mode = if route == "menu" {
                    string(&at(&matches, 1))
                } else {
                    route.into()
                };
                if self.needs_contract(&ctx, &me, &mode).await? {
                    return Ok(JsValue::UNDEFINED);
                }
                if route == "menu" {
                    run(&ctx, "answerCallbackQuery", &[]).await?;
                }
                self.ask_weapon(&ctx, &mode, lead(&mode)).await
            }
            "weapon" => {
                let mode = string(&at(&matches, 1));
                let weapon = at(&matches, 2);
                if !values(&get(&self.host(), "WEAPON_IDS")).contains(&weapon) {
                    return run(&ctx, "answerCallbackQuery", &[]).await;
                }
                let next = mode
                    .strip_prefix("join:")
                    .map(|s| format!("duel_{s}"))
                    .unwrap_or_else(|| mode.clone());
                if self.needs_contract(&ctx, &me, &next).await? {
                    return Ok(JsValue::UNDEFINED);
                }
                let player = me.clone();
                set(&player, "weapon", &weapon);
                if present(&at(&matches, 3)) {
                    set(
                        &player,
                        "kit",
                        &get(&get(&self.host(), "VETERAN_KITS"), &string(&weapon)),
                    );
                }
                let result = self.choose_weapon(&ctx, &mode, player).await;
                match result {
                    Err(error) if instance_of(&error, &get(&self.host(), "DuelError")) => {
                        run(
                            &ctx,
                            "answerCallbackQuery",
                            &[object(&[
                                ("text", get(&error, "message")),
                                ("show_alert", true.into()),
                            ])],
                        )
                        .await
                    }
                    other => other,
                }
            }
            "turn" => {
                let payload = string(&at(&matches, 3));
                if payload == "x" {
                    return run(
                        &ctx,
                        "answerCallbackQuery",
                        &[object(&[(
                            "text",
                            "🎯 — куда бить по врагу. 🛡 — какие зоны прикрыть щитом.".into(),
                        )])],
                    )
                    .await;
                }
                match self.turn(&ctx, &me, &matches, &payload).await {
                    Err(error) if instance_of(&error, &get(&self.host(), "DuelError")) => {
                        run(
                            &ctx,
                            "answerCallbackQuery",
                            &[object(&[("text", get(&error, "message"))])],
                        )
                        .await
                    }
                    other => other,
                }
            }
            _ => Ok(JsValue::UNDEFINED),
        }
    }
    async fn choose_weapon(
        &self,
        ctx: &JsValue,
        mode: &str,
        player: JsValue,
    ) -> Result<JsValue, JsValue> {
        if mode == "bot" {
            try_call(&self.duels(), "startVsBot", &[player])?;
        } else if mode == "duel" {
            let invite = try_call(&self.duels(), "createInvite", &[player])?;
            let link = format!(
                "https://t.me/{}?start=duel_{}",
                string(&get(&get(&self.bot(), "botInfo"), "username")),
                string(&get(&invite, "code"))
            );
            let url = format!(
                "https://t.me/share/url?url={}&text={}",
                string(&js_sys::encode_uri_component(&link).into()),
                string(
                    &js_sys::encode_uri_component("Вызываю тебя на дуэль в Crowned Dogs").into()
                )
            );
            let button = object(&[
                ("text", "Отправить другу".into()),
                ("data", "".into()),
                ("url", url.into()),
            ]);
            let rows: JsValue = array(&[array(&[button]).into()]).into();
            run(ctx, "reply", &[format!("Ссылка на дуэль (действует 30 минут):\n{link}\n\nКак только друг её откроет и выберет оружие, бой начнётся.").into(), markup(&rows)]).await?;
        } else {
            try_call(
                &self.duels(),
                "acceptInvite",
                &[
                    mode.strip_prefix("join:").unwrap_or_default().into(),
                    player,
                ],
            )?;
        }
        run(ctx, "answerCallbackQuery", &[]).await?;
        let _ = run(ctx, "editMessageReplyMarkup", &[empty_buttons()]).await;
        Ok(JsValue::UNDEFINED)
    }
    async fn turn(
        &self,
        ctx: &JsValue,
        me: &JsValue,
        matches: &JsValue,
        payload: &str,
    ) -> Result<JsValue, JsValue> {
        let turn = numeric(&at(matches, 2));
        let mut args = vec![get(me, "id"), at(matches, 1), turn.clone()];
        let method = if payload == "go" {
            "submit"
        } else {
            let command = engine("server.decodeCommand", &[payload.into()])?;
            if !present(&command) {
                return Err(construct(
                    &get(&self.host(), "DuelError"),
                    &["Неизвестная кнопка".into()],
                ));
            }
            args.push(command);
            "press"
        };
        let duel = try_call(&self.duels(), method, &args)?;
        let answer = if payload == "go" {
            object(&[("text", "Приказ отдан".into())])
        } else {
            JsValue::UNDEFINED
        };
        run(ctx, "answerCallbackQuery", &[answer]).await?;
        let match_state = get(&duel, "match");
        if get(&match_state, "turn") != turn || present(&get(&match_state, "result")) {
            return Ok(JsValue::UNDEFINED);
        }
        let side = engine("server.sideOf", &[duel.clone(), get(me, "id")])?;
        let screen = view("turnScreen", &[duel, side])?;
        if let Err(error) = run(
            ctx,
            "editMessageText",
            &[get(&screen, "text"), html_buttons(&get(&screen, "buttons"))],
        )
        .await
        {
            if !self.not_modified(&error) {
                return Err(error);
            }
        }
        Ok(JsValue::UNDEFINED)
    }
}

#[wasm_bindgen]
pub fn bind_server_bot(bot: JsValue, duels: JsValue, operators: JsValue, host: JsValue) -> JsValue {
    let context = object(&[
        ("bot", bot),
        ("duels", duels),
        ("operators", operators),
        ("host", host),
    ]);
    let state = Rc::new(Telegram {
        context: construct(&get(&global(), "WeakRef"), &[context.clone()]),
        screens: Map::new(),
        text: view("data", &[]).unwrap_throw(),
    });
    for (command, route) in [
        ("start", "start"),
        ("rules", "rules"),
        ("help", "help"),
        ("me", "me"),
        ("fight", "bot"),
        ("duel", "duel"),
    ] {
        register(&state, "command", command.into(), route);
    }
    for (pattern, route) in [
        (r"^c(?::(bot|duel|duel_[A-Z0-9]+))?$", "contract"),
        (r"^m:(bot|duel)$", "menu"),
        (r"^wk:(bot|duel|join:[A-Z0-9]+):(v|s)$", "weapon-kit"),
        (r"^w:(bot|duel|join:[A-Z0-9]+):([a-z]+)(:v)?$", "weapon"),
        (r"^t:([a-z0-9]+):(\d+):(.+)$", "turn"),
    ] {
        register(
            &state,
            "callbackQuery",
            RegExp::new(pattern, "").into(),
            route,
        );
    }
    register(&state, "on", "callback_query:data".into(), "stale");
    let error = Closure::<dyn FnMut(JsValue)>::new(|error| {
        log("error", "bot error", &get(&error, "error"))
    })
    .into_js_value();
    call(&state.bot(), "catch", &[error]);
    let event = Closure::<dyn FnMut(JsValue) -> Promise>::new(move |event| {
        let state = state.clone();
        let context = state.context();
        future_to_promise(async move {
            let result = state.on_event(event).await;
            drop(context);
            result
        })
    })
    .into_js_value();
    set(&event, "serverContext", &context);
    event
}
fn register(state: &Rc<Telegram>, method: &str, filter: JsValue, route: &'static str) {
    let state_callback = state.clone();
    let callback = Closure::<dyn FnMut(JsValue) -> Promise>::new(move |ctx| {
        let state = state_callback.clone();
        let context = state.context();
        future_to_promise(async move {
            let result = state.route(route, ctx).await;
            drop(context);
            result
        })
    })
    .into_js_value();
    set(&callback, "serverContext", &state.context());
    call(&state.bot(), method, &[filter, callback]);
}

#[wasm_bindgen]
pub fn run_server_main(host: JsValue) -> Promise {
    future_to_promise(async move {
        let process = get(&host, "process");
        let env = get(&process, "env");
        let token = string(&get(&env, "BOT_TOKEN")).trim().to_string();
        if token.is_empty() {
            log(
                "error",
                "Нет BOT_TOKEN. Получи токен у @BotFather, положи его в файл .env (образец в .env.example) и запусти снова.",
                &JsValue::UNDEFINED,
            );
            try_call(&process, "exit", &[1.into()])?;
            return Ok(JsValue::UNDEFINED);
        }
        let file = string(&get(&env, "OPERATORS_FILE")).trim().to_string();
        let file = if file.is_empty() {
            "data/operators.json"
        } else {
            &file
        };
        let operators = construct(
            &get(&host, "OperatorRegistry"),
            &[object(&[("file", file.into())])],
        );
        let service = try_call(
            &host,
            "createBot",
            &[token.into(), object(&[("operators", operators)])],
        )?;
        let bot = get(&service, "bot");
        let duels = get(&service, "duels");
        if let Err(error) = run(&bot, "init", &[]).await {
            log(
                "error",
                "Не получилось подключиться к Telegram. Проверь BOT_TOKEN и интернет.",
                &error,
            );
            try_call(&process, "exit", &[1.into()])?;
            return Ok(JsValue::UNDEFINED);
        }
        let commands: Vec<JsValue> = [
            ("fight", "Бой с ботом"),
            ("duel", "Позвать друга на дуэль"),
            ("rules", "Правила коротко"),
            ("me", "Мой жетон"),
        ]
        .iter()
        .map(|(command, description)| {
            object(&[
                ("command", (*command).into()),
                ("description", (*description).into()),
            ])
        })
        .collect();
        run(
            &get(&bot, "api"),
            "setMyCommands",
            &[array(&commands).into()],
        )
        .await?;
        let stop_bot = bot.clone();
        let stop = Closure::<dyn FnMut()>::new(move || {
            let _ = try_call(&duels, "close", &[]);
            let _ = try_call(&stop_bot, "stop", &[]);
        })
        .into_js_value();
        try_call(&process, "once", &["SIGINT".into(), stop.clone()])?;
        try_call(&process, "once", &["SIGTERM".into(), stop.clone()])?;
        let on_start = Closure::<dyn FnMut(JsValue)>::new(|me| {
            log(
                "log",
                &format!("Бот @{} запущен", string(&get(&me, "username"))),
                &JsValue::UNDEFINED,
            );
        })
        .into_js_value();
        let result = run(&bot, "start", &[object(&[("onStart", on_start)])]).await;
        let _ = try_call(&process, "removeListener", &["SIGINT".into(), stop.clone()]);
        let _ = try_call(&process, "removeListener", &["SIGTERM".into(), stop]);
        result
    })
}
