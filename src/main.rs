use i18n_embed::DesktopLanguageRequester;
use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

mod components;
mod config;
pub mod cosmic_session;
pub mod session_manager;
mod subscriptions;

pub static LANG_LOADER: LazyLock<FluentLanguageLoader> =
    LazyLock::new(|| fluent_language_loader!());

/// WMDE: the gettext domain that translates the layout names of xkeyboard-config.
const XKB_TEXT_DOMAIN: &str = "xkeyboard-config";

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::LANG_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::LANG_LOADER, $message_id, $($args), *)
    }};
}

fn main() {
    let requested_languages = DesktopLanguageRequester::requested_languages();
    i18n_embed::select(&*LANG_LOADER, &Localizations, &requested_languages)
        .expect("Failed to load languages");

    env_logger::init();
    bind_layout_names();
    components::app::main().unwrap();
}

/// WMDE: the layout names come out of the catalog as UTF-8 whatever the character type of the
/// process, so Cyrillic is never transliterated.
fn bind_layout_names() {
    if let Err(why) = gettextrs::bind_textdomain_codeset(XKB_TEXT_DOMAIN, "UTF-8") {
        log::warn!("could not set the codeset of {XKB_TEXT_DOMAIN}: {why}");
    }
}

/// WMDE: a layout description in the language of the session. xkeyboard-config names layouts
/// in English and ships a gettext catalog for them, the one the keyboard page of Settings
/// reads. The catalog is read under a locale set for this call only: a process-wide setlocale
/// would also turn glibc's own messages, such as the errors in the log, into Ukrainian.
pub fn layout_description(description: &str) -> String {
    // SAFETY: the locale object is created from the environment, installed for this thread
    // only, and the previous locale is restored before the object is freed.
    unsafe {
        let locale = libc::newlocale(
            libc::LC_MESSAGES_MASK | libc::LC_CTYPE_MASK,
            c"".as_ptr(),
            std::ptr::null_mut(),
        );
        if locale.is_null() {
            return description.to_owned();
        }
        let previous = libc::uselocale(locale);
        let translated = gettextrs::dgettext(XKB_TEXT_DOMAIN, description);
        libc::uselocale(previous);
        libc::freelocale(locale);
        translated
    }
}
