//! Help › Report an Issue…: the command, the Account entry, the Help menu entry and the fallback
//! when no browser opens.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use super::{Services, SlideApp};
use crate::Session;

type Opened = Arc<Mutex<Vec<String>>>;

/// An app whose browser opener records addresses and answers `result`.
fn app(result: Result<(), String>) -> (SlideApp, Opened) {
    let opened = Opened::default();
    let log = opened.clone();
    let services = Services {
        open_url: Some(Box::new(move |url: &str| {
            log.lock().unwrap().push(url.to_string());
            result.clone()
        })),
        ..Default::default()
    };
    (SlideApp::new(Session::with_new(), services), opened)
}

fn harness(app: SlideApp) -> Harness<'static, SlideApp> {
    Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    )
}

#[test]
fn the_command_opens_the_prefilled_form_once() {
    let (mut app, opened) = app(Ok(()));
    app.graphics = Some("Test GPU (Vulkan, DiscreteGpu)".into());
    let v = app.run("app.reportIssue", json!({})).unwrap();
    let opened = opened.lock().unwrap();
    assert_eq!(opened.len(), 1);
    assert_eq!(v["url"], opened[0].as_str());
    assert_eq!(v["opened"], true);
    assert!(opened[0].starts_with(&format!("{}?template=bug_report.yml&version=DeckCraft%20", deckcraft_engine::links::NEW_ISSUE)), "{}", opened[0]);
    assert!(v["details"].as_str().unwrap().contains("Graphics: Test GPU (Vulkan, DiscreteGpu)"));
    assert!(app.dialog.is_none(), "a browser that opened needs no dialog");
    assert!(app.pending_urls.is_empty());
}

#[test]
fn open_false_only_returns_the_address() {
    let (mut app, opened) = app(Ok(()));
    let v = app.run("app.reportIssue", json!({"open": false})).unwrap();
    assert!(v["url"].as_str().unwrap().starts_with(deckcraft_engine::links::NEW_ISSUE));
    assert_eq!(v["opened"], false);
    assert!(opened.lock().unwrap().is_empty() && app.pending_urls.is_empty() && app.dialog.is_none());
}

#[test]
fn without_a_host_opener_the_address_goes_to_egui() {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    let v = app.run("app.reportIssue", json!({})).unwrap();
    assert_eq!(app.pending_urls, [v["url"].as_str().unwrap()]);
}

#[test]
fn a_failed_open_keeps_the_details_on_screen_to_copy() {
    let (app, opened) = app(Err("no browser".into()));
    let mut h = harness(app);
    let v = h.state_mut().run("app.reportIssue", json!({})).unwrap();
    assert_eq!(v["opened"], false);
    assert_eq!(opened.lock().unwrap().len(), 1);
    h.run_steps(3);
    assert!(h.query_by_label("Couldn't open your web browser").is_some());
    h.get_by_label("Copy Details").click();
    // Each frame's output replaces the last, so collect it frame by frame.
    let mut copied = vec![];
    for _ in 0..3 {
        h.step();
        copied.extend(h.output().platform_output.commands.iter().filter_map(|c| match c {
            egui::output::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        }));
    }
    assert_eq!(copied.len(), 1, "{copied:?}");
    assert!(copied[0].starts_with("Version: DeckCraft ") && copied[0].contains("Document type: new (unsaved)"), "{}", copied[0]);
    assert!(copied[0].ends_with(deckcraft_engine::links::NEW_ISSUE));
    assert!(h.state().dialog.is_some(), "Copy Details leaves the dialog open");
    h.get_by_label("Close").click();
    h.run_steps(2);
    assert!(h.state().dialog.is_none());
}

#[test]
fn escape_closes_the_fallback() {
    let (app, _) = app(Err("no browser".into()));
    let mut h = harness(app);
    h.state_mut().run("app.reportIssue", json!({})).unwrap();
    h.run_steps(3);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(h.state().dialog.is_none());
}

#[test]
fn the_account_page_has_the_entry() {
    let (app, opened) = app(Ok(()));
    let mut h = harness(app);
    h.state_mut().run("app.backstage", json!({"page": "account"})).unwrap();
    h.run_steps(3);
    h.get_all_by_label("Report an Issue…").next().expect("the task's button").click();
    h.run_steps(2);
    assert_eq!(opened.lock().unwrap().len(), 1);
}

#[test]
fn help_lists_it() {
    let help = crate::menus::menu_tree().into_iter().find(|(name, _)| *name == "Help").unwrap().1;
    assert!(help.contains(&("Report an Issue…", "app.reportIssue")), "{help:?}");
}
