/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// =========================================== Imports ========================================== \\

use js_sys::Promise;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, Response, ResponseInit};

// ========================================= extern "C" ========================================= \\

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_class = "kv")]
    pub type Kv;

    #[wasm_bindgen(method, js_class = "kv", js_name = get)]
    fn raw_get(this: &Kv, key: &str, ty: &str) -> Promise;
}

// ========================================== handle() ========================================== \\

#[wasm_bindgen]
pub async fn handle(req: Request, kv: Kv) -> Result<Response, JsValue> {
    let url = Url::parse(&req.url()).unwrap_throw();
    let domain = url.domain().unwrap();
    let path = url.path();

    let mut key = String::with_capacity(domain.len() + path.len() + 1);
    key.push_str(domain);
    key.push_str(path);

    if !path.ends_with('/') {
        key.push('/');
    }

    let value = if let Some(value) = kv.get(&key).await? {
        value
    } else if let Some(value) = kv.get(domain).await? {
        value
    } else {
        return Response::new_with_opt_str_and_init(None, ResponseInit::new().status(404));
    };

    let (permanent, append_path, url) = parse(&value)?;
    let status = if permanent {
        301
    } else {
        307
    };

    if append_path {
        let mut url = String::from(url);
        url.push_str(path);

        Response::redirect_with_status(&url, status)
    } else {
        Response::redirect_with_status(url, status)
    }
}

// =========================================== parse() ========================================== \\

/// ## Format
///
/// ```text
/// key = match_all | match_path
/// match_all = domain
/// match_path = domain path (/)?
///
/// value = version ':' permanent ':' append_path ':' url
/// version = '000'
/// permanent = 't' | 'f'
/// append_path = 't' | 'f'
/// ```
fn parse(value: &str) -> Result<(bool, bool, &str), JsValue> {
    fn parse_option(value: char) -> Result<bool, JsValue> {
        match value {
            't' => Ok(true),
            'f' => Ok(false),
            _ => Err(JsValue::from_str(&"unknown option value")),
        }
    }

    match u8::from_str_radix(value.get(0..3).unwrap_throw(), 10).unwrap_throw() {
        0 => {
            Ok((
                parse_option(value.chars().nth(4).unwrap_throw())?,
                parse_option(value.chars().nth(6).unwrap_throw())?,
                value.get(8..).unwrap_throw(),
            ))
        },
        _ => Err(JsValue::from_str(&"unknown version")),
    }
}

// =========================================== impl Kv ========================================== \\

impl Kv {
    // ===================================== Read+Write ===================================== \\

    pub async fn get(&self, key: &str) -> Result<Option<String>, JsValue> {
        Ok(JsFuture::from(self.raw_get(key, "text")).await?.as_string())
    }
}
