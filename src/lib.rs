use {
    actix_web::{
        HttpResponse,
        Responder,
        web::Form,
    },
    dotenv::dotenv,
    serde::Deserialize,
    std::env::var as env_var,
};
#[derive(Deserialize)]
pub struct InboundMessage {
    #[serde(rename = "MessageSid")]
    message_sid: String,
    #[serde(rename = "AccountSid")]
    account_sid: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Body")]
    body: String,
}
impl InboundMessage {
    pub fn get_message_sid(&self) -> String {
        self.message_sid.clone()
    }
    pub fn get_account_sid(&self) -> String {
        self.account_sid.clone()
    }
    pub fn get_from(&self) -> String {
        self.from.clone()
    }
    pub fn get_to(&self) -> String {
        self.to.clone()
    }
    pub fn get_body(&self) -> String {
        self.body.clone()
    }
}
pub enum NumberAcceptance<'a> {
    All,
    Specific(Vec<&'a str>),
    Single(&'a str),
}
fn check_num(acc: NumberAcceptance, msg: &InboundMessage) -> bool {
    match acc {
        NumberAcceptance::All => true,
        NumberAcceptance::Single(n) => {
            if msg.from.eq(n) {
                true
            } else {
                false
            }
        },
        NumberAcceptance::Specific(ns) => {
            if ns.contains(&msg.from.as_str()) {
                true
            } else {
                false
            }
        },
    }
}
pub fn recv(
    req: Form<InboundMessage>, func: &dyn Fn(InboundMessage) -> bool
) -> impl Responder {
    const ACCEPTED_KEY: &'static str = "TWILIO_RECV_ACCEPTED_NUMS";
    match dotenv() {
        Err(_) => {
            return HttpResponse::InternalServerError().body(
                "Failed to derive dotenv environment"
            );
        },
        _ => {},
    }
    let accepted_nums = match env_var(ACCEPTED_KEY) {
        Ok(an) => an,
        Err(_) => {
            return HttpResponse::InternalServerError().body(
                "Failed to retrieve accepted numbers from environment"
            );
        },
    };
    let nums;
    if accepted_nums.eq(&"*") {
        nums = NumberAcceptance::All;
    } else if accepted_nums.contains(',') {
        nums = NumberAcceptance::Specific(
            accepted_nums.split(',').collect::<Vec<&str>>()
        );
    } else {
        nums = NumberAcceptance::Single(
            accepted_nums.as_str()
        );
    }
    let msg = req.into_inner();
    if !check_num(nums, &msg) {
        HttpResponse::InternalServerError().body(
            "Invalid \"from\" number"
        )
    } else if func(msg) {
        HttpResponse::Ok().body(
            "Message handler succeeded"
        )
    } else {
        HttpResponse::InternalServerError().body(
            "Message handler failed"
        )
    }
}
