use yew::prelude::*;
use yewdux::prelude::*;
use crate::components::context::AppState;
use yew::platform::spawn_local;
use web_sys::console;
use crate::requests::setting_reqs::{call_mfa_settings, call_save_mfa_secret};
use std::borrow::Borrow;
use otpauth::TOTP;
use qrcode::QrCode;
use base64::encode;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use wasm_bindgen::JsCast;
// use std::time::{SystemTime, UNIX_EPOCH};
use js_sys::Date;
use qrcode_png::QrCodeEcc;
use qrcode::Color;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use data_encoding::BASE32;
use qrcode::render::svg;
use crate::components::gen_funcs::verify_totp_code;
use rand::{RngCore, rngs::OsRng};

pub fn generate_totp_secret() -> String {
    let mut secret = [0u8; 32]; // 256-bit secret
    OsRng.fill_bytes(&mut secret);
    BASE32.encode(&secret)
}

pub fn generate_qr_code(email: &str, issuer: &str, secret: String) -> Result<String, Box<dyn std::error::Error>> {
    // let secret = generate_totp_secret();

    // Initialize TOTP with your secret key
    let totp = TOTP::new(secret);

    // Get the base32 encoded secret
    let secret_base32 = totp.base32_secret();

    // Construct the provisioning URL according to the otpauth protocol
    let provisioning_url = totp.to_uri(email, issuer);

    // Generate the QR code
    let qr = QrCode::new(provisioning_url.as_bytes())?;
    let svg = qr.render::<svg::Color>().build();

    // URL-encode the SVG data
    let encoded_svg = utf8_percent_encode(&svg, NON_ALPHANUMERIC).to_string();

    // Return the data URL for the SVG
    Ok(format!("data:image/svg+xml;utf8,{}", encoded_svg))
}

#[function_component(MFAOptions)]
pub fn mfa_options() -> Html {
    let (state, _dispatch) = use_store::<AppState>();
    let api_key = state.auth_details.as_ref().map(|ud| ud.api_key.clone());
    let _user_id = state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let server_name = state.auth_details.as_ref().map(|ud| ud.server_name.clone());
    let _error_message = state.error_message.clone();
    let email = state.user_details.as_ref().map(|ud| ud.Email.clone());
    let mfa_status = use_state(|| false);
    let code = use_state(|| "".to_string());


    {
        let mfa_status = mfa_status.clone();
        use_effect_with((api_key.clone(), server_name.clone()), move |(api_key, server_name)| {
            let mfa_status = mfa_status.clone();
            let api_key = api_key.clone();
            let server_name = server_name.clone();
            let user_id = _user_id.clone();
            let future = async move {
                if let (Some(api_key), Some(server_name)) = (api_key, server_name) {
                    let response = call_mfa_settings(server_name, api_key.unwrap(), user_id.unwrap()).await;
                    match response {
                        Ok(mfa_settings_response) => {
                            mfa_status.set(mfa_settings_response);
                        },
                        Err(e) => console::log_1(&format!("Error getting MFA status: {}", e).into()),
                    }
                }
            };
            spawn_local(future);
            // Return cleanup function
            || {}
        });
    }
    // let html_self_service = self_service_status.clone();
    let loading = use_state(|| false);

    // Define the state of the application
    #[derive(Clone, PartialEq)]
    enum PageState {
        Hidden,
        Setup,
    }

    // Define the initial state
    let page_state = use_state(|| PageState::Hidden);
    let mfa_code = use_state(|| String::new());
    let mfa_secret = use_state(|| String::new());


    // Define the function to close the modal
    let close_modal = {
        let page_state = page_state.clone();
        Callback::from(move |_| {
            page_state.set(PageState::Hidden);
        })
    };

    let open_setup_modal = {
        let mfa_code = mfa_code.clone();
        let page_state = page_state.clone();
        let mfa_secret = mfa_secret.clone();
        let email = email.clone();
        Callback::from(move |_| {
            let mfa_code = mfa_code.clone();
            let page_state = page_state.clone();
            let mfa_secret = mfa_secret.clone();
            let secret = generate_totp_secret();
            let email = email.clone();
            mfa_secret.set(secret.clone());
            // Assuming generate_qr_code_for_web is async, you might need to spawn a local future
            wasm_bindgen_futures::spawn_local(async move {
                let email = email; // Example email, use actual data
                let issuer = "Pinepods"; // Example issuer, use actual data
                match generate_qr_code(email.unwrap().unwrap().as_str(), issuer, secret.clone()) {
                    Ok(qr_code_base64) => {
                        mfa_code.set(qr_code_base64);
                        page_state.set(PageState::Setup); // Move to the setup page state
                    }
                    Err(e) => {
                        log::error!("Failed to generate QR code: {}", e);
                        // Handle error appropriately
                    }
                }
            });
        })
    };

    // Define the function to close the modal
    let verify_code = {
        let page_state = page_state.clone();
        let api_key = api_key.clone();
        let user_id = state.user_details.as_ref().map(|ud| ud.UserID.clone());
        let server_name = server_name.clone();
        let mfa_secret = mfa_secret.clone();
        let code = code.clone();

        Callback::from(move |_| {
            let api_key = api_key.clone();
            let user_id = user_id.clone();
            let server_name = server_name.clone();
            let page_state = page_state.clone();
            let mfa_secret = mfa_secret.clone();
            let code = code.clone();

            console::log_1(&"Verifying code".into());

            // Use the separate function to verify the code
            if verify_totp_code(&mfa_secret, &code) {
                console::log_1(&"Code verified successfully".into());
                // Proceed with action after successful verification
                wasm_bindgen_futures::spawn_local(async move {
                    match call_save_mfa_secret(&server_name.unwrap(), &api_key.unwrap().unwrap(), user_id.unwrap(), (*mfa_secret).clone()).await {
                        Ok(response) => {
                            console::log_1(&format!("MFA setup successful: {}", response.status).into());
                            page_state.set(PageState::Hidden);
                        },
                        Err(e) => {
                            console::log_1(&e.to_string().into());
                            // Handle error appropriately
                            page_state.set(PageState::Hidden);
                        },
                    }
                });
            } else {
                console::log_1(&"Invalid code".into());
                page_state.set(PageState::Hidden);
                // Handle invalid code
                // For example, you might want to display an error message to the user
            }
        })
    };


    let on_code_change = {
        let code = code.clone();
        Callback::from(move |e: InputEvent| {
            code.set(e.target_unchecked_into::<web_sys::HtmlInputElement>().value());
        })
    };

    let setup_mfa_modal = html! {
        <div id="setup-mfa-modal" tabindex="-1" aria-hidden="true" class="fixed top-0 right-0 left-0 z-50 flex justify-center items-center w-full h-[calc(100%-1rem)] max-h-full bg-black bg-opacity-25">
            <div class="relative p-4 w-full max-w-md max-h-full bg-white rounded-lg shadow dark:bg-gray-700">
                <div class="relative bg-white rounded-lg shadow dark:bg-gray-700">
                    <div class="flex flex-col items-start justify-between p-4 md:p-5 border-b rounded-t dark:border-gray-600">
                        <button onclick={close_modal.clone()} class="self-end text-gray-400 bg-transparent hover:bg-gray-200 hover:text-gray-900 rounded-lg text-sm w-8 h-8 ms-auto inline-flex justify-center items-center dark:hover:bg-gray-600 dark:hover:text-white">
                            <svg class="w-3 h-3" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 14 14">
                                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m1 1 6 6m0 0 6 6M7 7l6-6M7 7l-6 6"/>
                            </svg>
                            <span class="sr-only">{"Close modal"}</span>
                        </button>
                        <h3 class="text-xl font-semibold text-gray-900 dark:text-white">
                            {"Setup MFA"}
                        </h3>
                        <p class="text-m font-semibold text-gray-900 dark:text-white">
                        {"Either scan the QR code with your authenticator app or enter the code manually. Then enter the code from your authenticator app to verify."}
                        </p>

                        <div class="mt-4 bg-gray-100 p-4 rounded-md overflow-x-auto whitespace-nowrap max-w-full">
                            <img src={(*mfa_code).clone()} alt="QR Code" />
                        </div>
                        <div class="mt-4 bg-gray-100 p-4 rounded-md overflow-x-auto whitespace-nowrap max-w-full">
                            {(*mfa_secret).clone()}
                        </div>
                        <div>
                            <label for="fullname" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">{"Verify Code"}</label>
                            <input oninput={on_code_change} type="text" id="fullname" name="fullname" class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white" required=true />
                        </div>
                        <div class="flex justify-between">
                            <button onclick={verify_code.clone()} class="mt-4 bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" type="button">
                                {"Verify"}
                            </button>
                            <button onclick={close_modal.clone()} class="mt-4 bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" type="button">
                                {"Close"}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    };


    html! {
        <>
        {
            match *page_state {
            PageState::Setup => setup_mfa_modal,
            _ => html! {},
            }
        }
        <div class="p-4"> // You can adjust the padding as needed
            <p class="text-lg font-bold mb-4">{"MFA Options:"}</p>
            <p class="text-md mb-4">{"You can setup edit, or remove MFA for your account here. MFA will only be prompted when new authentication is needed."}</p> // Styled paragraph

            <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" disabled={**loading.borrow()} checked={**mfa_status.borrow()} class="sr-only peer" onclick={open_setup_modal.clone()} />
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                <span class="ms-3 text-sm font-medium text-gray-900 dark:text-gray-300">{"Enable MFA"}</span>
            </label>
        </div>
        </>
    }
}
