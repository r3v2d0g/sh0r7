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
use wasm_bindgen::JsCast;
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

    #[wasm_bindgen(js_name = fetch)]
    fn raw_fetch(url: &str) -> Promise;

    #[wasm_bindgen(js_class = "cache")]
    pub type Cache;

    #[wasm_bindgen(method, js_class = "cache", js_name = match)]
    fn raw_match(this: &Cache, url: &str) -> Promise;

    #[wasm_bindgen(method, js_class = "cache", js_name = put)]
    fn raw_put(this: &Cache, url: &str, response: Response) -> Promise;
}

// ========================================== handle() ========================================== \\

#[wasm_bindgen]
pub async fn handle(req: Request, kv: Kv, cache: Cache) -> Result<Response, JsValue> {
    let url = Url::parse(&req.url()).unwrap_throw();
    let domain = url.domain().unwrap();
    let path = url.path();

    let mut key = String::with_capacity(domain.len() + path.len() + 1);
    key.push_str(domain);
    key.push_str(path);

    if !path.ends_with('/') {
        key.push('/');
    }

    let mut value = if let Some(value) = kv.get(&key).await? {
        value
    } else if let Some(value) = kv.get(domain).await? {
        value
    } else {
        return Response::new_with_opt_str_and_init(None, ResponseInit::new().status(404));
    };

    if value.fetch {
        if let Some(resp) = cache.get(&value.url).await? {
            return Ok(resp);
        }

        let resp = fetch(&value.url).await?;
        cache.put(&value.url, resp.clone()?).await?;

        return Ok(resp);
    }

    let status = if value.permanent {
        301
    } else {
        307
    };

    if value.append_path {
        value.url.push_str(path);
    }

    Response::redirect_with_status(&value.url, status)
}

// =========================================== fetch() ========================================== \\

async fn fetch(url: &str) -> Result<Response, JsValue> {
    JsFuture::from(raw_fetch(url)).await?.dyn_into::<Response>()
}

// ============================================ Types =========================================== \\

#[derive(Default)]
/// ## Format
///
/// ```text
/// key = match_all | match_path
/// match_all = domain
/// match_path = domain path (/)?
///
/// value = version ':' options ':' url
/// version = '000'..'001'
///
/// options_v000 = permanent ':' append_path
/// options_v001 = options_v000 ':' fetch
///
/// permanent = 't' | 'f'
/// append_path = 't' | 'f'
/// fetch = 't' | 'f'
/// ```
struct Value {
    permanent: bool,
    append_path: bool,
    fetch: bool,
    url: String,
}

// =========================================== impl Kv ========================================== \\

impl Kv {
    // ======================================== Read ======================================== \\

    async fn get(&self, key: &str) -> Result<Option<Value>, JsValue> {
        if let Some(value) = JsFuture::from(self.raw_get(key, "text")).await?.as_string() {
            Ok(Some(Value::parse(&value)?))
        } else {
            Ok(None)
        }
    }
}

// ========================================= impl Cache ========================================= \\

impl Cache {
    // ======================================== Read ======================================== \\

    async fn get(&self, url: &str) -> Result<Option<Response>, JsValue> {
        Ok(JsFuture::from(self.raw_match(url)).await?.dyn_into::<Response>().ok())
    }

    // ======================================== Write ======================================= \\

    async fn put(&self, url: &str, resp: Response) -> Result<(), JsValue> {
        JsFuture::from(self.raw_put(url, resp)).await?;
        Ok(())
    }
}

// ========================================= impl Value ========================================= \\

impl Value {
    // ==================================== Constructors ==================================== \\

    fn parse(value: &str) -> Result<Self, JsValue> {
        match u8::from_str_radix(
            value.get(0..3).unwrap_throw(),
            10,
        ).unwrap_throw() {
            0 => Self::parse_v000(value),
            1 => Self::parse_v001(value),
            _ => Err(JsValue::from_str(&"unknown version")),
        }
    }

    fn parse_v000(value: &str) -> Result<Self, JsValue> {
        Ok(Value {
            permanent: Self::parse_option(value, 0)?,
            append_path: Self::parse_option(value, 1)?,
            url: Self::extract_url(value, 2)?,
            ..Default::default()
        })
    }

    fn parse_v001(value: &str) -> Result<Self, JsValue> {
        Ok(Value {
            permanent: Self::parse_option(value, 0)?,
            append_path: Self::parse_option(value, 1)?,
            fetch: Self::parse_option(value, 2)?,
            url: Self::extract_url(value, 3)?,
        })
    }

    // ======================================= Helpers ====================================== \\

    fn parse_option(value: &str, idx: usize) -> Result<bool, JsValue> {
        match value.chars().nth(4 + 2 * idx).ok_or(JsValue::from_str(&"invalid value"))? {
            't' => Ok(true),
            'f' => Ok(false),
            _ => Err(JsValue::from_str(&"invalid option value"))
        }
    }

    fn extract_url(value: &str, options: usize) -> Result<String, JsValue> {
        value.get((4 + 2 * options)..).map(String::from).ok_or(JsValue::from_str(&"invalid value"))
    }
}
