use std::collections::HashSet;

use anyhow::{Context, Result, bail, ensure};
use cookie::Cookie;
use regex::Regex;
use reqwest::{Method, header};
use rsa::pkcs8::DecodePublicKey;
use rsa::sha2::Sha256;
use rsa::{Oaep, RsaPublicKey};
use serde::{Deserialize, Serialize};

use crate::bilibili::{BiliError, Client, ErrorForStatusExt, Validate};

const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38,
    41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36,
    20, 34, 44, 52,
];

mod qrcode_status_code {
    pub const SUCCESS: i64 = 0;
    pub const NOT_SCANNED: i64 = 86101;
    pub const SCANNED_UNCONFIRMED: i64 = 86090;
    pub const EXPIRED: i64 = 86038;
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub sessdata: String,
    pub bili_jct: String,
    pub buvid3: String,
    pub dedeuserid: String,
    pub ac_time_value: String,
}

#[derive(Debug, Deserialize)]
pub struct WbiImg {
    pub(crate) img_url: String,
    pub(crate) sub_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Qrcode {
    pub url: String,
    pub qrcode_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollStatus {
    Success {
        credential: Credential,
    },
    Pending {
        message: String,
        #[serde(default)]
        scanned: bool,
    },
    Expired {
        message: String,
    },
}

impl WbiImg {
    pub fn into_mixin_key(self) -> Option<String> {
        let key = match (get_filename(self.img_url.as_str()), get_filename(self.sub_url.as_str())) {
            (Some(img_key), Some(sub_key)) => img_key.to_string() + sub_key,
            _ => return None,
        };
        let key = key.as_bytes();
        Some(MIXIN_KEY_ENC_TAB.iter().take(32).map(|&x| key[x] as char).collect())
    }
}

impl Credential {
    pub async fn wbi_img(&self, client: &Client) -> Result<WbiImg> {
        let mut res = client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/nav", Some(self))
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"]["wbi_img"].take())?)
    }

    pub async fn generate_qrcode(client: &Client) -> Result<Qrcode> {
        let mut res = client
            .request(
                Method::GET,
                "https://passport.bilibili.com/x/passport-login/web/qrcode/generate",
                None,
            )
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn poll_qrcode(client: &Client, qrcode_key: &str) -> Result<PollStatus> {
        let mut resp = client
            .request(
                Method::GET,
                "https://passport.bilibili.com/x/passport-login/web/qrcode/poll",
                None,
            )
            .query(&[("qrcode_key", qrcode_key)])
            .send()
            .await?
            .error_for_status_ext()?;
        let headers = std::mem::take(resp.headers_mut());
        let json = resp.json::<serde_json::Value>().await?.validate()?;
        let code = json["data"]["code"].as_i64().context("missing 'code' field in data")?;

        match code {
            qrcode_status_code::SUCCESS => {
                let mut credential = Self::extract(headers, json)?;
                credential.buvid3 = Self::get_buvid3(client).await?;
                Ok(PollStatus::Success { credential })
            }
            qrcode_status_code::NOT_SCANNED => Ok(PollStatus::Pending {
                message: "未扫描".to_owned(),
                scanned: false,
            }),
            qrcode_status_code::SCANNED_UNCONFIRMED => Ok(PollStatus::Pending {
                message: "已扫描，请在手机上确认登录".to_owned(),
                scanned: true,
            }),
            qrcode_status_code::EXPIRED => Ok(PollStatus::Expired {
                message: "二维码已过期".to_owned(),
            }),
            _ => {
                bail!(BiliError::InvalidResponse(json.to_string()));
            }
        }
    }

    /// 获取 buvid3 浏览器指纹
    ///
    /// 参考 https://github.com/SocialSisterYi/bilibili-API-collect/blob/master/docs/misc/buvid3_4.md
    async fn get_buvid3(client: &Client) -> Result<String> {
        let resp = client
            .request(Method::GET, "https://api.bilibili.com/x/web-frontend/getbuvid", None)
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        resp["data"]["buvid"]
            .as_str()
            .context("missing 'buvid' field in data")
            .map(|s| s.to_string())
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
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        res["data"]["refresh"].as_bool().context("check refresh failed")
    }

    pub async fn refresh(&self, client: &Client) -> Result<Self> {
        let correspond_path = Self::get_correspond_path();
        let csrf = self
            .get_refresh_csrf(client, correspond_path)
            .await
            .context("获取 refresh_csrf 失败")?;
        let new_credential = self
            .get_new_credential(client, &csrf)
            .await
            .context("刷新 Credential 失败")?;
        self.confirm_refresh(client, &new_credential)
            .await
            .context("确认更新 Credential 失败")?;
        Ok(new_credential)
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
        .expect("fail to decode public key");
        // 精确到毫秒的时间戳可能出现时间比服务器快的情况，提前 20s 以防万一
        let ts = chrono::Local::now().timestamp_millis() - 20000;
        let data = format!("refresh_{}", ts).into_bytes();
        let encrypted = key
            .encrypt(&mut rand::rng(), Oaep::new::<Sha256>(), &data)
            .expect("fail to encrypt");
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
            .await?
            .error_for_status_ext()?;
        regex_find(r#"<div id="1-name">(.+?)</div>"#, res.text().await?.as_str())
    }

    async fn get_new_credential(&self, client: &Client, csrf: &str) -> Result<Credential> {
        let mut resp = client
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
            .await?
            .error_for_status_ext()?;
        let headers = std::mem::take(resp.headers_mut());
        let json = resp.json::<serde_json::Value>().await?.validate()?;
        let mut credential = Self::extract(headers, json)?;
        credential.buvid3 = self.buvid3.clone();
        Ok(credential)
    }

    async fn confirm_refresh(&self, client: &Client, new_credential: &Credential) -> Result<()> {
        client
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
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(())
    }

    /// 解析 header 和 json，获取除 buvid3 字段外全部填充的 Credential
    fn extract(headers: header::HeaderMap, json: serde_json::Value) -> Result<Credential> {
        let mut credential = Credential::default();
        let required_cookies = HashSet::from(["SESSDATA", "bili_jct", "DedeUserID"]);
        let cookies: Vec<Cookie> = headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|x| x.to_str().ok())
            .filter_map(|x| Cookie::parse(x).ok())
            .filter(|x| required_cookies.contains(x.name()))
            .collect();
        ensure!(
            cookies.len() == required_cookies.len(),
            "not all required cookies found"
        );
        for cookie in cookies {
            match cookie.name() {
                "SESSDATA" => credential.sessdata = cookie.value().to_string(),
                "bili_jct" => credential.bili_jct = cookie.value().to_string(),
                "DedeUserID" => credential.dedeuserid = cookie.value().to_string(),
                _ => unreachable!(),
            }
        }
        match json["data"]["refresh_token"].as_str() {
            Some(token) => credential.ac_time_value = token.to_string(),
            None => bail!("refresh_token not found"),
        }
        Ok(credential)
    }
}

// 用指定的 pattern 正则表达式在 doc 中查找，返回第一个匹配的捕获组
fn regex_find(pattern: &str, doc: &str) -> Result<String> {
    let re = Regex::new(pattern)?;
    Ok(re
        .captures(doc)
        .context("no match found")?
        .get(1)
        .context("no capture found")?
        .as_str()
        .to_string())
}

fn get_filename(url: &str) -> Option<&str> {
    url.rsplit_once('/')
        .and_then(|(_, s)| s.rsplit_once('.'))
        .map(|(s, _)| s)
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

    #[test]
    fn test_encode_query() {
        let query = vec![
            ("bar", "五一四".to_string()),
            ("baz", "1919810".to_string()),
            ("foo", "one one four".to_string()),
        ];
        assert_eq!(
            serde_urlencoded::to_string(query).unwrap().replace('+', "%20"),
            "bar=%E4%BA%94%E4%B8%80%E5%9B%9B&baz=1919810&foo=one%20one%20four"
        );
    }

    #[test]
    fn test_extract_credential_success() {
        let mut headers = header::HeaderMap::new();
        headers.append(
            header::SET_COOKIE,
            "SESSDATA=test_sessdata; Path=/; Domain=bilibili.com".parse().unwrap(),
        );
        headers.append(
            header::SET_COOKIE,
            "bili_jct=test_jct; Path=/; Domain=bilibili.com".parse().unwrap(),
        );
        headers.append(
            header::SET_COOKIE,
            "DedeUserID=123456; Path=/; Domain=bilibili.com".parse().unwrap(),
        );

        let json = serde_json::json!({
            "data": {
                "refresh_token": "test_refresh_token"
            }
        });

        let credential = Credential::extract(headers, json).unwrap();

        assert_eq!(credential.sessdata, "test_sessdata");
        assert_eq!(credential.bili_jct, "test_jct");
        assert_eq!(credential.dedeuserid, "123456");
        assert_eq!(credential.ac_time_value, "test_refresh_token");
        assert!(credential.buvid3.is_empty());
    }

    #[test]
    fn test_extract_credential_missing_sessdata() {
        let headers = header::HeaderMap::new();
        let json = serde_json::json!({
            "data": {
                "refresh_token": "test_refresh_token"
            }
        });
        assert!(Credential::extract(headers, json).is_err());
    }

    #[test]
    fn test_extract_credential_missing_refresh_token() {
        let mut headers = header::HeaderMap::new();
        headers.append(header::SET_COOKIE, "SESSDATA=test_sessdata".parse().unwrap());
        headers.append(header::SET_COOKIE, "bili_jct=test_jct".parse().unwrap());
        headers.append(header::SET_COOKIE, "DedeUserID=123456".parse().unwrap());
        let json = serde_json::json!({
            "data": {}
        });
        assert!(Credential::extract(headers, json).is_err());
    }

    #[ignore = "requires manual testing with real QR code scan"]
    #[tokio::test]
    async fn test_qrcode_login_flow() -> Result<()> {
        let client = Client::new();
        // 1. 生成二维码
        let qr_response = Credential::generate_qrcode(&client).await?;
        println!("二维码 URL: {}", qr_response.url);
        println!("qrcode_key: {}", qr_response.qrcode_key);
        println!("\n请使用 B 站 APP 扫描二维码...\n");
        // 2. 轮询登录状态（最多轮询 90 次，每 2 秒一次，共 180 秒）
        for i in 1..=90 {
            println!("第 {} 次轮询...", i);
            let status = Credential::poll_qrcode(&client, &qr_response.qrcode_key).await?;
            match status {
                PollStatus::Success { credential } => {
                    println!("\n登录成功！");
                    println!("SESSDATA: {}", credential.sessdata);
                    println!("bili_jct: {}", credential.bili_jct);
                    println!("buvid3: {}", credential.buvid3);
                    println!("DedeUserID: {}", credential.dedeuserid);
                    println!("ac_time_value: {}", credential.ac_time_value);
                    return Ok(());
                }
                PollStatus::Pending { message, scanned } => {
                    println!("状态: {}, 已扫描: {}", message, scanned);
                }
                PollStatus::Expired { message } => {
                    println!("\n二维码已过期: {}", message);
                    anyhow::bail!("二维码过期");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        bail!("轮询超时")
    }
}
