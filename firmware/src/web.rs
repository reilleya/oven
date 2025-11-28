use embassy_net::Stack;
use embassy_time::Duration;
use esp_alloc as _;

use picoserve::{
    AppBuilder, AppRouter, Router,
    extract::State,
    response::{File, IntoResponse, IntoResponseWithState, Redirect, with_state::WithStateUpdate},
    routing,
    routing::{get, parse_path_segment},
};

use core::cell::RefCell;

struct AppState {
    temperature: RefCell<i32>,
    time: RefCell<i32>,
}

#[derive(serde::Serialize)]
struct AppStateValue {
    temperature: i32,
    time: i32,
}

impl picoserve::extract::FromRef<AppState> for AppStateValue {
    fn from_ref(
        AppState {
            temperature, time, ..
        }: &AppState,
    ) -> Self {
        Self {
            temperature: *temperature.borrow(),
            time: *time.borrow(),
        }
    }
}

async fn get_temperature(State(value): State<AppStateValue>) -> impl IntoResponse {
    picoserve::response::Json(value)
}

async fn increment_temperature() -> impl IntoResponseWithState<AppState> {
    Redirect::to(".").with_state_update(async |state: &AppState| {
        *state.temperature.borrow_mut() += 1;
    })
}

async fn set_temperature(value: i32) -> impl IntoResponseWithState<AppState> {
    Redirect::to("..").with_state_update(async move |state: &AppState| {
        *state.temperature.borrow_mut() = value;
    })
}

pub struct Application;

impl AppBuilder for Application {
    type PathRouter = impl routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        let state = AppState {
            temperature: 0.into(),
            time: 0.into(),
        };

        picoserve::Router::new()
            .route(
                "/",
                routing::get_service(File::html(include_str!("web/index.html"))),
            )
            .route(
                "/buttons.js",
                routing::get_service(File::javascript(include_str!("web/buttons.js"))),
            )
            .route(
                "/styles.css",
                routing::get_service(File::css(include_str!("web/styles.css"))),
            )
            .route("/get", get(get_temperature))
            .route("/increment", get(increment_temperature))
            .route(("/set", parse_path_segment()), get(set_temperature))
            .with_state(state)
    }
}

pub const WEB_TASK_POOL_SIZE: usize = 2;

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(
    task_id: usize,
    stack: Stack<'static>,
    router: &'static AppRouter<Application>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::Server::new(router, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}

pub struct WebApp {
    pub router: &'static Router<<Application as AppBuilder>::PathRouter>,
    pub config: &'static picoserve::Config<Duration>,
}

impl Default for WebApp {
    fn default() -> Self {
        let router = picoserve::make_static!(AppRouter<Application>, Application.build_app());

        let config = picoserve::make_static!(
            picoserve::Config<Duration>,
            picoserve::Config::new(picoserve::Timeouts {
                start_read_request: Some(Duration::from_secs(5)),
                read_request: Some(Duration::from_secs(1)),
                write: Some(Duration::from_secs(1)),
                persistent_start_read_request: Some(Duration::from_secs(1)),
            })
            .keep_connection_alive()
        );

        Self { router, config }
    }
}
