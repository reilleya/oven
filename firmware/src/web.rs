use embassy_net::Stack;
use embassy_time::Duration;
use esp_alloc as _;

use picoserve::{
    AppWithStateBuilder, AppRouter, Router,
    extract::Form,
    extract::State,
    response::{File, IntoResponse, IntoResponseWithState, Redirect, with_state::WithStateUpdate},
    routing,
    routing::{get, parse_path_segment, post},
};

use core::cell::RefCell;

#[derive(serde::Deserialize)]
struct FormValue {
    temperature: i32,
    time: i32,
}

struct AppState {
    current_temp: RefCell<i32>,
    setpoint_temp: RefCell<i32>,
    run_time_elapsed: RefCell<i32>,
    run_time_total: RefCell<i32>,
}

#[derive(serde::Serialize)]
struct AppStateValue {
    current_temp: i32,
    setpoint_temp: i32,
    run_time_elapsed: i32,
    run_time_total: i32,
}

impl picoserve::extract::FromRef<AppState> for AppStateValue {
    fn from_ref(
        AppState {
            current_temp,
            setpoint_temp,
            run_time_elapsed,
            run_time_total,
            ..
        }: &AppState,
    ) -> Self {
        Self {
            current_temp: *current_temp.borrow(),
            setpoint_temp: *setpoint_temp.borrow(),
            run_time_elapsed: *run_time_elapsed.borrow(),
            run_time_total: *run_time_total.borrow(),
        }
    }
}

async fn get_state(State(value): State<AppStateValue>) -> impl IntoResponse {
    picoserve::response::Json(value)
}

async fn set_temperature(value: i32) -> impl IntoResponseWithState<AppState> {
    Redirect::to("..").with_state_update(async move |state: &AppState| {
        *state.setpoint_temp.borrow_mut() = value;
    })
}

async fn set_config(
    Form(FormValue { temperature, time }): Form<FormValue>,
) -> impl IntoResponseWithState<AppState> {
    picoserve::response::Json(0).with_state_update(async move |state: &AppState| {
        // TODO: better response than Json(0)
        *state.setpoint_temp.borrow_mut() = temperature; // TODO: validate?
        *state.run_time_total.borrow_mut() = time;
        // TODO: start the run
    })
}

pub struct Application;

impl AppWithStateBuilder for Application {
    type State = AppState;
    type PathRouter = impl routing::PathRouter<Self::State>;

    fn build_app(self) -> picoserve::Router<Self::PathRouter, AppState> {
        let state = AppState {
            current_temp: 0.into(),
            setpoint_temp: 0.into(),
            run_time_elapsed: 0.into(),
            run_time_total: 0.into(),
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
            .route(("/set", parse_path_segment()), get(set_temperature))
            .route("/get_state", get(get_state))
            .route("/set_config", post(set_config))
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
    pub router: &'static AppRouter<Application>,
    pub config: &'static picoserve::Config<Duration>,
}

impl Default for WebApp {
    fn default() -> Self {

        let state = AppState {
            current_temp: 0.into(),
            setpoint_temp: 0.into(),
            run_time_elapsed: 0.into(),
            run_time_total: 0.into(),
        };

        let router = picoserve::Router::new()
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
            .route(("/set", parse_path_segment()), get(set_temperature))
            .route("/get_state", get(get_state))
            .route("/set_config", post(set_config))
            .with_state(state);

        let router = picoserve::make_static!(Router<routing::PathRouter<AppState>>, router);

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
