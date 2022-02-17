use {
    actix_web::{
        HttpResponse,
        Responder,
        web::Form,
    },
    chrono::Utc,
    serde::Deserialize,
    std::{
        env::var as env_var,
        fs::OpenOptions,
        io::Write,
        path::PathBuf,
    },
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
enum LogType {
    Log,
    Error,
}
impl LogType {
    fn to_str(&self) -> String {
        match self {
            Self::Log => {
                format!("LOG  ")
            },
            Self::Error => {
                format!("ERROR")
            },
        }
    }
}
fn write_to_log(log_type: LogType, msg: impl AsRef<str>) {
    const LOG_DIR: &'static str = "TWILIO_RECV_LOG_DIR";
    const BASE_NAME: &'static str = "twilio_sms_recv";
    let now = Utc::now();
    let now_short_fmt = now.format("%Y%m%d");
    let now_long_fmt = now.format("%+");
    let log_dir = match env_var(LOG_DIR) {
        Ok(l) => l,
        Err(e) => {
            println!("Failed to find environment variable for log: {}", e);
            return;
        },
    };
    let mut path = PathBuf::from(log_dir);
    let file_name = format!("{}.{}.log", BASE_NAME, now_short_fmt);
    path.push(file_name);
    let mut file = match OpenOptions::new()
        .create(true)
        .write(true)
        .read(false)
        .append(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to open {} for writing: {}", path.to_str().unwrap(), e);
            return;
        },
    };
    match file.write_all(
        format!("{} {}: {}\n", log_type.to_str(), now_long_fmt, msg.as_ref())
            .as_bytes()
    ) {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to write to {}: {}", path.to_str().unwrap(), e);
            return;
        },
    }
}
fn log(msg: impl AsRef<str>) {
    write_to_log(LogType::Log, msg);
}
fn error(msg: impl AsRef<str>) {
    write_to_log(LogType::Error, msg);
}
pub fn recv(
    req: Form<InboundMessage>, func: &dyn Fn(InboundMessage) -> bool
) -> impl Responder {
    log("Received request");
    const ACCEPTED_KEY: &'static str = "TWILIO_RECV_ACCEPTED_NUMS";
    let accepted_nums = match env_var(ACCEPTED_KEY) {
        Ok(an) => an,
        Err(e) => {
            error(format!(
                "Failed to retrieve variable from environment {}: {}",
                ACCEPTED_KEY, e
            ));
            return HttpResponse::InternalServerError().body(
                "Failed to retrieve accepted numbers from environment"
            );
        },
    };
    let nums;
    if accepted_nums.eq(&"*") {
        log("Accepting all numbers");
        nums = NumberAcceptance::All;
    } else if accepted_nums.contains(',') {
        log("Accepting specific numbers");
        nums = NumberAcceptance::Specific(
            accepted_nums.split(',').collect::<Vec<&str>>()
        );
    } else {
        log("Accepting single number");
        nums = NumberAcceptance::Single(
            accepted_nums.as_str()
        );
    }
    let msg = req.into_inner();
    if !check_num(nums, &msg) {
        error("From number failed check against accepted numbers");
        HttpResponse::InternalServerError().body(
            "Invalid \"from\" number"
        )
    } else if func(msg) {
        log("Handler succeeded");
        HttpResponse::Ok().body(
            "Message handler succeeded"
        )
    } else {
        error("Handler failed");
        HttpResponse::InternalServerError().body(
            "Message handler failed"
        )
    }
}
