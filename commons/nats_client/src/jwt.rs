use crypto::base64::b64_encode;
use nkeys::KeyPair;
use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
struct JWTHeader {
    typ: String,
    alg: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NatsClaim {
    #[serde(rename = "pub")]
    _pub: NatsSubjectList,
    sub: NatsSubjectList,
    subs: i8,
    data: i8,
    payload: i8,
    issuer_account: String,
    #[serde(rename = "type")]
    _type: String,
    version: u8
}

#[derive(Debug, Serialize, Deserialize)]
struct NatsSubjectList {
    allow: Vec<String>,
    deny: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    name: String,
    iat: usize,
    iss: String,
    exp: usize,
    nats: NatsClaim,
}

fn generate_user_jwt(user_public_key: &str, account_public_key: &str, account_signing_key: KeyPair) -> Result<String, ()> {
    let my_claims = Claims {
        sub: user_public_key.to_owned(),
        name: "user_2".to_string(),
        iat: 1694806456,
        iss: account_public_key.to_owned(),
        exp: 10000000000,
        nats: NatsClaim {
            _pub: NatsSubjectList {
                deny: vec![],
                allow: vec!["foo".to_string()]
            },
            sub: NatsSubjectList {
                deny: vec![],
                allow: vec!["foo".to_string()]
            },
            subs: -1,
            data: -1,
            payload: -1,
            issuer_account: "AAM54HW4JLIVU2OSHM34UOTQTKD65L522TAR36HNCPS7AKSGT2ECD4AR".to_string(),
            _type: "user".to_string(),
            version: 2,
        }
    };

    let header = JWTHeader { 
        typ: "JWT".to_string(),
        alg: "ed25519-nkey".to_string(),
    };

    let encoded_header = b64_encode(serde_json::to_vec(&header).unwrap());
    let encoded_claims = b64_encode(serde_json::to_vec(&my_claims).unwrap());

    let message = [encoded_header, encoded_claims].join(".");

    let signature = account_signing_key.sign(message.as_bytes()).unwrap();
    let encoded_signature = b64_encode(signature);
    Ok([message, encoded_signature].join("."))
}

pub fn create_dummy_jwt(user_key: &KeyPair) -> Result<String, ()> {
    let account_public_key = String::from("AC3ABYTYEVS2362XE65U4VODFDI45WKGOKO5F7V7ZWBLRL2XFNALTO6N");
    let account_seed = "SAANAKBWHE4QY772SHRTQKULC2UWS33IU33U54PPCG4BLUR3DYG3I7C4TU";
    let account_signing_key = nkeys::KeyPair::from_seed(account_seed).unwrap();

    let token = generate_user_jwt(&user_key.public_key(), &account_public_key, account_signing_key);
    Ok(token.unwrap())
}