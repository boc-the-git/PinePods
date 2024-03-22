use yew::{function_component, Html, html};
use yew::prelude::*;
use super::app_drawer::App_drawer;
use super::gen_components::{Search_nav, empty_message, episode_item, on_shownotes_click};
use crate::requests::pod_req;
use yewdux::prelude::*;
use yew_router::history::BrowserHistory;
use crate::components::context::{AppState, UIState};
use crate::components::audio::AudioPlayer;
use crate::components::gen_funcs::{sanitize_html_with_blank_target, truncate_description, format_datetime, parse_date, DateFormat};
use crate::requests::pod_req::RecentEps;
use crate::components::audio::on_play_click;
use crate::components::episodes_layout::AppStateMsg;
// use crate::components::gen_funcs::check_auth;
use crate::components::episodes_layout::UIStateMsg;
use wasm_bindgen::closure::Closure;
use web_sys::{console, window};
use wasm_bindgen::JsCast;
use crate::requests::login_requests::use_check_authentication;


#[function_component(Home)]
pub fn home() -> Html {
    let (state, dispatch) = use_store::<AppState>();
    let effect_dispatch = dispatch.clone();
    let history = BrowserHistory::new();

    console::log_1(&format!("About to run check auth").into());
    // check_auth(effect_dispatch);

    let session_dispatch = effect_dispatch.clone();
    let session_state = state.clone();

    use_effect_with((), move |_| {
        // Check if the page reload action has already occurred to prevent redundant execution
        if session_state.reload_occured.unwrap_or(false) {
            // Logic for the case where reload has already been processed
        } else {
            // Normal effect logic for handling page reload
            let window = web_sys::window().expect("no global `window` exists");
            let performance = window.performance().expect("should have performance");
            let navigation_type = performance.navigation().type_();
            
            if navigation_type == 1 { // 1 stands for reload
                let session_storage = window.session_storage().unwrap().unwrap();
                session_storage.set_item("isAuthenticated", "false").unwrap();
                console::log_1(&"Page was reloaded, user not authenticated, clearing session storage".into());
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
    
    
    
    
    // use_effect_with(
    //     (),
    //     move |_| {
    //         let effect_dispatch_clone = effect_dispatch.clone();

    //         spawn_local(async move {
    //             let window = window().expect("no global `window` exists");
    //             let location = window.location();
    //             let current_route = location.href().expect("should be able to get href");
    //             console::log_1(&current_route.clone().into());
    //             console::log_1(&"Checking authentication... Inside Check_auth".into());
    //             use_check_authentication(effect_dispatch_clone, &current_route);
    //         });

    //         || ()
    //     }
    // );

    let error = use_state(|| None);
    let (post_state, _post_dispatch) = use_store::<AppState>();
    let (audio_state, audio_dispatch) = use_store::<UIState>();
    let error_message = audio_state.error_message.clone();
    let info_message = audio_state.info_message.clone();
    let loading = use_state(|| true);
    web_sys::console::log_1(&"testlog".into());

    {
        let ui_dispatch = audio_dispatch.clone();
        use_effect(move || {
            let window = window().unwrap();
            let document = window.document().unwrap();

            let closure = Closure::wrap(Box::new(move |_event: Event| {
                ui_dispatch.apply(UIStateMsg::ClearErrorMessage);
                ui_dispatch.apply(UIStateMsg::ClearInfoMessage);
            }) as Box<dyn Fn(_)>);

            document.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();

            // Return cleanup function
            move || {
                document.remove_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
                closure.forget(); // Prevents the closure from being dropped
            }
        });
    }


    // Fetch episodes on component mount
    let loading_ep = loading.clone();
    {
        // let episodes = episodes.clone();
        console::log_1(&format!("loading ep value: {:?}", loading_ep).into());

        let error = error.clone();
        let api_key = post_state.auth_details.as_ref().map(|ud| ud.api_key.clone());
        let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
        let server_name = post_state.auth_details.as_ref().map(|ud| ud.server_name.clone());

        let effect_dispatch = dispatch.clone();

        // fetch_episodes(api_key.flatten(), user_id, server_name, dispatch, error, pod_req::call_get_recent_eps);

        use_effect_with(
            (api_key.clone(), user_id.clone(), server_name.clone()),
            move |_| {
                let error_clone = error.clone();
                if let (Some(api_key), Some(user_id), Some(server_name)) = (api_key.clone(), user_id.clone(), server_name.clone()) {
                    let dispatch = effect_dispatch.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        match pod_req::call_get_recent_eps(&server_name, &api_key, &user_id).await {
                            Ok(fetched_episodes) => {
                                dispatch.reduce_mut(move |state| {
                                    state.server_feed_results = Some(RecentEps { episodes: Some(fetched_episodes) });
                                });
                                loading_ep.set(false);
                            },
                            Err(e) => {
                                error_clone.set(Some(e.to_string()));
                                loading_ep.set(false); // Set loading to false here
                            },
                        }
                    });
                }
                || ()
            },
        );
    }

    console::log_1(&format!("loading ep value: {:?}", *loading).into());

    html! {
        <>
        <div class="main-container">
            <Search_nav />
            {
                if *loading { // If loading is true, display the loading animation
                    html! {
                        <div class="loading-animation">
                            <div class="frame1"></div>
                            <div class="frame2"></div>
                            <div class="frame3"></div>
                            <div class="frame4"></div>
                            <div class="frame5"></div>
                            <div class="frame6"></div>
                        </div>
                    }
                } else {
                    if let Some(recent_eps) = state.server_feed_results.clone() {
                        let int_recent_eps = recent_eps.clone();
                        if let Some(episodes) = int_recent_eps.episodes {
                            let api_key = post_state.auth_details.as_ref().map(|ud| ud.api_key.clone());
                            let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
                            let server_name = post_state.auth_details.as_ref().map(|ud| ud.server_name.clone());
                            let history_clone = history.clone();
                    
                            if episodes.is_empty() {
                                // Render "No Recent Episodes Found" if episodes list is empty
                                empty_message(
                                    "No Recent Episodes Found",
                                    "You can add new podcasts by using the search bar above. Search for your favorite podcast and click the plus button to add it."
                                )
                            } else {
                            episodes.into_iter().map(|episode| {
                                let id_string = &episode.EpisodeID.to_string();
        
                                let is_expanded = state.expanded_descriptions.contains(id_string);
        
                                let dispatch = dispatch.clone();
        
                                let episode_url_clone = episode.EpisodeURL.clone();
                                let episode_title_clone = episode.EpisodeTitle.clone();
                                let episode_artwork_clone = episode.EpisodeArtwork.clone();
                                let episode_duration_clone = episode.EpisodeDuration.clone();
                                let episode_id_clone = episode.EpisodeID.clone();
                                let episode_listened_clone = episode.ListenDuration.clone();

                                let sanitized_description = sanitize_html_with_blank_target(&episode.EpisodeDescription.clone());

                                let (description, _is_truncated) = if is_expanded {
                                    (sanitized_description, false)
                                } else {
                                    truncate_description(sanitized_description, 300)
                                };
        
                                let toggle_expanded = {
                                    let search_dispatch_clone = dispatch.clone();
                                    let state_clone = state.clone();
                                    let episode_guid = episode.EpisodeID.clone();
        
                                    Callback::from(move |_: MouseEvent| {
                                        let guid_clone = episode_guid.to_string().clone();
                                        let search_dispatch_call = search_dispatch_clone.clone();
        
                                        if state_clone.expanded_descriptions.contains(&guid_clone) {
                                            search_dispatch_call.apply(AppStateMsg::CollapseEpisode(guid_clone));
                                        } else {
                                            search_dispatch_call.apply(AppStateMsg::ExpandEpisode(guid_clone));
                                        }
                                    })
                                };

                                let episode_url_for_closure = episode_url_clone.clone();
                                let episode_title_for_closure = episode_title_clone.clone();
                                let episode_artwork_for_closure = episode_artwork_clone.clone();
                                let episode_duration_for_closure = episode_duration_clone.clone();
                                let listener_duration_for_closure = episode_listened_clone.clone();
                                let episode_id_for_closure = episode_id_clone.clone();
                                let user_id_play = user_id.clone();
                                let server_name_play = server_name.clone();
                                let api_key_play = api_key.clone();
                                let audio_dispatch = audio_dispatch.clone();

                                let on_play_click = on_play_click(
                                    episode_url_for_closure.clone(),
                                    episode_title_for_closure.clone(),
                                    episode_artwork_for_closure.clone(),
                                    episode_duration_for_closure.clone(),
                                    episode_id_for_closure.clone(),
                                    listener_duration_for_closure.clone(),
                                    api_key_play.unwrap().unwrap(),
                                    user_id_play.unwrap(),
                                    server_name_play.unwrap(),
                                    audio_dispatch.clone(),
                                    audio_state.clone(),
                                );

                                let on_shownotes_click = on_shownotes_click(
                                    history_clone.clone(),
                                    dispatch.clone(),
                                    episode_id_for_closure.clone(),
                                );
    
                                // let format_release = format!("Released on: {}", &episode.EpisodePubDate);
                                console::log_1(&format!("Episode pub date: {}", &episode.EpisodePubDate).into());
                                console::log_1(&format!("User timezone: {:?}", &state.user_tz).into());
                                console::log_1(&format!("User date format: {:?}", &state.date_format).into());
                                let date_format = match state.date_format.as_deref() {
                                    Some("MDY") => DateFormat::MDY,
                                    Some("DMY") => DateFormat::DMY,
                                    Some("YMD") => DateFormat::YMD,
                                    Some("JUL") => DateFormat::JUL,
                                    Some("ISO") => DateFormat::ISO,
                                    Some("USA") => DateFormat::USA,
                                    Some("EUR") => DateFormat::EUR,
                                    Some("JIS") => DateFormat::JIS,
                                    _ => DateFormat::ISO, // default to ISO if the format is not recognized
                                };
                                
                                let datetime = parse_date(&episode.EpisodePubDate, &state.user_tz);
                                // let datetime = parse_date(&episode.EpisodePubDate, &state.user_tz, &state.date_format);
                                let format_release = format!("{}", format_datetime(&datetime, &state.hour_preference, date_format));
                                console::log_1(&format!("Formatted release: {}", &format_release).into());
                                let item = episode_item(
                                    Box::new(episode),
                                    description.clone(),
                                    is_expanded,
                                    &format_release,
                                    on_play_click,
                                    on_shownotes_click,
                                    toggle_expanded,
                                    episode_duration_clone,
                                    episode_listened_clone,
                                    "home",
                                    Callback::from(|_| {}), 
                                    false,
                                );

                                item
                            }).collect::<Html>()
                            }
                        } else {
                            empty_message(
                                "No Recent Episodes Found",
                                "You can add new podcasts by using the search bar above. Search for your favorite podcast and click the plus button to add it."
                            )
                        }
                    } else {
                        empty_message(
                            "No Recent Episodes Found",
                            "You can add new podcasts by using the search bar above. Search for your favorite podcast and click the plus button to add it."
                        )
                    }
                }
            }
        {
            if let Some(audio_props) = &audio_state.currently_playing {
                html! { <AudioPlayer src={audio_props.src.clone()} title={audio_props.title.clone()} artwork_url={audio_props.artwork_url.clone()} duration={audio_props.duration.clone()} episode_id={audio_props.episode_id.clone()} duration_sec={audio_props.duration_sec.clone()} start_pos_sec={audio_props.start_pos_sec.clone()} /> }
            } else {
                html! {}
            }
        }
        // Conditional rendering for the error banner
        if let Some(error) = error_message {
            <div class="error-snackbar">{ error }</div>
        }
        if let Some(info) = info_message {
            <div class="info-snackbar">{ info }</div>
        }
        </div>
        <App_drawer />
        </>
    }
}