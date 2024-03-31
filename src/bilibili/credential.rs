use std::collections::HashSet;

use cookie::Cookie;
use regex::Regex;
use reqwest::{header, Method};
use rsa::pkcs8::DecodePublicKey;
use rsa::sha2::Sha256;
use rsa::{Oaep, RsaPublicKey};
use serde::{Deserialize, Serialize};

use crate::bilibili::Client;
use crate::Result;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub sessdata: String,
    pub bili_jct: String,
    pub buvid3: String,
    pub dedeuserid: String,
    pub ac_time_value: String,
}

impl Credential {
    pub fn new(sessdata: String, bili_jct: String, buvid3: String, dedeuserid: String, ac_time_value: String) -> Self {
        Self {
            sessdata,
            bili_jct,
            buvid3,
            dedeuserid,
            ac_time_value,
        }
    }

    /// 检查凭据是否有效
    pub async fn need_refresh(&self, client: &Client) -> Result<bool> {
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
        res["data"]["refresh"].as_bool().ok_or("check refresh failed".into())
    }

    pub async fn refresh(&mut self, client: &Client) -> Result<()> {
        let correspond_path = Self::get_correspond_path();
        let csrf = self.get_refresh_csrf(client, correspond_path).await?;
        let new_credential = self.get_new_credential(client, &csrf).await?;
        self.confirm_refresh(client, &new_credential).await?;
        *self = new_credential;
        Ok(())
    }

    fn get_correspond_path() -> String {
        // 调用频率很低，让 key 在函数内部构造影响不大
        let key = RsaPublicKey::from_public_key_pem(
            "-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDLgd2OAkcGVtoE3ThUREbio0Eg
Uc/prcajMKXvkCKFCWhJYJcLkcM2DKKcSeFpD/j6Boy538YXnR6VhcuUJOhH2x71
nzPjfdTcqMz7djHum0qSZA0AyCBDABUqCrfNgCiJ00Ra7GmRj+YCK1NJEuewlb40
JNrRuoEUXpabUzGB8QIDAQAB
-----END PUBLIC KEY-----",
        )
        .unwrap();
        let ts = chrono::Local::now().timestamp_millis();
        let data = format!("refresh_{}", ts).into_bytes();
        let mut rng = rand::rngs::OsRng;
        let encrypted = key.encrypt(&mut rng, Oaep::new::<Sha256>(), &data).unwrap();
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
            return match res.status().as_u16() {
                404 => Err("correspond path is wrong or expired".into()),
                _ => Err("get csrf failed".into()),
            };
        }
        regex_find(r#"<div id="1-name">(.+?)</div>"#, res.text().await?.as_str())
    }

    async fn get_new_credential(&self, client: &Client, csrf: &str) -> Result<Credential> {
        let res = client
            .request(
                Method::POST,
                "https://passport.bilibili.com/x/passport-login/web/cookie/refresh",
                Some(self),
            )
            .header(header::COOKIE, "Domain=.bilibili.com")
            .form(&[
                // 这里不是 json，而是 form data
                ("csrf", self.bili_jct.as_str()),
                ("refresh_csrf", csrf),
                ("refresh_token", self.ac_time_value.as_str()),
                ("source", "main_web"),
            ])
            .send()
            .await?;
        let set_cookies = res.headers().get_all(header::SET_COOKIE);
        let mut credential = Credential {
            buvid3: self.buvid3.clone(),
            ..Default::default()
        };
        let required_cookies = HashSet::from(["SESSDATA", "bili_jct", "DedeUserID"]);
        let cookies: Vec<Cookie> = set_cookies
            .iter()
            .filter_map(|x| x.to_str().ok())
            .filter_map(|x| Cookie::parse(x).ok())
            .filter(|x| required_cookies.contains(x.name()))
            .collect();
        if cookies.len() != required_cookies.len() {
            return Err("not all required cookies found".into());
        }
        for cookie in cookies {
            match cookie.name() {
                "SESSDATA" => credential.sessdata = cookie.value().to_string(),
                "bili_jct" => credential.bili_jct = cookie.value().to_string(),
                "DedeUserID" => credential.dedeuserid = cookie.value().to_string(),
                _ => unreachable!(),
            }
        }
        let json = res.json::<serde_json::Value>().await?;
        if !json["data"]["refresh_token"].is_string() {
            return Err("refresh_token not found".into());
        }
        credential.ac_time_value = json["data"]["refresh_token"].as_str().unwrap().to_string();
        Ok(credential)
    }

    async fn confirm_refresh(&self, client: &Client, new_credential: &Credential) -> Result<()> {
        let res = client
            .request(
                Method::POST,
                "https://passport.bilibili.com/x/passport-login/web/confirm/refresh",
                // 此处用的是新的凭证
                Some(new_credential),
            )
            .form(&[
                ("csrf", new_credential.bili_jct.as_str()),
                ("refresh_token", self.ac_time_value.as_str()),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        if res["code"] != 0 {
            return Err(format!("confirm refresh failed: {}", res["message"]).into());
        }
        Ok(())
    }
}

// 用指定的 pattern 正则表达式在 doc 中查找，返回第一个匹配的捕获组
fn regex_find(pattern: &str, doc: &str) -> Result<String> {
    let re = Regex::new(pattern)?;
    Ok(re
        .captures(doc)
        .ok_or("pattern not match")?
        .get(1)
        .unwrap()
        .as_str()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_find() {
        let doc = r#"
        <html lang="zh-Hans">
            <body>
                <div id="1-name">b0cc8411ded2f9db2cff2edb3123acac</div>
        </body>
        </html>
        "#;
        assert_eq!(
            regex_find(r#"<div id="1-name">(.+?)</div>"#, doc).unwrap(),
            "b0cc8411ded2f9db2cff2edb3123acac",
        );
    }
}
