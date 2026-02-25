use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserOperationTemplate {
    pub browser: BrowserTemplate,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserTemplate {
    pub session_id: Option<String>,
    #[serde(default = "default_headless")]
    pub headless: bool,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub on_failure_screenshot: bool,
    pub steps: Vec<BrowserStep>,
}

fn default_headless() -> bool {
    true
}
fn default_timeout_ms() -> u64 {
    30_000
}
fn default_true() -> bool {
    true
}

// ── BrowserStep ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowserStep {
    // ── Navigation ──────────────────────────────────────────────────────────
    Navigate {
        url: String,
        #[serde(default)]
        expected_status: Option<u16>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    NavigateBack {
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    NavigateForward {
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Reload {
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },

    // ── Observation ─────────────────────────────────────────────────────────
    Snapshot {
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Screenshot {
        #[serde(default)]
        path: Option<String>,
        #[serde(default, rename = "type")]
        r#type: Option<String>,
        #[serde(default)]
        full_page: bool,
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    ConsoleMessages {
        #[serde(default)]
        level: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    ConsoleClear {
        #[serde(default)]
        optional: bool,
    },
    NetworkRequests {
        #[serde(default)]
        include_static: bool,
        #[serde(default)]
        optional: bool,
    },
    NetworkClear {
        #[serde(default)]
        optional: bool,
    },

    // ── Interaction ─────────────────────────────────────────────────────────
    Click {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        double_click: bool,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Hover {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Drag {
        #[serde(default, rename = "start_ref")]
        start_ref: Option<String>,
        #[serde(default)]
        start_selector: Option<String>,
        #[serde(default, rename = "end_ref")]
        end_ref: Option<String>,
        #[serde(default)]
        end_selector: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Fill {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        selector: Option<String>,
        text: String,
        #[serde(default)]
        submit: Option<bool>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    FillForm {
        #[rkyv(with = earl_core::with::AsJson)]
        fields: Vec<Value>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    SelectOption {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        selector: Option<String>,
        values: Vec<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    PressKey {
        key: String,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Check {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    Uncheck {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },

    // ── Mouse ────────────────────────────────────────────────────────────────
    MouseMove {
        x: f64,
        y: f64,
        #[serde(default)]
        optional: bool,
    },
    MouseClick {
        x: f64,
        y: f64,
        #[serde(default)]
        button: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    MouseDrag {
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        #[serde(default)]
        optional: bool,
    },
    MouseDown {
        #[serde(default)]
        button: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    MouseUp {
        #[serde(default)]
        button: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    MouseWheel {
        delta_x: f64,
        delta_y: f64,
        #[serde(default)]
        optional: bool,
    },

    // ── Wait & Assert ────────────────────────────────────────────────────────
    WaitFor {
        #[serde(default)]
        time: Option<f64>,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        text_gone: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    VerifyElementVisible {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        accessible_name: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    VerifyTextVisible {
        text: String,
        #[serde(default)]
        optional: bool,
    },
    VerifyListVisible {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        items: Vec<String>,
        #[serde(default)]
        optional: bool,
    },
    VerifyValue {
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        value: String,
        #[serde(default)]
        optional: bool,
    },

    // ── JavaScript ───────────────────────────────────────────────────────────
    Evaluate {
        function: String,
        #[serde(default, rename = "ref")]
        r#ref: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    RunCode {
        code: String,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },

    // ── Tabs & Viewport ──────────────────────────────────────────────────────
    Tabs {
        operation: String,
        #[serde(default)]
        index: Option<usize>,
        #[serde(default)]
        optional: bool,
    },
    Resize {
        width: u32,
        height: u32,
        #[serde(default)]
        optional: bool,
    },
    Close {
        #[serde(default)]
        optional: bool,
    },

    // ── Network mocking ──────────────────────────────────────────────────────
    Route {
        pattern: String,
        #[serde(default)]
        status: Option<u16>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        content_type: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    RouteList {
        #[serde(default)]
        optional: bool,
    },
    Unroute {
        pattern: String,
        #[serde(default)]
        optional: bool,
    },

    // ── Cookies ──────────────────────────────────────────────────────────────
    CookieList {
        #[serde(default)]
        domain: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    CookieGet {
        name: String,
        #[serde(default)]
        optional: bool,
    },
    CookieSet {
        name: String,
        value: String,
        #[serde(default)]
        domain: Option<String>,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        expires: Option<f64>,
        #[serde(default)]
        http_only: bool,
        #[serde(default)]
        secure: bool,
        #[serde(default)]
        optional: bool,
    },
    CookieDelete {
        name: String,
        #[serde(default)]
        optional: bool,
    },
    CookieClear {
        #[serde(default)]
        optional: bool,
    },

    // ── Web Storage ──────────────────────────────────────────────────────────
    LocalStorageGet {
        key: String,
        #[serde(default)]
        optional: bool,
    },
    LocalStorageSet {
        key: String,
        value: String,
        #[serde(default)]
        optional: bool,
    },
    LocalStorageDelete {
        key: String,
        #[serde(default)]
        optional: bool,
    },
    LocalStorageClear {
        #[serde(default)]
        optional: bool,
    },
    SessionStorageGet {
        key: String,
        #[serde(default)]
        optional: bool,
    },
    SessionStorageSet {
        key: String,
        value: String,
        #[serde(default)]
        optional: bool,
    },
    SessionStorageDelete {
        key: String,
        #[serde(default)]
        optional: bool,
    },
    SessionStorageClear {
        #[serde(default)]
        optional: bool,
    },
    StorageState {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    SetStorageState {
        path: String,
        #[serde(default)]
        optional: bool,
    },

    // ── File, Dialog, Download ───────────────────────────────────────────────
    FileUpload {
        paths: Vec<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },
    HandleDialog {
        accept: bool,
        #[serde(default)]
        prompt_text: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    Download {
        save_to: String,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        optional: bool,
    },

    // ── Output ───────────────────────────────────────────────────────────────
    PdfSave {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    StartVideo {
        #[serde(default)]
        width: Option<u32>,
        #[serde(default)]
        height: Option<u32>,
        #[serde(default)]
        optional: bool,
    },
    StopVideo {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    StartTracing {
        #[serde(default)]
        optional: bool,
    },
    StopTracing {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    GenerateLocator {
        #[serde(rename = "ref")]
        r#ref: String,
        #[serde(default)]
        optional: bool,
    },
}

impl BrowserStep {
    pub fn action_name(&self) -> &'static str {
        match self {
            Self::Navigate { .. } => "navigate",
            Self::NavigateBack { .. } => "navigate_back",
            Self::NavigateForward { .. } => "navigate_forward",
            Self::Reload { .. } => "reload",
            Self::Snapshot { .. } => "snapshot",
            Self::Screenshot { .. } => "screenshot",
            Self::ConsoleMessages { .. } => "console_messages",
            Self::ConsoleClear { .. } => "console_clear",
            Self::NetworkRequests { .. } => "network_requests",
            Self::NetworkClear { .. } => "network_clear",
            Self::Click { .. } => "click",
            Self::Hover { .. } => "hover",
            Self::Drag { .. } => "drag",
            Self::Fill { .. } => "fill",
            Self::FillForm { .. } => "fill_form",
            Self::SelectOption { .. } => "select_option",
            Self::PressKey { .. } => "press_key",
            Self::Check { .. } => "check",
            Self::Uncheck { .. } => "uncheck",
            Self::MouseMove { .. } => "mouse_move",
            Self::MouseClick { .. } => "mouse_click",
            Self::MouseDrag { .. } => "mouse_drag",
            Self::MouseDown { .. } => "mouse_down",
            Self::MouseUp { .. } => "mouse_up",
            Self::MouseWheel { .. } => "mouse_wheel",
            Self::WaitFor { .. } => "wait_for",
            Self::VerifyElementVisible { .. } => "verify_element_visible",
            Self::VerifyTextVisible { .. } => "verify_text_visible",
            Self::VerifyListVisible { .. } => "verify_list_visible",
            Self::VerifyValue { .. } => "verify_value",
            Self::Evaluate { .. } => "evaluate",
            Self::RunCode { .. } => "run_code",
            Self::Tabs { .. } => "tabs",
            Self::Resize { .. } => "resize",
            Self::Close { .. } => "close",
            Self::Route { .. } => "route",
            Self::RouteList { .. } => "route_list",
            Self::Unroute { .. } => "unroute",
            Self::CookieList { .. } => "cookie_list",
            Self::CookieGet { .. } => "cookie_get",
            Self::CookieSet { .. } => "cookie_set",
            Self::CookieDelete { .. } => "cookie_delete",
            Self::CookieClear { .. } => "cookie_clear",
            Self::LocalStorageGet { .. } => "local_storage_get",
            Self::LocalStorageSet { .. } => "local_storage_set",
            Self::LocalStorageDelete { .. } => "local_storage_delete",
            Self::LocalStorageClear { .. } => "local_storage_clear",
            Self::SessionStorageGet { .. } => "session_storage_get",
            Self::SessionStorageSet { .. } => "session_storage_set",
            Self::SessionStorageDelete { .. } => "session_storage_delete",
            Self::SessionStorageClear { .. } => "session_storage_clear",
            Self::StorageState { .. } => "storage_state",
            Self::SetStorageState { .. } => "set_storage_state",
            Self::FileUpload { .. } => "file_upload",
            Self::HandleDialog { .. } => "handle_dialog",
            Self::Download { .. } => "download",
            Self::PdfSave { .. } => "pdf_save",
            Self::StartVideo { .. } => "start_video",
            Self::StopVideo { .. } => "stop_video",
            Self::StartTracing { .. } => "start_tracing",
            Self::StopTracing { .. } => "stop_tracing",
            Self::GenerateLocator { .. } => "generate_locator",
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            Self::Navigate { optional, .. } => *optional,
            Self::NavigateBack { optional, .. } => *optional,
            Self::NavigateForward { optional, .. } => *optional,
            Self::Reload { optional, .. } => *optional,
            Self::Snapshot { optional, .. } => *optional,
            Self::Screenshot { optional, .. } => *optional,
            Self::ConsoleMessages { optional, .. } => *optional,
            Self::ConsoleClear { optional, .. } => *optional,
            Self::NetworkRequests { optional, .. } => *optional,
            Self::NetworkClear { optional, .. } => *optional,
            Self::Click { optional, .. } => *optional,
            Self::Hover { optional, .. } => *optional,
            Self::Drag { optional, .. } => *optional,
            Self::Fill { optional, .. } => *optional,
            Self::FillForm { optional, .. } => *optional,
            Self::SelectOption { optional, .. } => *optional,
            Self::PressKey { optional, .. } => *optional,
            Self::Check { optional, .. } => *optional,
            Self::Uncheck { optional, .. } => *optional,
            Self::MouseMove { optional, .. } => *optional,
            Self::MouseClick { optional, .. } => *optional,
            Self::MouseDrag { optional, .. } => *optional,
            Self::MouseDown { optional, .. } => *optional,
            Self::MouseUp { optional, .. } => *optional,
            Self::MouseWheel { optional, .. } => *optional,
            Self::WaitFor { optional, .. } => *optional,
            Self::VerifyElementVisible { optional, .. } => *optional,
            Self::VerifyTextVisible { optional, .. } => *optional,
            Self::VerifyListVisible { optional, .. } => *optional,
            Self::VerifyValue { optional, .. } => *optional,
            Self::Evaluate { optional, .. } => *optional,
            Self::RunCode { optional, .. } => *optional,
            Self::Tabs { optional, .. } => *optional,
            Self::Resize { optional, .. } => *optional,
            Self::Close { optional, .. } => *optional,
            Self::Route { optional, .. } => *optional,
            Self::RouteList { optional, .. } => *optional,
            Self::Unroute { optional, .. } => *optional,
            Self::CookieList { optional, .. } => *optional,
            Self::CookieGet { optional, .. } => *optional,
            Self::CookieSet { optional, .. } => *optional,
            Self::CookieDelete { optional, .. } => *optional,
            Self::CookieClear { optional, .. } => *optional,
            Self::LocalStorageGet { optional, .. } => *optional,
            Self::LocalStorageSet { optional, .. } => *optional,
            Self::LocalStorageDelete { optional, .. } => *optional,
            Self::LocalStorageClear { optional, .. } => *optional,
            Self::SessionStorageGet { optional, .. } => *optional,
            Self::SessionStorageSet { optional, .. } => *optional,
            Self::SessionStorageDelete { optional, .. } => *optional,
            Self::SessionStorageClear { optional, .. } => *optional,
            Self::StorageState { optional, .. } => *optional,
            Self::SetStorageState { optional, .. } => *optional,
            Self::FileUpload { optional, .. } => *optional,
            Self::HandleDialog { optional, .. } => *optional,
            Self::Download { optional, .. } => *optional,
            Self::PdfSave { optional, .. } => *optional,
            Self::StartVideo { optional, .. } => *optional,
            Self::StopVideo { optional, .. } => *optional,
            Self::StartTracing { optional, .. } => *optional,
            Self::StopTracing { optional, .. } => *optional,
            Self::GenerateLocator { optional, .. } => *optional,
        }
    }

    pub fn timeout_ms(&self, global: u64) -> u64 {
        match self {
            Self::Navigate { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::NavigateBack { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::NavigateForward { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Reload { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Snapshot { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Screenshot { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::ConsoleMessages { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::ConsoleClear { .. } => global,
            Self::NetworkRequests { .. } => global,
            Self::NetworkClear { .. } => global,
            Self::Click { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Hover { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Drag { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Fill { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::FillForm { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::SelectOption { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::PressKey { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Check { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Uncheck { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::MouseMove { .. } => global,
            Self::MouseClick { .. } => global,
            Self::MouseDrag { .. } => global,
            Self::MouseDown { .. } => global,
            Self::MouseUp { .. } => global,
            Self::MouseWheel { .. } => global,
            Self::WaitFor { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::VerifyElementVisible { .. } => global,
            Self::VerifyTextVisible { .. } => global,
            Self::VerifyListVisible { .. } => global,
            Self::VerifyValue { .. } => global,
            Self::Evaluate { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::RunCode { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::Tabs { .. } => global,
            Self::Resize { .. } => global,
            Self::Close { .. } => global,
            Self::Route { .. } => global,
            Self::RouteList { .. } => global,
            Self::Unroute { .. } => global,
            Self::CookieList { .. } => global,
            Self::CookieGet { .. } => global,
            Self::CookieSet { .. } => global,
            Self::CookieDelete { .. } => global,
            Self::CookieClear { .. } => global,
            Self::LocalStorageGet { .. } => global,
            Self::LocalStorageSet { .. } => global,
            Self::LocalStorageDelete { .. } => global,
            Self::LocalStorageClear { .. } => global,
            Self::SessionStorageGet { .. } => global,
            Self::SessionStorageSet { .. } => global,
            Self::SessionStorageDelete { .. } => global,
            Self::SessionStorageClear { .. } => global,
            Self::StorageState { .. } => global,
            Self::SetStorageState { .. } => global,
            Self::FileUpload { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::HandleDialog { .. } => global,
            Self::Download { timeout_ms, .. } => timeout_ms.unwrap_or(global),
            Self::PdfSave { .. } => global,
            Self::StartVideo { .. } => global,
            Self::StopVideo { .. } => global,
            Self::StartTracing { .. } => global,
            Self::StopTracing { .. } => global,
            Self::GenerateLocator { .. } => global,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_navigate_step() {
        let json = r#"{"action":"navigate","url":"https://example.com"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::Navigate { url, .. } if url == "https://example.com"));
    }

    #[test]
    fn deserialize_click_step_with_selector() {
        let json = "{\"action\":\"click\",\"selector\":\"#submit\"}";
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::Click { selector: Some(s), .. } if s == "#submit"));
    }

    #[test]
    fn deserialize_fill_step() {
        let json = r#"{"action":"fill","selector":"input[name=q]","text":"hello","submit":true}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::Fill { text, submit: Some(true), .. } if text == "hello")
        );
    }

    #[test]
    fn deserialize_operation_template() {
        let json = r#"{
            "headless": true,
            "steps": [
                {"action":"navigate","url":"https://example.com"},
                {"action":"snapshot"}
            ]
        }"#;
        let tmpl: BrowserTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(tmpl.steps.len(), 2);
    }

    #[test]
    fn unknown_field_rejected() {
        let json = r#"{"action":"navigate","url":"https://x.com","bogus":true}"#;
        assert!(serde_json::from_str::<BrowserStep>(json).is_err());
    }

    #[test]
    fn optional_defaults_to_false() {
        let json = r#"{"action":"navigate","url":"https://x.com"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(!step.is_optional());
    }

    #[test]
    fn timeout_ms_falls_back_to_global() {
        let json = r#"{"action":"navigate","url":"https://x.com"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.timeout_ms(5000), 5000);
    }
}
