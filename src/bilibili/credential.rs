use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use cookie::Cookie;
use regex::Regex;
use reqwest::{header, Method};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

use crate::bilibili::Client;
use crate::Result;

#[derive(Default)]
pub struct Credential {
    pub sessdata: String,
    pub bili_jct: String,
    pub buvid3: String,
    pub dedeuserid: String,
    pub ac_time_value: String,
}

impl Credential {
    pub fn new(
        sessdata: String,
        bili_jct: String,
        buvid3: String,
        dedeuserid: String,
        ac_time_value: String,
    ) -> Self {
        Self {
            sessdata,
            bili_jct,
            buvid3,
            dedeuserid,
            ac_time_value,
        }
    }

    pub async fn check(&self, client: &Client) -> Result<bool> {
        let res = client
            .request(
                Method::GET,
                "https://passport.bilibili.com/x/passport-login/web/cookie/info",
                Some(self),
            )
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        res["refresh"]
            .as_bool()
            .ok_or("check refresh failed".into())
    }

    pub async fn refresh(&mut self, client: &Client) -> Result<()> {
        let correspond_path = Self::get_correspond_path();
        let csrf = self.get_refresh_csrf(client, correspond_path).await?;
        let new_credential = self.get_new_credential(client, &csrf).await?;
        self.sessdata = new_credential.sessdata;
        self.bili_jct = new_credential.bili_jct;
        self.dedeuserid = new_credential.dedeuserid;
        Ok(())
    }

    fn get_correspond_path() -> String {
        // maybe as a static value
        let key = RsaPublicKey::from_public_key_pem(
            "-----BEGIN PUBLIC KEY-----
        MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDLgd2OAkcGVtoE3ThUREbio0Eg
        Uc/prcajMKXvkCKFCWhJYJcLkcM2DKKcSeFpD/j6Boy538YXnR6VhcuUJOhH2x71
        nzPjfdTcqMz7djHum0qSZA0AyCBDABUqCrfNgCiJ00Ra7GmRj+YCK1NJEuewlb40
        JNrRuoEUXpabUzGB8QIDAQAB
        -----END PUBLIC KEY-----",
        )
        .unwrap();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let data = format!("refresh_{}", ts).into_bytes();
        let mut rng = rand::rngs::OsRng;
        let encrypted = key.encrypt(&mut rng, Pkcs1v15Encrypt, &data).unwrap();
        hex::encode(encrypted)
    }

    async fn get_refresh_csrf(&self, client: &Client, correspond_path: String) -> Result<String> {
        let res = client
            .request(
                Method::GET,
                format!("https://www.bilibili.com/correspond/1/{}", correspond_path).as_str(),
                Some(self),
            )
            .header(header::COOKIE, "Domain=.bilibili.com")
            .send()
            .await?;
        if !res.status().is_success() {
            return Err("error get csrf".into());
        }
        let re = Regex::new("<div id=\"1-name\">(.+?)</div>").unwrap();
        if let Some(res) = re.find(&res.text().await?) {
            return Ok(res.as_str().to_string());
        }
        Err("error get csrf".into())
    }

    async fn get_new_credential(&self, client: &Client, csrf: &str) -> Result<Credential> {
        let res = client
            .request(
                Method::POST,
                "https://passport.bilibili.com/x/passport-login/web/cookie/refresh",
                Some(self),
            )
            .header(header::COOKIE, "Domain=.bilibili.com")
            .json(&serde_json::json!({
                "csrf": self.bili_jct,
                "refresh_csrf": csrf,
                "refresh_token": self.ac_time_value,
                "source": "main_web",
            }))
            .send()
            .await?;
        let set_cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .ok_or("error refresh credential")?
            .to_str()
            .unwrap();
        let mut credential = Credential::default();
        let required_cookies = HashSet::from(["SESSDATA", "bili_jct", "DedeUserID"]);
        let cookies: Vec<Cookie> = Cookie::split_parse_encoded(set_cookie)
            .filter(|x| {
                x.as_ref()
                    .is_ok_and(|x| required_cookies.contains(x.name()))
            })
            .map(|x| x.unwrap())
            .collect();
        for cookie in cookies {
            match cookie.name() {
                "SESSDATA" => credential.sessdata = cookie.value().to_string(),
                "bili_jct" => credential.bili_jct = cookie.value().to_string(),
                "DedeUserID" => credential.dedeuserid = cookie.value().to_string(),
                _ => continue,
            }
        }
        let json = res.json::<serde_json::Value>().await?;
        if !json["data"]["refresh_token"].is_string() {
            return Err("error refresh credential".into());
        }
        credential.ac_time_value = json["data"]["refresh_token"].as_str().unwrap().to_string();
        Ok(credential)
    }
}
