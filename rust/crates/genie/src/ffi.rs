//! In-process inference via litert-lm's C API (`c/engine.h`), linking the
//! prebuilt liblitert-lm.so. Optional: enabled with `--features ffi`. M6.
//!
//! Uses the conversation API one-shot `send_message` (the path that renders the
//! chat template internally). Supports text and image/audio (via file-path
//! message content). Two things were attempted and reverted because they crash
//! the multithreaded GPU runtime: (a) fd-level filtering of the "Loaded OpenCL"
//! init line, and (b) the streaming callback (`send_message_stream`). So this
//! path prints the full answer at once and leaves the cosmetic init line; the
//! subprocess path streams and filters it.
#![allow(non_camel_case_types)]

use anyhow::{bail, Result};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// Opaque handles.
pub enum LiteRtLmEngineSettings {}
pub enum LiteRtLmEngine {}
pub enum LiteRtLmConversation {}
pub enum LiteRtLmConversationConfig {}
pub enum LiteRtLmConversationOptionalArgs {}
pub enum LiteRtLmJsonResponse {}

extern "C" {
    fn litert_lm_set_min_log_level(level: c_int);
    fn litert_lm_engine_settings_create(
        model_path: *const c_char,
        backend: *const c_char,
        vision_backend: *const c_char,
        audio_backend: *const c_char,
    ) -> *mut LiteRtLmEngineSettings;
    fn litert_lm_engine_settings_delete(s: *mut LiteRtLmEngineSettings);
    fn litert_lm_engine_settings_set_max_num_images(s: *mut LiteRtLmEngineSettings, n: c_int);
    fn litert_lm_engine_create(s: *const LiteRtLmEngineSettings) -> *mut LiteRtLmEngine;
    fn litert_lm_engine_delete(e: *mut LiteRtLmEngine);
    fn litert_lm_conversation_create(
        engine: *mut LiteRtLmEngine,
        config: *mut LiteRtLmConversationConfig,
    ) -> *mut LiteRtLmConversation;
    fn litert_lm_conversation_delete(conv: *mut LiteRtLmConversation);
    fn litert_lm_conversation_send_message(
        conv: *mut LiteRtLmConversation,
        message_json: *const c_char,
        extra_context: *const c_char,
        optional_args: *const LiteRtLmConversationOptionalArgs,
    ) -> *mut LiteRtLmJsonResponse;
    fn litert_lm_json_response_get_string(resp: *const LiteRtLmJsonResponse) -> *const c_char;
    fn litert_lm_json_response_delete(resp: *mut LiteRtLmJsonResponse);
}

/// One-shot in-process generation. `message_json` is a conversation message
/// (text, or a content array with image/audio file paths). Returns the answer.
pub fn run(
    model_path: &str,
    backend: &str,
    vision_backend: Option<&str>,
    audio_backend: Option<&str>,
    message_json: &str,
) -> Result<String> {
    let mp = CString::new(model_path)?;
    let be = CString::new(backend)?;
    let vis = vision_backend.map(|s| CString::new(s).unwrap());
    let aud = audio_backend.map(|s| CString::new(s).unwrap());
    let msg = CString::new(message_json)?;
    let ctx = CString::new("{}")?;

    unsafe {
        litert_lm_set_min_log_level(4); // ERROR
        let settings = litert_lm_engine_settings_create(
            mp.as_ptr(),
            be.as_ptr(),
            vis.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            aud.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
        );
        if settings.is_null() {
            bail!("litert_lm_engine_settings_create returned NULL");
        }
        // Images are only processed when the engine reserves image slots.
        if vision_backend.is_some() {
            litert_lm_engine_settings_set_max_num_images(settings, 1);
        }
        let engine = litert_lm_engine_create(settings);
        if engine.is_null() {
            litert_lm_engine_settings_delete(settings);
            bail!("litert_lm_engine_create returned NULL");
        }
        let conv = litert_lm_conversation_create(engine, std::ptr::null_mut());
        if conv.is_null() {
            litert_lm_engine_delete(engine);
            litert_lm_engine_settings_delete(settings);
            bail!("litert_lm_conversation_create returned NULL");
        }
        let resp = litert_lm_conversation_send_message(
            conv,
            msg.as_ptr(),
            ctx.as_ptr(),
            std::ptr::null(),
        );
        let mut out = String::new();
        if !resp.is_null() {
            let s = litert_lm_json_response_get_string(resp);
            if !s.is_null() {
                out = extract_text(&CStr::from_ptr(s).to_string_lossy());
            }
            litert_lm_json_response_delete(resp);
        }
        litert_lm_conversation_delete(conv);
        litert_lm_engine_delete(engine);
        litert_lm_engine_settings_delete(settings);
        if out.trim().is_empty() {
            bail!("generation produced no output");
        }
        Ok(out)
    }
}

/// Extract assistant text from the JSON response
/// (`{"content":[{"type":"text","text":"..."}]}`, or simpler shapes).
fn extract_text(json: &str) -> String {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return json.to_string(),
    };
    if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
        let s: String = arr
            .iter()
            .filter_map(|it| it.get("text").and_then(|t| t.as_str()))
            .collect();
        if !s.is_empty() {
            return s;
        }
    }
    if let Some(c) = v.get("content").and_then(|c| c.as_str()) {
        return c.to_string();
    }
    if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
        return t.to_string();
    }
    json.to_string()
}
