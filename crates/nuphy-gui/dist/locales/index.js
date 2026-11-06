// Available languages. To add one: copy locales/en.js to locales/<code>.js, translate it,
// and add a line here. `code` is a BCP 47 tag; `name` is the language's own name.
// Must be strict JSON inside the call (checked by `cargo test -p nuphy-gui`).
I18N_LANGUAGES([
  { "code": "en", "name": "English" },
  { "code": "zh-CN", "name": "简体中文" }
]);
