use super::app_drawer::App_drawer;
use super::gen_components::ContextButton;
use super::gen_components::{EpisodeTrait, Search_nav, UseScrollToTop};
use super::gen_funcs::{format_datetime, match_date_format, parse_date};
use crate::components::audio::{on_play_click, AudioPlayer};
use crate::components::context::{AppState, UIState};
use crate::components::gen_funcs::format_time;
use crate::components::gen_funcs::{
    convert_time_to_seconds, sanitize_html_with_blank_target, truncate_description,
};
use crate::requests::login_requests::use_check_authentication;
use crate::requests::pod_req::{
    call_add_podcast, call_adjust_skip_times, call_check_podcast, call_download_all_podcast,
    call_enable_auto_download, call_get_auto_download_status, call_get_auto_skip_times,
    call_get_podcast_id_from_ep, call_get_podcast_id_from_ep_name, call_remove_podcasts_name,
    AutoDownloadRequest, DownloadAllPodcastRequest, PodcastValues, RemovePodcastValuesName,
    SkipTimesRequest,
};
use htmlentity::entity::decode;
use htmlentity::entity::ICodedDataTrait;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Event, HtmlInputElement, MouseEvent};
use yew::prelude::*;
use yew::Properties;
use yew::{
    function_component, html, use_effect, use_effect_with, use_node_ref, Callback, Html, TargetCast,
};
use yew_router::history::{BrowserHistory, History};
use yewdux::prelude::*;

fn add_icon() -> Html {
    html! {
        <span class="material-icons">{ "add_box" }</span>
    }
}

fn trash_icon() -> Html {
    html! {
        <span class="material-icons">{ "delete" }</span>

    }
}
fn settings_icon() -> Html {
    html! {
        <span class="material-icons">{ "more_vert" }</span>

    }
}
fn download_icon() -> Html {
    html! {
        <span class="material-icons">{ "download_for_offline" }</span>

    }
}
fn no_icon() -> Html {
    html! {}
}

#[allow(dead_code)]
fn play_icon() -> Html {
    html! {
    <svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 -960 960 960" width="24"><path d="m380-300 280-180-280-180v360ZM480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm0-80q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>
        }
}

#[allow(dead_code)]
fn pause_icon() -> Html {
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 -960 960 960" width="24"><path d="M360-320h80v-320h-80v320Zm160 0h80v-320h-80v320ZM480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm0-80q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub html: String,
}

#[function_component(SafeHtml)]
pub fn safe_html(props: &Props) -> Html {
    let div = gloo_utils::document().create_element("div").unwrap();
    div.set_inner_html(&props.html.clone());

    Html::VRef(div.into())
}

fn sanitize_html(html: &str) -> String {
    let cleaned_html = ammonia::clean(html);
    let decoded_data = decode(cleaned_html.as_bytes());
    match decoded_data.to_string() {
        Ok(decoded_html) => decoded_html,
        Err(_) => String::from("Invalid HTML content"),
    }
}

pub enum AppStateMsg {
    ExpandEpisode(String),
    CollapseEpisode(String),
}

impl Reducer<AppState> for AppStateMsg {
    fn apply(self, mut state: Rc<AppState>) -> Rc<AppState> {
        let state_mut = Rc::make_mut(&mut state);

        match self {
            AppStateMsg::ExpandEpisode(guid) => {
                state_mut.expanded_descriptions.insert(guid);
            }
            AppStateMsg::CollapseEpisode(guid) => {
                state_mut.expanded_descriptions.remove(&guid);
            }
        }

        // Return the Rc itself, not a reference to it
        state
    }
}

pub enum UIStateMsg {
    ClearErrorMessage,
    ClearInfoMessage,
}

impl Reducer<UIState> for UIStateMsg {
    fn apply(self, mut state: Rc<UIState>) -> Rc<UIState> {
        let state = Rc::make_mut(&mut state);

        match self {
            UIStateMsg::ClearErrorMessage => {
                state.error_message = None;
            }
            UIStateMsg::ClearInfoMessage => {
                state.info_message = None;
            }
        }

        (*state).clone().into()
    }
}

#[function_component(EpisodeLayout)]
pub fn episode_layout() -> Html {
    let is_added = use_state(|| false);
    let (state, _dispatch) = use_store::<UIState>();
    let (search_state, _search_dispatch) = use_store::<AppState>();
    let podcast_feed_results = search_state.podcast_feed_results.clone();
    let clicked_podcast_info = search_state.clicked_podcast_info.clone();
    let episode_name_pre: Option<String> = search_state
        .podcast_feed_results
        .as_ref()
        .and_then(|results| results.episodes.get(0))
        .and_then(|episode| episode.title.clone());
    let episode_url_pre: Option<String> = search_state
        .podcast_feed_results
        .as_ref()
        .and_then(|results| results.episodes.get(0))
        .and_then(|episode| episode.enclosure_url.clone());

    let history = BrowserHistory::new();
    // let node_ref = use_node_ref();
    let user_id = search_state
        .user_details
        .as_ref()
        .map(|ud| ud.UserID.clone());
    let api_key = search_state
        .auth_details
        .as_ref()
        .map(|ud| ud.api_key.clone());
    let server_name = search_state
        .auth_details
        .as_ref()
        .map(|ud| ud.server_name.clone());

    let session_dispatch = _search_dispatch.clone();
    let session_state = search_state.clone();
    let podcast_added = search_state.podcast_added.unwrap_or_default();

    use_effect_with((), move |_| {
        // Check if the page reload action has already occurred to prevent redundant execution
        if session_state.reload_occured.unwrap_or(false) {
            // Logic for the case where reload has already been processed
        } else {
            // Normal effect logic for handling page reload
            let window = web_sys::window().expect("no global `window` exists");
            let performance = window.performance().expect("should have performance");
            let navigation_type = performance.navigation().type_();

            if navigation_type == 1 {
                // 1 stands for reload
                let session_storage = window.session_storage().unwrap().unwrap();
                session_storage
                    .set_item("isAuthenticated", "false")
                    .unwrap();
            }

            // Always check authentication status
            let current_route = window.location().href().unwrap_or_default();
            use_check_authentication(session_dispatch.clone(), &current_route);

            // Mark that the page reload handling has occurred
            session_dispatch.reduce_mut(|state| {
                state.reload_occured = Some(true);
                state.clone() // Return the modified state
            });
        }

        || ()
    });

    // On mount, check if the podcast is in the database
    let effect_user_id = user_id.unwrap().clone();
    let effect_api_key = api_key.clone();

    {
        let is_added = is_added.clone();
        let podcast = clicked_podcast_info.clone();
        let user_id = effect_user_id.clone();
        let api_key = effect_api_key.clone();
        let server_name = server_name.clone();

        use_effect_with(&(), move |_| {
            let is_added = is_added.clone();
            let podcast = podcast.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let added = call_check_podcast(
                    &server_name.unwrap(),
                    &api_key.unwrap().unwrap(),
                    user_id,
                    podcast.clone().unwrap().podcast_title.as_str(),
                    podcast.clone().unwrap().podcast_url.as_str(),
                )
                .await
                .unwrap_or_default()
                .exists;
                is_added.set(added);
            });
            || ()
        });
    }

    let download_status = use_state(|| false);
    let podcast_id = use_state(|| 0);
    let start_skip = use_state(|| 0);
    let end_skip = use_state(|| 0);

    {
        let api_key = api_key.clone();
        let server_name = server_name.clone();
        let podcast_id = podcast_id.clone();
        let download_status = download_status.clone();
        let episode_name = episode_name_pre.clone();
        let episode_url = episode_url_pre.clone();
        let user_id = search_state.user_details.as_ref().map(|ud| ud.UserID);
        let effect_start_skip = start_skip.clone();
        let effect_end_skip = end_skip.clone();
        let effect_added = is_added.clone();

        use_effect_with(effect_added.clone(), move |_| {
            let bool_true = *effect_added; // Dereference here
            if bool_true {
                let api_key = api_key.clone();
                let server_name = server_name.clone();
                let podcast_id = podcast_id.clone();
                let download_status = download_status.clone();
                let episode_name = episode_name;
                let episode_url = episode_url;
                let user_id = user_id.unwrap();

                wasm_bindgen_futures::spawn_local(async move {
                    if let (Some(api_key), Some(server_name)) =
                        (api_key.as_ref(), server_name.as_ref())
                    {
                        match call_get_podcast_id_from_ep_name(
                            &server_name,
                            &api_key,
                            episode_name.unwrap(),
                            episode_url.unwrap(),
                            user_id,
                        )
                        .await
                        {
                            Ok(id) => {
                                podcast_id.set(id);

                                match call_get_auto_download_status(
                                    &server_name,
                                    user_id,
                                    &Some(api_key.clone().unwrap()),
                                    id,
                                )
                                .await
                                {
                                    Ok(status) => {
                                        download_status.set(status);
                                    }
                                    Err(e) => {
                                        web_sys::console::log_1(
                                            &format!("Error getting auto-download status: {}", e)
                                                .into(),
                                        );
                                    }
                                }
                                match call_get_auto_skip_times(
                                    &server_name,
                                    &Some(api_key.clone().unwrap()),
                                    user_id,
                                    id,
                                )
                                .await
                                {
                                    Ok((start, end)) => {
                                        effect_start_skip.set(start);
                                        effect_end_skip.set(end);
                                    }
                                    Err(e) => {
                                        web_sys::console::log_1(
                                            &format!("Error getting auto-skip times: {}", e).into(),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                web_sys::console::log_1(
                                    &format!("Error getting podcast ID: {}", e).into(),
                                );
                            }
                        }
                    }
                });
            }
            || ()
        });
    }

    // Function to handle link clicks
    let handle_click = Callback::from(move |event: MouseEvent| {
        if let Some(target) = event.target_dyn_into::<web_sys::HtmlElement>() {
            if let Some(href) = target.get_attribute("href") {
                event.prevent_default();
                if href.starts_with("http") {
                    // External link, open in a new tab
                    web_sys::window()
                        .unwrap()
                        .open_with_url_and_target(&href, "_blank")
                        .unwrap();
                } else {
                    // Internal link, use Yew Router to navigate
                    history.push(href);
                }
            }
        }
    });

    let node_ref = use_node_ref();

    use_effect_with((), move |_| {
        if let Some(container) = node_ref.cast::<web_sys::HtmlElement>() {
            if let Ok(links) = container.query_selector_all("a") {
                for i in 0..links.length() {
                    if let Some(link) = links.item(i) {
                        let link = link.dyn_into::<web_sys::HtmlElement>().unwrap();
                        let handle_click_clone = handle_click.clone();
                        let listener =
                            gloo_events::EventListener::new(&link, "click", move |event| {
                                handle_click_clone
                                    .emit(event.clone().dyn_into::<web_sys::MouseEvent>().unwrap());
                            });
                        listener.forget(); // Prevent listener from being dropped
                    }
                }
            }
        }

        || ()
    });

    {
        let dispatch = _dispatch.clone();
        use_effect(move || {
            let window = window().unwrap();
            let document = window.document().unwrap();

            let closure = Closure::wrap(Box::new(move |_event: Event| {
                dispatch.apply(UIStateMsg::ClearErrorMessage);
                dispatch.apply(UIStateMsg::ClearInfoMessage);
            }) as Box<dyn Fn(_)>);

            document
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();

            // Return cleanup function
            move || {
                document
                    .remove_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget(); // Prevents the closure from being dropped
            }
        });
    }

    let toggle_podcast = {
        let add_dispatch = _dispatch.clone();
        let pod_values = clicked_podcast_info.clone();

        let pod_title_og = pod_values.clone().unwrap().podcast_title.clone();
        let pod_artwork_og = pod_values.clone().unwrap().podcast_artwork.clone();
        let pod_author_og = pod_values.clone().unwrap().podcast_author.clone();
        let categories_og = pod_values
            .clone()
            .unwrap()
            .podcast_categories
            .unwrap()
            .clone();
        let pod_description_og = pod_values.clone().unwrap().podcast_description.clone();
        let pod_episode_count_og = pod_values.clone().unwrap().podcast_episode_count.clone();
        let pod_feed_url_og = pod_values.clone().unwrap().podcast_url.clone();
        let pod_website_og = pod_values.clone().unwrap().podcast_link.clone();
        let pod_explicit_og = pod_values.clone().unwrap().podcast_explicit.clone();
        let user_id_og = user_id.unwrap().clone();

        let api_key_clone = api_key.clone();
        let server_name_clone = server_name.clone();
        let user_id_clone = user_id.clone();
        let app_dispatch = _search_dispatch.clone();

        let is_added = is_added.clone();

        if *is_added == true {
            Callback::from(move |_: MouseEvent| {
                app_dispatch.reduce_mut(|state| state.is_loading = Some(true));
                let is_added_inner = is_added.clone();
                let call_dispatch = add_dispatch.clone();
                let pod_title = pod_title_og.clone();
                let pod_feed_url = pod_feed_url_og.clone();
                let user_id = user_id_og.clone();
                let podcast_values = RemovePodcastValuesName {
                    podcast_name: pod_title,
                    podcast_url: pod_feed_url,
                    user_id: user_id,
                };
                let api_key_call = api_key_clone.clone();
                let server_name_call = server_name_clone.clone();
                let app_dispatch = app_dispatch.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let dispatch_wasm = call_dispatch.clone();
                    let api_key_wasm = api_key_call.clone().unwrap();
                    let server_name_wasm = server_name_call.clone();
                    let pod_values_clone = podcast_values.clone(); // Make sure you clone the podcast values
                    match call_remove_podcasts_name(
                        &server_name_wasm.unwrap(),
                        &api_key_wasm,
                        &pod_values_clone,
                    )
                    .await
                    {
                        Ok(success) => {
                            if success {
                                dispatch_wasm.reduce_mut(|state| {
                                    state.info_message =
                                        Option::from("Podcast successfully removed".to_string())
                                });
                                app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                                is_added_inner.set(false);
                            } else {
                                dispatch_wasm.reduce_mut(|state| {
                                    state.error_message =
                                        Option::from("Failed to add podcast".to_string())
                                });
                                app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                            }
                        }
                        Err(e) => {
                            dispatch_wasm.reduce_mut(|state| {
                                state.error_message =
                                    Option::from(format!("Error adding podcast: {:?}", e))
                            });
                            app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                        }
                    }
                });
            })
        } else {
            Callback::from(move |_: MouseEvent| {
                // Ensure this is triggered only by a MouseEvent
                let app_dispatch = app_dispatch.clone();
                app_dispatch.reduce_mut(|state| state.is_loading = Some(true));
                let is_added_inner = is_added.clone();
                let call_dispatch = add_dispatch.clone();
                let pod_title = pod_title_og.clone();
                let pod_artwork = pod_artwork_og.clone();
                let pod_author = pod_author_og.clone();
                let categories = categories_og.clone();
                let pod_description = pod_description_og.clone();
                let pod_episode_count = pod_episode_count_og.clone();
                let pod_feed_url = pod_feed_url_og.clone();
                let pod_website = pod_website_og.clone();
                let pod_explicit = pod_explicit_og.clone();
                let user_id = user_id_og.clone();
                let podcast_values = PodcastValues {
                    pod_title,
                    pod_artwork,
                    pod_author,
                    categories,
                    pod_description,
                    pod_episode_count,
                    pod_feed_url,
                    pod_website,
                    pod_explicit,
                    user_id,
                };
                let api_key_call = api_key_clone.clone();
                let server_name_call = server_name_clone.clone();
                let user_id_call = user_id_clone.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let dispatch_wasm = call_dispatch.clone();
                    let api_key_wasm = api_key_call.clone().unwrap();
                    let user_id_wasm = user_id_call.clone().unwrap();
                    let server_name_wasm = server_name_call.clone();
                    let pod_values_clone = podcast_values.clone(); // Make sure you clone the podcast values

                    match call_add_podcast(
                        &server_name_wasm.unwrap(),
                        &api_key_wasm,
                        user_id_wasm,
                        &pod_values_clone,
                    )
                    .await
                    {
                        Ok(success) => {
                            if success {
                                dispatch_wasm.reduce_mut(|state| {
                                    state.info_message =
                                        Option::from("Podcast successfully added".to_string())
                                });
                                app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                                is_added_inner.set(true);
                            } else {
                                dispatch_wasm.reduce_mut(|state| {
                                    state.error_message =
                                        Option::from("Failed to add podcast".to_string())
                                });
                                app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                            }
                        }
                        Err(e) => {
                            dispatch_wasm.reduce_mut(|state| {
                                state.error_message =
                                    Option::from(format!("Error adding podcast: {:?}", e))
                            });
                            app_dispatch.reduce_mut(|state| state.is_loading = Some(false));
                        }
                    }
                });
            })
        }
    };

    let download_server_name = server_name.clone();
    let download_api_key = api_key.clone();
    let download_dispatch = _dispatch.clone();
    let app_state = search_state.clone();

    let download_all_click = {
        let call_dispatch = download_dispatch.clone();
        let server_name_copy = download_server_name.clone();
        let api_key_copy = download_api_key.clone();
        let user_id_copy = user_id.unwrap();
        let search_call_state = app_state.clone();

        Callback::from(move |_: MouseEvent| {
            let server_name = server_name_copy.clone();
            let api_key = api_key_copy.clone();
            let search_state = search_call_state.clone();
            let call_down_dispatch = call_dispatch.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let episode_id = match search_state
                    .podcast_feed_results
                    .as_ref()
                    .and_then(|results| results.episodes.get(0))
                    .and_then(|episode| episode.episode_id)
                {
                    Some(id) => id,
                    None => {
                        eprintln!("No episode_id found");
                        return;
                    }
                };
                let ep_api_key = api_key.clone();
                let ep_server_name = server_name.clone();
                let ep_user_id = user_id_copy.clone();
                match call_get_podcast_id_from_ep(
                    &ep_server_name.unwrap(),
                    &ep_api_key.unwrap(),
                    episode_id,
                    ep_user_id,
                )
                .await
                {
                    Ok(podcast_id) => {
                        let request = DownloadAllPodcastRequest {
                            podcast_id: podcast_id,
                            user_id: user_id_copy,
                        };

                        match call_download_all_podcast(
                            &server_name.unwrap(),
                            &api_key.flatten(),
                            &request,
                        )
                        .await
                        {
                            Ok(success_message) => {
                                call_down_dispatch.reduce_mut(|state| {
                                    state.info_message =
                                        Option::from(format!("{}", success_message))
                                });
                            }
                            Err(e) => {
                                call_down_dispatch.reduce_mut(|state| {
                                    state.error_message = Option::from(format!("{}", e))
                                });
                            }
                        }
                    }
                    Err(e) => {
                        call_down_dispatch.reduce_mut(|state| {
                            state.error_message =
                                Option::from(format!("Failed to get podcast ID: {}", e))
                        });
                    }
                }
            });
        })
    };

    // Define the state of the application
    #[derive(Clone, PartialEq)]
    enum PageState {
        Hidden,
        Shown,
    }

    let button_content = if *is_added { trash_icon() } else { add_icon() };

    let setting_content = if *is_added {
        settings_icon()
    } else {
        no_icon()
    };
    let download_all = if *is_added {
        download_icon()
    } else {
        no_icon()
    };

    let page_state = use_state(|| PageState::Hidden);

    let on_close_modal = {
        let page_state = page_state.clone();
        Callback::from(move |_| {
            page_state.set(PageState::Hidden);
        })
    };

    let toggle_download = {
        let api_key = api_key.clone();
        let server_name = server_name.clone();
        let download_status = download_status.clone();
        let podcast_id = podcast_id.clone();
        let user_id = user_id.clone();

        Callback::from(move |_| {
            let api_key = api_key.clone();
            let server_name = server_name.clone();
            let download_status = download_status.clone();
            let auto_download = !*download_status;
            let pod_id_deref = *podcast_id.clone();
            let user_id = user_id.clone().unwrap();

            let request_data = AutoDownloadRequest {
                podcast_id: pod_id_deref, // Replace with the actual podcast ID
                user_id,
                auto_download,
            };

            wasm_bindgen_futures::spawn_local(async move {
                if let (Some(api_key), Some(server_name)) = (api_key.as_ref(), server_name.as_ref())
                {
                    match call_enable_auto_download(
                        &server_name,
                        &api_key.clone().unwrap(),
                        &request_data,
                    )
                    .await
                    {
                        Ok(_) => {
                            download_status.set(auto_download);
                        }
                        Err(e) => {
                            web_sys::console::log_1(
                                &format!("Error enabling/disabling downloads: {}", e).into(),
                            );
                        }
                    }
                }
            });
        })
    };

    let start_skip_call = start_skip.clone();
    let end_skip_call = end_skip.clone();
    let start_skip_call_button = start_skip.clone();
    let end_skip_call_button = end_skip.clone();
    let skip_dispatch = _dispatch.clone();

    // Save the skip times to the server
    let save_skip_times = {
        let start_skip = start_skip.clone();
        let end_skip = end_skip.clone();
        let api_key = api_key.clone();
        let user_id = user_id.clone();
        let server_name = server_name.clone();
        let podcast_id = podcast_id.clone();
        let skip_dispatch = skip_dispatch.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let skip_call_dispatch = skip_dispatch.clone();
            let start_skip = *start_skip;
            let end_skip = *end_skip;
            let api_key = api_key.clone();
            let user_id = user_id.clone().unwrap();
            let server_name = server_name.clone();
            let podcast_id = *podcast_id;

            wasm_bindgen_futures::spawn_local(async move {
                if let (Some(api_key), Some(server_name)) = (api_key.as_ref(), server_name.as_ref())
                {
                    let request = SkipTimesRequest {
                        podcast_id,
                        start_skip,
                        end_skip,
                        user_id,
                    };

                    match call_adjust_skip_times(&server_name, &api_key, &request).await {
                        Ok(_) => {
                            skip_call_dispatch.reduce_mut(|state| {
                                state.info_message = Option::from("Skip times Adjusted".to_string())
                            });
                        }
                        Err(e) => {
                            web_sys::console::log_1(
                                &format!("Error updating skip times: {}", e).into(),
                            );
                            skip_call_dispatch.reduce_mut(|state| {
                                state.error_message =
                                    Option::from("Error Adjusting Skip Times".to_string())
                            });
                        }
                    }
                }
            });
        })
    };

    // Define the modal components
    let podcast_option_model = html! {
        <div id="podcast_option_model" tabindex="-1" aria-hidden="true" class="fixed top-0 right-0 left-0 z-50 flex justify-center items-center w-full h-[calc(100%-1rem)] max-h-full bg-black bg-opacity-25">
            <div class="modal-container relative p-4 w-full max-w-md max-h-full rounded-lg shadow">
                <div class="modal-container relative rounded-lg shadow">
                    <div class="flex items-center justify-between p-4 md:p-5 border-b rounded-t">
                        <h3 class="text-xl font-semibold">
                            {"Podcast Options"}
                        </h3>
                        <button onclick={on_close_modal.clone()} class="end-2.5 text-gray-400 bg-transparent hover:bg-gray-200 hover:text-gray-900 rounded-lg text-sm w-8 h-8 ms-auto inline-flex justify-center items-center dark:hover:bg-gray-600 dark:hover:text-white">
                            <svg class="w-3 h-3" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 14 14">
                                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m1 1 6 6m0 0 6 6M7 7l6-6M7 7l-6 6"/>
                            </svg>
                            <span class="sr-only">{"Close modal"}</span>
                        </button>
                    </div>
                    <div class="p-4 md:p-5">
                        <form class="space-y-4" action="#">
                            <div>
                                <label for="download_schedule" class="block mb-2 text-sm font-medium">{"Download Future Episodes Automatically:"}</label>
                                <label class="inline-flex relative items-center cursor-pointer">
                                    <input type="checkbox" checked={*download_status} class="sr-only peer" onclick={toggle_download} />
                                    <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                                </label>
                            </div>
                            <div class="mt-4">
                                <label for="auto-skip" class="block mb-2 text-sm font-medium">{"Auto Skip Intros and Outros:"}</label>
                                <div class="flex items-center space-x-2">
                                    <div class="flex items-center space-x-2">
                                        <label for="start-skip" class="block text-sm font-medium">{"Start Skip (seconds):"}</label>
                                        <input
                                            type="number"
                                            id="start-skip"
                                            value={start_skip_call_button.to_string()}
                                            class="email-input border text-sm rounded-lg p-2.5 w-16"
                                            oninput={Callback::from(move |e: InputEvent| {
                                                if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                                    let value = input.value().parse::<i32>().unwrap_or(0);
                                                    start_skip_call.set(value);
                                                }
                                            })}
                                        />
                                    </div>
                                    <div class="flex items-center space-x-2">
                                        <label for="end-skip" class="block text-sm font-medium">{"End Skip (seconds):"}</label>
                                        <input
                                            type="number"
                                            id="end-skip"
                                            value={end_skip_call_button.to_string()}
                                            class="email-input border text-sm rounded-lg p-2.5 w-16"
                                            oninput={Callback::from(move |e: InputEvent| {
                                                if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                                    let value = input.value().parse::<i32>().unwrap_or(0);
                                                    end_skip_call.set(value);
                                                }
                                            })}
                                        />
                                    </div>
                                    <button
                                        class="download-button font-bold py-2 px-4 rounded"
                                        onclick={save_skip_times}
                                    >
                                        {"Confirm"}
                                    </button>
                                </div>
                            </div>
                            // <div>
                            //     <label for="tag-adjust" class="block mb-2 text-sm font-medium">{"Adjust Tags Associated with this Podcast"}</label>
                            //     <input placeholder="my_S3creT_P@$$" type="password" id="password" name="password" class="search-bar-input border text-sm rounded-lg block w-full p-2.5" required=true />
                            // </div>
                        </form>
                    </div>
                </div>
            </div>
        </div>
    };

    // Define the callback functions
    let toggle_settings = {
        let page_state = page_state.clone();
        Callback::from(move |_| {
            page_state.set(PageState::Shown);
        })
    };

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window)]
        fn toggle_description(guid: &str);
    }

    html! {
        <div class="main-container">
            <Search_nav />
            <UseScrollToTop />
            <h1 class="page_header text-2xl font-bold my-4 text-center">{ "Podcast Episode Results" }</h1>
            {
                match *page_state {
                PageState::Shown => podcast_option_model,
                _ => html! {},
                }
            }
        {
            if let Some(podcast_info) = clicked_podcast_info {
                let sanitized_title = podcast_info.podcast_title.replace(|c: char| !c.is_alphanumeric(), "-");
                let desc_id = format!("desc-{}", sanitized_title);
                // let toggle_description = {
                //     let desc_id = desc_id.clone();
                //     Callback::from(move |_: MouseEvent| {
                //         let desc_id = desc_id.clone();
                //         wasm_bindgen_futures::spawn_local(async move {
                //             let window = web_sys::window().expect("no global `window` exists");
                //             let function = window
                //                 .get("toggle_description")
                //                 .expect("should have `toggle_description` as a function")
                //                 .dyn_into::<js_sys::Function>()
                //                 .unwrap();
                //             let this = JsValue::NULL;
                //             let guid = JsValue::from_str(&desc_id);
                //             function.call1(&this, &guid).unwrap();
                //         });
                //     })
                // };



                let toggle_description = {
                    let desc_id = desc_id.clone();
                    Callback::from(move |_| {
                        let desc_id = desc_id.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let window = web_sys::window().expect("no global `window` exists");
                            let function = window
                                .get("toggle_description")
                                .expect("should have `toggle_description` as a function")
                                .dyn_into::<js_sys::Function>()
                                .unwrap();
                            let this = JsValue::NULL;
                            let guid = JsValue::from_str(&desc_id);
                            function.call1(&this, &guid).unwrap();
                        });
                    })
                };
                let sanitized_description = sanitize_html(&podcast_info.podcast_description);

                html! {
                    <div class="item-header">
                        <img src={podcast_info.podcast_artwork.clone()} alt={format!("Cover for {}", &podcast_info.podcast_title)} class="item-header-cover"/>
                        <div class="item-header-info">
                            <div class="title-button-container">
                                <h2 class="item-header-title">{ &podcast_info.podcast_title }</h2>
                                <button onclick={toggle_podcast} title="Click to add or remove podcast from feed" class={"item-container-button selector-button font-bold py-2 px-4 rounded-full self-center mr-8"} style="width: 60px; height: 60px;">
                                    { button_content }
                                </button>
                                <button onclick={toggle_settings} title="Click to setup podcast specific settings" class={"item-container-button selector-button font-bold py-2 px-4 rounded-full self-center mr-8"} style="width: 60px; height: 60px;">
                                    { setting_content }
                                </button>
                                <button onclick={download_all_click} title="Click to download all episodes for this podcast" class={"item-container-button selector-button font-bold py-2 px-4 rounded-full self-center mr-8"} style="width: 60px; height: 60px;">
                                    { download_all }
                                </button>
                            </div>

                            // <p class="item-header-description">{ &podcast_info.podcast_description }</p>
                            <div class="item-header-description desc-collapsed" id={desc_id.clone()} onclick={toggle_description.clone()}>
                                { sanitized_description }
                                <button class="toggle-desc-btn" onclick={toggle_description}>{ "" }</button>
                            </div>
                            <div class="item-header-info">
                                <p class="header-text">{ format!("Episode Count: {}", &podcast_info.podcast_episode_count) }</p>
                                <p class="header-text">{ format!("Authors: {}", &podcast_info.podcast_author) }</p>
                                <p class="header-text">{ format!("Explicit: {}", if podcast_info.podcast_explicit { "Yes" } else { "No" }) }</p>

                                <div>
                                    {
                                        if let Some(categories) = &podcast_info.podcast_categories {
                                            html! {
                                                for categories.values().map(|category_name| {
                                                    html! { <span class="category-box">{ category_name }</span> }
                                                })
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>

                            </div>
                        </div>
                    </div>
                }
            } else {
                html! {}
            }
        }
        {
                if let Some(results) = podcast_feed_results {
                    html! {
                        <div>
                            { for results.episodes.iter().map(|episode| {
                                let dispatch = _dispatch.clone();
                                let search_dispatch = _search_dispatch.clone();
                                let search_state_clone = search_state.clone(); // Clone search_state

                                // Clone the variables outside the closure
                                let episode_url_clone = episode.enclosure_url.clone().unwrap_or_default();
                                let episode_title_clone = episode.title.clone().unwrap_or_default();
                                let episode_artwork_clone = episode.artwork.clone().unwrap_or_default();
                                // let episode_duration_clone = episode.duration.clone().unwrap_or_default();
                                let episode_duration_clone = episode.duration.clone().unwrap_or_default();
                                let episode_duration_in_seconds = match convert_time_to_seconds(&episode_duration_clone) {
                                    Ok(seconds) => seconds as i32,
                                    Err(e) => {
                                        eprintln!("Failed to convert time to seconds: {}", e);
                                        0
                                    }
                                };
                                let episode_id_clone = episode.episode_id.unwrap_or(0);
                                let server_name_play = server_name.clone();
                                let user_id_play = user_id.clone();
                                let api_key_play = api_key.clone();

                                let is_expanded = search_state.expanded_descriptions.contains(
                                    &episode.guid.clone().unwrap()
                                );


                                let sanitized_description = sanitize_html_with_blank_target(&episode.description.clone().unwrap_or_default());

                                let (description, _is_truncated) = if is_expanded {
                                    (sanitized_description, false)
                                } else {
                                    truncate_description(sanitized_description, 300)
                                };

                                let search_state_toggle = search_state_clone.clone();
                                let toggle_expanded = {
                                    let search_dispatch_clone = search_dispatch.clone();
                                    let episode_guid = episode.guid.clone().unwrap();
                                    Callback::from(move |_: MouseEvent| {
                                        let guid_clone = episode_guid.clone();
                                        let search_dispatch_call = search_dispatch_clone.clone();

                                        if search_state_toggle.expanded_descriptions.contains(&guid_clone) {
                                            search_dispatch_call.apply(AppStateMsg::CollapseEpisode(guid_clone));
                                        } else {
                                            search_dispatch_call.apply(AppStateMsg::ExpandEpisode(guid_clone));
                                        }

                                    })
                                };


                                let state = state.clone();
                                let on_play_click = on_play_click(
                                    episode_url_clone.clone(),
                                    episode_title_clone.clone(),
                                    episode_artwork_clone.clone(),
                                    episode_duration_in_seconds,
                                    episode_id_clone.clone(),
                                    Some(0),
                                    api_key_play.unwrap().unwrap(),
                                    user_id_play.unwrap(),
                                    server_name_play.unwrap(),
                                    dispatch.clone(),
                                    state.clone(),
                                    None,
                                );

                                let description_class = if is_expanded {
                                    "desc-expanded".to_string()
                                } else {
                                    "desc-collapsed".to_string()
                                };

                                let date_format = match_date_format(search_state_clone.date_format.as_deref());
                                let datetime = parse_date(&episode.pub_date.clone().unwrap_or_default(), &search_state_clone.user_tz);
                                let format_release = format!("{}", format_datetime(&datetime, &search_state_clone.hour_preference, date_format));
                                let boxed_episode = Box::new(episode.clone()) as Box<dyn EpisodeTrait>;
                                let formatted_duration = format_time(episode_duration_in_seconds.into());

                                let episode_url_for_ep_item = episode_url_clone.clone();
                                let should_show_buttons = !episode_url_for_ep_item.is_empty();
                                html! {
                                    <div class="item-container flex items-center mb-4 shadow-md rounded-lg">
                                        <img src={episode.artwork.clone().unwrap_or_default()} alt={format!("Cover for {}", &episode.title.clone().unwrap_or_default())} class="object-cover align-top-cover w-full item-container img"/>
                                        <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12">
                                            <p class="item_container-text text-xl font-semibold">{ &episode.title.clone().unwrap_or_default() }</p>
                                            // <p class="text-gray-600">{ &episode.description.clone().unwrap_or_default() }</p>
                                            {
                                                html! {
                                                    <div class="item-container-text hidden md:block">
                                                        <div class={format!("item_container-text episode-description-container {}", description_class)}>
                                                            <SafeHtml html={description} />
                                                        </div>
                                                        <a class="link hover:underline cursor-pointer mt-4" onclick={toggle_expanded}>
                                                            { if is_expanded { "See Less" } else { "See More" } }
                                                        </a>
                                                    </div>
                                                }
                                            }
                                            <span class="episode-time-badge inline-flex items-center px-2.5 py-0.5 rounded me-2">
                                                <svg class="time-icon w-2.5 h-2.5 me-1.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20">
                                                    <path d="M10 0a10 10 0 1 0 10 10A10.011 10.011 0 0 0 10 0Zm3.982 13.982a1 1 0 0 1-1.414 0l-3.274-3.274A1.012 1.012 0 0 1 9 10V6a1 1 0 0 1 2 0v3.586l2.982 2.982a1 1 0 0 1 0 1.414Z"/>
                                                </svg>
                                                { format_release }
                                            </span>
                                            {
                                                // if formatted_listen_duration.is_some() {
                                                //     html! {
                                                //         <div class="flex items-center space-x-2">
                                                //             <span class="item_container-text">{ formatted_listen_duration.clone() }</span>
                                                //             <div class="progress-bar-container">
                                                //                 <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                                //             </div>
                                                //             <span class="item_container-text">{ formatted_duration }</span>
                                                //         </div>
                                                //     }

                                                // } else {
                                                    html! {
                                                        <span class="item_container-text">{ format!("{}", formatted_duration) }</span>
                                                    }
                                                // }
                                            }
                                        </div>
                                        {
                                            html! {
                                                <div class="flex flex-col items-center h-full w-2/12 px-2 space-y-4 md:space-y-8 button-container" style="align-self: center;"> // Add align-self: center; heren medium and larger screens
                                                    if should_show_buttons {
                                                        <button
                                                            class="item-container-button border-solid border selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
                                                            onclick={on_play_click}
                                                        >
                                                        <span class="material-bonus-color material-icons large-material-icons md:text-6xl text-4xl">{"play_arrow"}</span>
                                                        </button>
                                                        {
                                                            if podcast_added {
                                                                let page_type = "episode_layout".to_string();

                                                                let context_button = html! {
                                                                    <ContextButton episode={boxed_episode} page_type={page_type.clone()} />
                                                                };


                                                                context_button

                                                            } else {
                                                                html! {}
                                                            }
                                                        }
                                                    }
                                                </div>
                                            }
                                        }


                                    </div>
                                }
                            })}
                        </div>
                    }
                } else {
                    html! {
                        <div class="empty-episodes-container" id="episode-container">
                            <img src="static/assets/favicon.png" alt="Logo" class="logo"/>
                            <h1 class="page-subtitles">{ "No Episodes Found" }</h1>
                            <p class="page-paragraphs">{"This podcast strangely doesn't have any episodes. Try a more mainstream one maybe?"}</p>
                        </div>
                    }
                }
            }
        <App_drawer />
        // Conditional rendering for the error banner
        {
            if state.error_message.as_ref().map_or(false, |msg| !msg.is_empty()) {
                html! { <div class="error-snackbar">{ &state.error_message }</div> }
            } else {
                html! {}
            }
        }
        //     if !state.error_message.is_empty() {
        //         html! { <div class="error-snackbar">{ &state.error_message }</div> }
        //     } else {
        //         html! {}
        //     }
        // }
        //     // Conditional rendering for the info banner
        {
        if state.info_message.as_ref().map_or(false, |msg| !msg.is_empty()) {
                html! { <div class="info-snackbar">{ &state.info_message }</div> }
            } else {
                html! {}
            }
        }
        // {
        //     if !state.info_message.is_empty() {
        //         html! { <div class="info-snackbar">{ &state.info_message }</div> }
        //     } else {
        //         html! {}
        //     }
        // }
        {
            if let Some(audio_props) = &state.currently_playing {
                html! { <AudioPlayer src={audio_props.src.clone()} title={audio_props.title.clone()} artwork_url={audio_props.artwork_url.clone()} duration={audio_props.duration.clone()} episode_id={audio_props.episode_id.clone()} duration_sec={audio_props.duration_sec.clone()} start_pos_sec={audio_props.start_pos_sec.clone()} end_pos_sec={audio_props.end_pos_sec.clone()} offline={audio_props.offline.clone()} /> }
            } else {
                html! {}
            }
        }
        </div>

    }
}
