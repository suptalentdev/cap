use ic_kit::candid::candid_method;
use ic_kit::candid::CandidType;
use ic_kit::interfaces::{management, Method};
use ic_kit::{ic, Principal};
use serde::Deserialize;

// It's ok.
use ic_history_common::*;
use ic_kit::macros::*;

#[cfg(debug_cfg)]
const WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/debug/ic_history_root-deb-opt.wasm");

#[cfg(not(debug_cfg))]
const WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/ic_history_root-rel-opt.wasm");

#[update]
#[candid_method(update)]
async fn install_bucket_code(canister_id: RootBucketId) {
    use management::{CanisterStatus, InstallMode, WithCanisterId};

    let (response,) = CanisterStatus::perform(
        Principal::management_canister(),
        (WithCanisterId { canister_id },),
    )
    .await
    .expect("Failed to retrieve canister status");

    if response.settings.controllers.len() > 1 {
        panic!("Expected one controller on canister {}", canister_id);
    }

    if response.module_hash.is_some() {
        panic!(
            "Expected an empty canister. Canister {} already has an installed wasm on it.",
            canister_id
        );
    }

    #[derive(CandidType, Deserialize)]
    struct InstallCodeArgumentBorrowed<'a> {
        mode: InstallMode,
        canister_id: Principal,
        #[serde(with = "serde_bytes")]
        wasm_module: &'a [u8],
        arg: Vec<u8>,
    }

    let install_config = InstallCodeArgumentBorrowed {
        mode: InstallMode::Install,
        canister_id,
        wasm_module: WASM,
        arg: b" ".to_vec(),
    };

    let _: () = ic::call(
        Principal::management_canister(),
        "install_code",
        (install_config,),
    )
    .await
    .expect("Install code failed.");
}
