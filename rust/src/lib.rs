// Copyright 2019 Ivan Sorokin.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


// This code is mostly based on Ivan Sorokin's work in IronBelly. Original copyright has been retained.

use grin_core::global::ChainTypes;
use grin_keychain::ExtKeychain;
use grin_util::file::get_first_line;
use grin_util::Mutex;
use grin_wallet::libwallet::api::{APIForeign, APIOwner};
use grin_wallet::libwallet::types::{NodeClient, WalletInst};
use grin_wallet::{
    instantiate_wallet, FileWalletCommAdapter, HTTPNodeClient, LMDBBackend, WalletConfig,
    WalletSeed, HTTPWalletCommAdapter,
};
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

fn c_str_to_rust(s: *const c_char) -> String {
    unsafe { CStr::from_ptr(s).to_string_lossy().into_owned() }
}

#[no_mangle]
pub unsafe extern "C" fn cstr_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    CString::from_raw(s);
}

pub fn get_wallet_config(wallet_dir: &str, chain_type: &str, check_node_api_http_addr: &str) -> WalletConfig {
    let chain_type_config = match chain_type {
        "floonet" => ChainTypes::Floonet,
        "usernet" => ChainTypes::UserTesting,
        "mainnet" => ChainTypes::Mainnet,
        _ => ChainTypes::Mainnet,
    };
    WalletConfig {
        chain_type: Some(chain_type_config),
        api_listen_interface: "127.0.0.1".to_string(),
        api_listen_port: 13415,
        api_secret_path: Some(".api_secret".to_string()),
        node_api_secret_path: Some(wallet_dir.to_owned() + "/.api_secret"),
        check_node_api_http_addr: check_node_api_http_addr.to_string(),
        data_file_dir: wallet_dir.to_owned() + "/wallet_data",
        tls_certificate_file: None,
        tls_certificate_key: None,
        dark_background_color_scheme: Some(true),
        keybase_notify_ttl: Some(1),
        no_commit_cache: None,
        owner_api_include_foreign: None,
        owner_api_listen_port: Some(WalletConfig::default_owner_api_listen_port()),
    }
}

fn wallet_init(
    path: &str,
    chain_type: &str,
    password: &str,
    check_node_api_http_addr: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet_config = get_wallet_config(path, chain_type, check_node_api_http_addr);
    let node_api_secret = get_first_line(wallet_config.node_api_secret_path.clone());
    let seed = WalletSeed::init_file(&wallet_config, 16, None, &password)?;
    let client_n = HTTPNodeClient::new(
        &wallet_config.check_node_api_http_addr,
        node_api_secret.clone(),
    );
    let _: LMDBBackend<HTTPNodeClient, ExtKeychain> =
        LMDBBackend::new(wallet_config.clone(), &password, client_n)?;
    seed.to_mnemonic()
}

macro_rules! unwrap_to_c (
	($func:expr, $error:expr) => (
	match $func {
        Ok(res) => {
            *$error = 0;
            CString::new(res.to_owned()).unwrap().into_raw()
        }
        Err(e) => {
            *$error = 1;
            CString::new(
                serde_json::to_string(&format!("{}",e)).unwrap()).unwrap().into_raw()
        }
    }
));

#[no_mangle]
pub unsafe extern "C" fn grin_wallet_init(
    path: *const c_char,
    chain_type: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        wallet_init(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
        ),
        error
    )
}

fn wallet_recovery(
    path: &str,
    chain_type: &str,
    phrase: &str,
    password: &str,
    check_node_api_http_addr: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet_config = get_wallet_config(path, chain_type, check_node_api_http_addr);
    let node_api_secret = get_first_line(wallet_config.node_api_secret_path.clone());
    let _res = WalletSeed::recover_from_phrase(&wallet_config, &phrase, &password)?;
    let node_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, node_api_secret);
    let wallet = instantiate_wallet(wallet_config.clone(), node_client, password, "default")?;
    let mut api = APIOwner::new(wallet.clone());
    match api.restore() {
        Ok(_) => Ok("".to_owned()),
        Err(e) => Err(grin_wallet::Error::from(e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn grin_wallet_recovery(
    path: *const c_char,
    chain_type: *const c_char,
    phrase: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        wallet_recovery(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(phrase),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
        ),
        error
    )
}

fn wallet_phrase(
    path: &str,
    chain_type: &str,
    password: &str,
    check_node_api_http_addr: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet_config = get_wallet_config(path, chain_type, check_node_api_http_addr);
    let seed = WalletSeed::from_file(&wallet_config, &password)?;
    seed.to_mnemonic()
}

#[no_mangle]
pub unsafe extern "C" fn grin_wallet_phrase(
    path: *const c_char,
    chain_type: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        wallet_phrase(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
        ),
        error
    )
}

fn get_wallet(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
) -> Result<Arc<Mutex<WalletInst<impl NodeClient, ExtKeychain>>>, grin_wallet::Error> {
    let wallet_config = get_wallet_config(path, chain_type, check_node_api_http_addr);
    let node_api_secret = get_first_line(wallet_config.node_api_secret_path.clone());

    let node_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, node_api_secret);
    instantiate_wallet(wallet_config.clone(), node_client, password, account)
}

fn tx_get(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    refresh_from_node: bool,
    tx_id: u32,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let api = APIOwner::new(wallet.clone());
    let txs = api.retrieve_txs(refresh_from_node, Some(tx_id), None)?;
    Ok(serde_json::to_string(&txs).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_get(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    refresh_from_node: bool,
    tx_id: u32,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_get(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            refresh_from_node,
            tx_id,
        ),
        error
    )
}

fn txs_get(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    refresh_from_node: bool,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let api = APIOwner::new(wallet.clone());

    match api.retrieve_txs(refresh_from_node, None, None) {
        Ok(txs) => Ok(serde_json::to_string(&txs).unwrap()),
        Err(e) => Err(grin_wallet::Error::from(e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn grin_txs_get(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    refresh_from_node: bool,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        txs_get(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            refresh_from_node,
        ),
        error
    )
}

fn balance(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    refresh_from_node: bool,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    let (_validated, wallet_info) = api.retrieve_summary_info(refresh_from_node, 1)?;
    Ok(serde_json::to_string(&wallet_info).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn grin_balance(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    refresh_from_node: bool,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        balance(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            refresh_from_node,
        ),
        error
    )
}

#[derive(Serialize, Deserialize)]
struct Strategy {
    selection_strategy_is_use_all: bool,
    total: u64,
    fee: u64,
}

fn tx_strategies(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    amount: u64,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    let mut result = vec![];
    if let Ok(smallest) = api.estimate_initiate_tx(None, amount, 1, 1, false) {
        result.push(Strategy {
            selection_strategy_is_use_all: false,
            total: smallest.0,
            fee: smallest.1,
        })
    }
    match api.estimate_initiate_tx(None, amount, 1, 1, true) {
        Ok(all) => {
            result.push(Strategy {
                selection_strategy_is_use_all: true,
                total: all.0,
                fee: all.1,
            });
            Ok(serde_json::to_string(&result).unwrap())
        }
        Err(e) => Err(grin_wallet::Error::from(e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_strategies(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    amount: u64,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_strategies(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            amount,
        ),
        error
    )
}

fn tx_create(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    message: &str,
    amount: u64,
    selection_strategy_is_use_all: bool,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    let (slate, lock_fn) = api.initiate_tx(
        None,
        amount,
        1,
        1,
        selection_strategy_is_use_all,
        Some(message.to_owned()),
    )?;
    api.tx_lock_outputs(&slate, lock_fn)?;
    Ok(serde_json::to_string(&slate).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_create(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    amount: u64,
    selection_strategy_is_use_all: bool,
    message: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_create(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            &c_str_to_rust(message),
            amount,
            selection_strategy_is_use_all,
        ),
        error
    )
}

fn tx_cancel(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    id: u32,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    api.cancel_tx(Some(id), None)?;
    Ok("".to_owned())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_cancel(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    id: u32,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_cancel(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            id,
        ),
        error
    )
}

fn tx_receive(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    slate_path: &str,
    message: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIForeign::new(wallet.clone());
    let adapter = FileWalletCommAdapter::new();
    let mut slate = adapter.receive_tx_async(&slate_path)?;
    api.verify_slate_messages(&slate)?;
    api.receive_tx(&mut slate, Some(account), Some(message.to_owned()))?;
    Ok(serde_json::to_string(&slate).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_receive(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    slate_path: *const c_char,
    message: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_receive(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            &c_str_to_rust(slate_path),
            &c_str_to_rust(message),
        ),
        error
    )
}

fn tx_finalize(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    slate_path: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    let adapter = FileWalletCommAdapter::new();
    let mut slate = adapter.receive_tx_async(&slate_path)?;
    api.verify_slate_messages(&slate)?;
    api.finalize_tx(&mut slate)?;
    api.post_tx(&slate.tx, true)?;
    Ok("".to_owned())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_finalize(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    slate_path: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_finalize(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            &c_str_to_rust(slate_path),
        ),
        error
    )
}

fn tx_send(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    amount: u64,
    selection_strategy_is_use_all: bool,
    message: &str,
    dest: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    let (mut slate, lock_fn) = api.initiate_tx(
        None,
        amount,
        1,
        1,
        selection_strategy_is_use_all,
        Some(message.to_owned()),
    )?;
    let adapter =  HTTPWalletCommAdapter::new();
    slate = adapter.send_tx_sync(dest, &slate)?;
    api.tx_lock_outputs(&slate, lock_fn)?;
    api.verify_slate_messages(&slate)?;
    api.finalize_tx(&mut slate)?;
    api.post_tx(&slate.tx, true)?;
    Ok("".to_owned())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_send(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    amount: u64,
    selection_strategy_is_use_all: bool,
    message: *const c_char,
    dest: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_send(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            amount,
            selection_strategy_is_use_all,
            &c_str_to_rust(message),
            &c_str_to_rust(dest),
        ),
        error
    )
}

fn tx_repost(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
    tx_id: u32,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let api = APIOwner::new(wallet.clone());
    let (_, txs) = api.retrieve_txs(true, Some(tx_id), None)?;
    let stored_tx = api.get_stored_tx(&txs[0])?;
    if stored_tx.is_none() {
        return Ok("".to_owned());
    }
    if txs[0].confirmed {    
        return Ok("".to_owned());
    }
    api.post_tx(&stored_tx.unwrap(), true)?;
    Ok("".to_owned())
}

#[no_mangle]
pub unsafe extern "C" fn grin_tx_repost(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    tx_id: u32,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        tx_repost(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
            tx_id,
        ),
        error
    )
}

fn wallet_restore(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    match api.restore() {
        Ok(_) => Ok("".to_owned()),
        Err(e) => Err(grin_wallet::Error::from(e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn grin_wallet_restore(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        wallet_restore(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
        ),
        error
    )
}

fn wallet_check(
    path: &str,
    chain_type: &str,
    account: &str,
    password: &str,
    check_node_api_http_addr: &str,
) -> Result<String, grin_wallet::Error> {
    let wallet = get_wallet(path, chain_type, account, password, check_node_api_http_addr)?;
    let mut api = APIOwner::new(wallet.clone());
    match api.check_repair() {
        Ok(_) => Ok("".to_owned()),
        Err(e) => Err(grin_wallet::Error::from(e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn grin_wallet_check(
    path: *const c_char,
    chain_type: *const c_char,
    account: *const c_char,
    password: *const c_char,
    check_node_api_http_addr: *const c_char,
    error: *mut u8,
) -> *const c_char {
    unwrap_to_c!(
        wallet_check(
            &c_str_to_rust(path),
            &c_str_to_rust(chain_type),
            &c_str_to_rust(account),
            &c_str_to_rust(password),
            &c_str_to_rust(check_node_api_http_addr),
        ),
        error
    )
}






