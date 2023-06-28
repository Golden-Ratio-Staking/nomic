use futures_lite::future::block_on;
use nomic::orga::abci::App;
use nomic::orga::call::Call;
use nomic::orga::client::{Client, Transport};
use nomic::orga::encoding::Encode;
use nomic::orga::merk::ProofStore;
use nomic::orga::plugins::{ABCICall, ABCIPlugin};
use nomic::orga::query::Query;
use nomic::orga::state::State;
use nomic::orga::store::Store;
use nomic::orga::store::{BackingStore, Shared};
use nomic::orga::{Error, Result};
use std::cell::RefCell;
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::XmlHttpRequest;

use web_sys::{Request, RequestInit, RequestMode, Response};

const REST_PORT: u64 = 8443;

#[derive(Default)]
pub struct WebClient {
    height: RefCell<Option<u32>>,
}

impl WebClient {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Clone for WebClient {
    fn clone(&self) -> Self {
        WebClient {
            height: self.height.clone(),
        }
    }
}

impl<T: App + Call + Query + State + Default> Transport<ABCIPlugin<T>> for WebClient {
    fn call(&self, call: <ABCIPlugin<T> as Call>::Call) -> Result<()> {
        todo!()
        // TODO: shouldn't need to deal with ABCIPlugin at this level
        // let call = match call {
        //     ABCICall::DeliverTx(call) => call,
        //     _ => return Err(Error::Client("Unexpected call type".into())),
        // };
        // let call_bytes = call.encode()?;
        // let tx = base64::encode(&call_bytes);
        // // let res = block_on(self.client.broadcast_tx_commit(call_bytes))?;

        // let window = match web_sys::window() {
        //     Some(window) => window,
        //     None => return Err(Error::App("Window not found".to_string())),
        // };

        // let storage = window
        //     .local_storage()
        //     .map_err(|_| Error::App("Could not get local storage".into()))?
        //     .unwrap();
        // let rest_server = storage
        //     .get("nomic/rest_server")
        //     .map_err(|_| Error::App("Could not load from local storage".into()))?
        //     .unwrap();

        // let url = format!("{}/txs", rest_server);

        // // let request = Request::new_with_str_and_init(&url, &opts)
        // //     .map_err(|e| Error::App(format!("{:?}", e)))?;

        // // let resp_value = JsFuture::from(window.fetch_with_request(&request))
        // //     .await
        // //     .map_err(|e| Error::App(format!("{:?}", e)))?;

        // // let res: Response = resp_value
        // //     .dyn_into()
        // //     .map_err(|e| Error::App(format!("{:?}", e)))?;
        // // let res = JsFuture::from(
        // //     res.array_buffer()
        // //         .map_err(|e| Error::App(format!("{:?}", e)))?,
        // // )
        // // .await
        // // .map_err(|e| Error::App(format!("{:?}", e)))?;
        // let client = reqwest_wasm::blocking::Client::new();
        // let res = client
        //     .post(url)
        //     .body(tx)
        //     .send()
        //     .map_err(|e| Error::App(format!("{:?}", e)))?
        //     .text()
        //     .map_err(|e| Error::App(format!("{:?}", e)))?;
        // // let res = js_sys::Uint8Array::new(&res).to_vec();
        // // let res = String::from_utf8(res).map_err(|e| Error::App(format!("{:?}", e)))?;

        // #[cfg(feature = "logging")]
        // web_sys::console::log_1(&format!("response: {}", &res).into());

        // self.last_res
        //     .lock()
        //     .map_err(|e| Error::App(format!("{:?}", e)))?
        //     .replace(res);

        // // if let tendermint::abci::Code::Err(code) = res.check_tx.code {
        // //     let msg = format!("code {}: {}", code, res.check_tx.log);
        // //     return Err(Error::Call(msg));
        // // }

        // Ok(())
    }

    fn query(&self, query: T::Query) -> Result<Store> {
        // spawn_local(async {
        let query_bytes = query.encode()?;
        let query = hex::encode(query_bytes);
        let maybe_height: Option<u32> = self.height.borrow().map(Into::into);

        let window = match web_sys::window() {
            Some(window) => window,
            None => return Err(Error::App("Window not found".to_string())),
        };

        let storage = window
            .local_storage()
            .map_err(|_| Error::App("Could not get local storage".into()))?
            .unwrap();
        let rest_server = storage
            .get("nomic/rest_server")
            .map_err(|_| Error::App("Could not load from local storage".into()))?
            .unwrap();

        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        let mut url = format!("{}/query/{}", rest_server, query);
        if let Some(height) = maybe_height {
            url.push_str(&format!("?height={}", height));
        }

        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| Error::App(format!("{:?}", e)))?;

        let req = XmlHttpRequest::new().unwrap();
        req.open_with_async("GET", &url, false)
            .map_err(|e| Error::App(format!("{:?}", e)))?;

        req.send().map_err(|e| Error::App(format!("{:?}", e)))?;

        let res = req
            .response_text()
            .map_err(|e| Error::App(format!("{:?}", e)))?
            .unwrap();

        let res = base64::decode(res).map_err(|e| Error::App(format!("{:?}", e)))?;

        // TODO: we shouldn't need to include the root hash in the result, it
        // should come from a trusted source
        let res_height = match res[0..4].try_into() {
            Ok(inner) => u32::from_be_bytes(inner),
            _ => panic!("Cannot convert result to fixed size array"),
        };
        if let Some(height) = self.height.borrow().as_ref() {
            if *height != res_height {
                return Err(Error::App(format!(
                    "Height mismatch: expected {}, got {}",
                    height, res_height
                )));
            }
        }
        self.height.replace(Some(res_height));
        let root_hash = match res[4..36].try_into() {
            Ok(inner) => inner,
            _ => panic!("Cannot convert result to fixed size array"),
        };
        let proof_bytes = &res[36..];

        let map = nomic::orga::merk::merk::proofs::query::verify(proof_bytes, root_hash)?;

        let store: Shared<ProofStore> = Shared::new(ProofStore(map));
        let store = Store::new(BackingStore::ProofMap(store));

        Ok(store)
    }
}
