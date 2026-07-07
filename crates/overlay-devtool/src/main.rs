mod mock;
mod render;

use std::{env, io::Cursor};

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use vrcx_0_vr_overlay::{
    FavoriteFriendsPanelModel, FriendPanelAction, MainSurfaceModel, UvPoint, WristSurfaceModel,
};

use crate::render::DevtoolRenderer;

const INDEX_HTML: &str = include_str!("../web/index.html");

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = env::var("VRCX_OVERLAY_DEVTOOL_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(47391);
    let address = format!("127.0.0.1:{port}");
    let server = Server::http(&address)?;
    let mut app = AppState::new();
    let mut renderer = DevtoolRenderer::new();
    println!("VRCX-0 overlay devtool: http://{address}");
    for mut request in server.incoming_requests() {
        let response = handle_request(&mut app, &mut renderer, &mut request);
        if let Err(error) = request.respond(response) {
            eprintln!("failed to respond to overlay devtool request: {error}");
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceKind {
    Friends,
    Toast,
    Wrist,
}

impl SurfaceKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "friends" => Some(Self::Friends),
            "toast" | "hmd" | "main" => Some(Self::Toast),
            "wrist" => Some(Self::Wrist),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Friends => "friends",
            Self::Toast => "toast",
            Self::Wrist => "wrist",
        }
    }
}

struct AppState {
    surface: SurfaceKind,
    friends_scenario: String,
    toast_scenario: String,
    wrist_scenario: String,
    friends: FavoriteFriendsPanelModel,
    toast: MainSurfaceModel,
    wrist: WristSurfaceModel,
    injected_toasts: usize,
}

impl AppState {
    fn new() -> Self {
        let friends_scenario = mock::friends::default_scenario_key().to_string();
        let toast_scenario = mock::toast::default_scenario_key().to_string();
        let wrist_scenario = mock::wrist::default_scenario_key().to_string();
        Self {
            surface: SurfaceKind::Friends,
            friends: mock::friends::build(&friends_scenario),
            toast: mock::toast::build(&toast_scenario),
            wrist: mock::wrist::build(&wrist_scenario),
            friends_scenario,
            toast_scenario,
            wrist_scenario,
            injected_toasts: 0,
        }
    }

    fn select(&mut self, surface: SurfaceKind, scenario: &str) {
        self.surface = surface;
        match surface {
            SurfaceKind::Friends => {
                self.friends_scenario = mock::friends::normalize_scenario(scenario).to_string();
            }
            SurfaceKind::Toast => {
                self.toast_scenario = mock::toast::normalize_scenario(scenario).to_string();
            }
            SurfaceKind::Wrist => {
                self.wrist_scenario = mock::wrist::normalize_scenario(scenario).to_string();
            }
        }
        self.reset_current();
    }

    fn reset_current(&mut self) {
        match self.surface {
            SurfaceKind::Friends => {
                self.friends = mock::friends::build(&self.friends_scenario);
            }
            SurfaceKind::Toast => {
                self.toast = mock::toast::build(&self.toast_scenario);
                self.injected_toasts = 0;
            }
            SurfaceKind::Wrist => {
                self.wrist = mock::wrist::build(&self.wrist_scenario);
            }
        }
    }

    fn apply_friends_input(&mut self, input: InputRequest) -> serde_json::Value {
        let uv = UvPoint::new(input.x, input.y);
        let previous_category = self.friends.selected_category_key.clone();
        let hit = match input.action.as_str() {
            "hover" => self.friends.apply_uv_action(uv, FriendPanelAction::Hover),
            "click" => {
                self.friends
                    .apply_uv_action(uv, FriendPanelAction::ClickDown);
                self.friends.apply_uv_action(uv, FriendPanelAction::ClickUp)
            }
            "scroll" | "touchScroll" => self.friends.apply_uv_action(
                uv,
                FriendPanelAction::Scroll {
                    delta: input.delta.unwrap_or_default(),
                },
            ),
            _ => None,
        };
        if previous_category != self.friends.selected_category_key {
            self.friends.rows = mock::friends::rows_for_category(
                &self.friends_scenario,
                &self.friends.selected_category_key,
            );
        }
        json!({
            "hit": hit,
            "selectedCategory": self.friends.selected_category_key,
            "categoryScrollOffset": self.friends.category_scroll_offset,
            "rowScrollOffset": self.friends.row_scroll_offset
        })
    }

    fn apply_toast_action(&mut self, action: &str) {
        match action {
            "append" => {
                mock::toast::append_mock_toast(&mut self.toast, self.injected_toasts);
                self.injected_toasts += 1;
            }
            "clear" => {
                self.toast.toasts.clear();
            }
            _ => {}
        }
    }

    fn step_spinner(&mut self) {
        self.friends.spinner_phase = (self.friends.spinner_phase + 0.1).rem_euclid(1.0);
    }

    fn current_scenario(&self) -> &str {
        match self.surface {
            SurfaceKind::Friends => &self.friends_scenario,
            SurfaceKind::Toast => &self.toast_scenario,
            SurfaceKind::Wrist => &self.wrist_scenario,
        }
    }

    fn state_json(&self) -> serde_json::Value {
        json!({
            "surface": self.surface.as_str(),
            "scenario": self.current_scenario(),
            "scenarios": {
                "friends": scenario_json(mock::friends::scenario_infos()),
                "toast": scenario_json(mock::toast::scenario_infos()),
                "wrist": scenario_json(mock::wrist::scenario_infos())
            },
            "friends": {
                "selectedCategory": self.friends.selected_category_key,
                "categoryScrollOffset": self.friends.category_scroll_offset,
                "rowScrollOffset": self.friends.row_scroll_offset,
                "rows": self.friends.rows.len()
            },
            "toast": {
                "toasts": self.toast.toasts.len()
            }
        })
    }
}

#[derive(Deserialize)]
struct SelectRequest {
    surface: String,
    scenario: String,
}

#[derive(Deserialize)]
struct InputRequest {
    action: String,
    x: f32,
    y: f32,
    #[serde(default)]
    delta: Option<f32>,
}

#[derive(Deserialize)]
struct ToastRequest {
    action: String,
}

fn handle_request(
    app: &mut AppState,
    renderer: &mut DevtoolRenderer,
    request: &mut Request,
) -> Response<Cursor<Vec<u8>>> {
    let path = request.url().split('?').next().unwrap_or(request.url());
    match (request.method(), path) {
        (&Method::Get, "/") | (&Method::Get, "/index.html") => {
            text_response(200, "text/html; charset=utf-8", INDEX_HTML)
        }
        (&Method::Get, "/api/state") => json_response(200, app.state_json()),
        (&Method::Get, "/frame.png") => match render_current_png(app, renderer) {
            Ok(png) => bytes_response(200, "image/png", png)
                .with_header(header("Cache-Control", "no-store, max-age=0")),
            Err(error) => json_response(500, json!({ "error": error })),
        },
        (&Method::Post, "/api/select") => json_post::<SelectRequest, _>(request, |input| {
            if let Some(surface) = SurfaceKind::parse(&input.surface) {
                app.select(surface, &input.scenario);
                json_response(200, app.state_json())
            } else {
                json_response(400, json!({ "error": "unknown surface" }))
            }
        }),
        (&Method::Post, "/api/input") => json_post::<InputRequest, _>(request, |input| {
            let result = app.apply_friends_input(input);
            json_response(200, json!({ "state": app.state_json(), "result": result }))
        }),
        (&Method::Post, "/api/toast") => json_post::<ToastRequest, _>(request, |input| {
            app.apply_toast_action(&input.action);
            json_response(200, app.state_json())
        }),
        (&Method::Post, "/api/spinner") => {
            app.step_spinner();
            json_response(200, app.state_json())
        }
        (&Method::Post, "/api/reset") => {
            app.reset_current();
            json_response(200, app.state_json())
        }
        _ => json_response(404, json!({ "error": "not found" })),
    }
}

fn render_current_png(app: &AppState, renderer: &mut DevtoolRenderer) -> Result<Vec<u8>, String> {
    match app.surface {
        SurfaceKind::Friends => renderer.friends_png(&app.friends),
        SurfaceKind::Toast => renderer.main_png(&app.toast),
        SurfaceKind::Wrist => renderer.wrist_png(&app.wrist),
    }
}

fn json_post<T, F>(request: &mut Request, on_ok: F) -> Response<Cursor<Vec<u8>>>
where
    T: DeserializeOwned,
    F: FnOnce(T) -> Response<Cursor<Vec<u8>>>,
{
    match read_json::<T>(request) {
        Ok(input) => on_ok(input),
        Err(error) => json_response(400, json!({ "error": error })),
    }
}

fn read_json<T: DeserializeOwned>(request: &mut Request) -> Result<T, String> {
    let mut body = String::new();
    request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|error| format!("read request body failed: {error}"))?;
    serde_json::from_str(&body).map_err(|error| format!("invalid JSON: {error}"))
}

fn scenario_json(infos: &[mock::ScenarioInfo]) -> serde_json::Value {
    serde_json::Value::Array(
        infos
            .iter()
            .map(|info| json!({ "key": info.key, "label": info.label }))
            .collect(),
    )
}

fn text_response(status: u16, content_type: &str, body: &str) -> Response<Cursor<Vec<u8>>> {
    bytes_response(status, content_type, body.as_bytes().to_vec())
}

fn json_response(status: u16, value: serde_json::Value) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{\"error\":\"json\"}".to_vec());
    bytes_response(status, "application/json; charset=utf-8", body)
}

fn bytes_response(status: u16, content_type: &str, body: Vec<u8>) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(body)
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", content_type))
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid HTTP header")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_mock_surfaces_to_png() {
        let mut app = AppState::new();
        let mut renderer = DevtoolRenderer::new();
        for surface in [SurfaceKind::Friends, SurfaceKind::Toast, SurfaceKind::Wrist] {
            let scenario = match surface {
                SurfaceKind::Friends => mock::friends::default_scenario_key(),
                SurfaceKind::Toast => mock::toast::default_scenario_key(),
                SurfaceKind::Wrist => mock::wrist::default_scenario_key(),
            };
            app.select(surface, scenario);
            let png = render_current_png(&app, &mut renderer).expect("render PNG");
            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }

    #[test]
    fn friends_input_can_switch_mock_category_rows() {
        let mut app = AppState::new();
        let all_rows = app.friends.rows.len();
        let uv = vrcx_0_vr_overlay::build_friends_panel_scene(&app.friends)
            .hit_regions
            .iter()
            .find(|region| region.id == "cat:group:remote:Travelers")
            .map(|region| region.rect.center_uv(app.friends.size))
            .expect("travelers category region");
        app.apply_friends_input(InputRequest {
            action: "click".to_string(),
            x: uv.x,
            y: uv.y,
            delta: None,
        });

        assert_ne!(app.friends.selected_category_key, "all");
        assert_ne!(app.friends.rows.len(), all_rows);
    }

    #[test]
    fn friends_many_groups_mock_exercises_category_scroll() {
        let mut app = AppState::new();
        app.select(SurfaceKind::Friends, "manyGroups");
        let mock_group_count = app
            .friends
            .categories
            .iter()
            .filter(|category| {
                category.key.starts_with("group:friend:mock_group_")
                    || category.key.starts_with("group:local:mock_local_")
            })
            .count();
        assert_eq!(mock_group_count, 42);
        assert!(app.friends.max_category_scroll_offset() > 0);

        let uv = vrcx_0_vr_overlay::build_friends_panel_scene(&app.friends)
            .hit_regions
            .iter()
            .find(|region| region.id == "category-list")
            .map(|region| region.rect.center_uv(app.friends.size))
            .expect("category list region");
        app.apply_friends_input(InputRequest {
            action: "touchScroll".to_string(),
            x: uv.x,
            y: uv.y,
            delta: Some(3.0),
        });

        assert!(app.friends.category_scroll_offset > 0);
    }

    #[test]
    fn friends_same_instance_mock_defaults_to_same_instance_rows() {
        let mut app = AppState::new();
        app.select(SurfaceKind::Friends, "sameInstance");

        assert_eq!(app.friends.selected_category_key, "sameInstance");
        assert!(!app.friends.rows.is_empty());
        let section_count = app
            .friends
            .rows
            .iter()
            .filter(|row| row.section_label.is_some())
            .count();
        assert!(section_count >= 3);
        assert!(app.friends.rows.iter().any(|row| {
            row.section_label
                .as_deref()
                .is_some_and(|label| label == "The Black Cat")
        }));
        assert!(app
            .friends
            .rows
            .iter()
            .filter(|row| !row.user_id.is_empty())
            .all(|row| !row.is_traveling && row.location_text != "Private"));

        let same_instance_count = app
            .friends
            .categories
            .iter()
            .find(|category| category.key == "sameInstance")
            .map(|category| category.count)
            .expect("same instance category");
        assert_eq!(
            same_instance_count,
            app.friends
                .rows
                .iter()
                .filter(|row| !row.user_id.is_empty())
                .count()
        );
    }
}
